use async_std::fs::File;
use async_std::io::BufReader;
use bytes::Bytes;
use futures::future::{BoxFuture, FutureExt};
use futures::io::Cursor;
use http::{header, StatusCode};
use httpdate::HttpDate;
use tide::Response;

use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;
use std::{fs, io};

const DEFAULT_4XX_BODY: &[u8] = b"Oops! I can't find what you're looking for..." as &[_];
const DEFAULT_5XX_BODY: &[u8] = b"I'm broken, apparently." as &[_];

/// Simple static file handler for Tide.
#[derive(Clone)]
pub struct StaticFile {
    // FIXME: These should be setters which also determine and set the MIME type.
    pub body_4xx: Vec<u8>,
    pub body_5xx: Vec<u8>,
    root: PathBuf,
}

impl StaticFile {
    /// Creates a new instance of this handler.
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = PathBuf::from(root.as_ref());
        if !root.exists() {
            warn!("Path {} doesn't exist.", root.display());
        }

        StaticFile {
            root,
            body_4xx: Vec::from(DEFAULT_4XX_BODY),
            body_5xx: Vec::from(DEFAULT_5XX_BODY),
        }
    }

    // FIXME: Refactor and add tests

    /// Stream file!
    pub fn stream_bytes<'a>(&'a self, actual_path: &'a str, if_modified_since: Option<&'a str>, if_none_match: Option<&'a str>) -> BoxFuture<'a, Result<Response, io::Error>> {
        let path = self.get_path(actual_path);
        let mut response = Response::new(StatusCode::OK.as_u16());
        let meta = fs::metadata(&path).ok();
        // Check if the path exists and handle if it's a directory containing `index.html`
        if meta.is_some() && meta.as_ref().map(|m| !m.is_file()).unwrap_or(false) {
            // Redirect if path is a dir and URL doesn't end with "/"
            if !actual_path.ends_with("/") {
                return async move {
                    Ok(response
                    .set_status(StatusCode::MOVED_PERMANENTLY)
                    .set_header(header::LOCATION.as_str(), String::from(actual_path) + "/")
                    .body(futures::io::empty()))
                }.boxed()
            }

            let index = Path::new(actual_path).join("index.html");
            return async move {
                self.stream_bytes(&*index.to_string_lossy(), if_modified_since, if_none_match).await
            }.boxed();
        }

        // If the file doesn't exist, then bail out.
        let meta = match meta {
            Some(m) => m,
            None => return async move {
                Ok(response
                .set_status(StatusCode::NOT_FOUND)
                .set_header(header::CONTENT_TYPE.as_str(), mime::TEXT_HTML.as_ref())
                .body(Cursor::new(Bytes::from(self.body_4xx.clone()))))
            }.boxed(),
        };

        // Caching-related thingies.
        let mime = mime_guess::from_path(&path).first_or_octet_stream();
        let last_modified = match meta.modified() {
            Ok(m) => m,
            Err(e) => return async move { Err(e) }.boxed(),
        };

        let size = meta.len();
        let etag = format!(
            "{:x}-{:x}",
            last_modified
                .duration_since(UNIX_EPOCH)
                .expect("unix epoch is wrong?")
                .as_secs(),
            size
        );

        response = response
            .set_header(
                header::LAST_MODIFIED.as_str(),
                httpdate::fmt_http_date(last_modified),
            )
            .set_header(header::ETAG.as_str(), etag.as_str())
            .set_header(header::CONTENT_DISPOSITION.as_str(), {
                let ty = match mime.type_() {
                    mime::IMAGE | mime::TEXT | mime::VIDEO => "inline",
                    _ => "attachment",
                };

                match path.file_name().expect("already normalized path?").to_str() {
                    Some(name) => format!(
                        "{}; filename*=\"{}\"",
                        ty,
                        percent_encoding::utf8_percent_encode(
                            name,
                            percent_encoding::NON_ALPHANUMERIC
                        )
                    )
                    .into(),
                    None => String::from(ty),
                }
            });

        let respond_cache = if let Some(etags) = if_none_match {
            etags.split(',').map(str::trim).any(|x| x == etag)
        } else {
            if_modified_since
                .and_then(|x| x.parse::<HttpDate>().ok())
                .map(|x| x == HttpDate::from(last_modified))
                .unwrap_or(false)
        };

        if respond_cache {
            return async move {
                Ok(response
                    .set_status(StatusCode::NOT_MODIFIED)
                    .body(futures::io::empty()))
            }.boxed()
        }

        // We're done with the checks. Stream file!
        response = response
            .set_status(StatusCode::OK)
            .set_header(header::CONTENT_LENGTH.as_str(), size.to_string());

        async move {
            if size == 0 {
                return Ok(response.body(futures::io::empty()).set_mime(mime));
            }

            let fd = BufReader::new(File::open(path).await?);
            Ok(response.body(fd).set_mime(mime))
        }.boxed()
    }

    /// Percent-decode, normalize path components and return the final path joined with root.
    fn get_path(&self, path: &str) -> PathBuf {
        let rel_path = Path::new(path)
            .components()
            .fold(PathBuf::new(), |mut result, p| {
                match p {
                    Component::Normal(x) => result.push({
                        let s = x.to_str().unwrap_or("");
                        &*percent_encoding::percent_decode(s.as_bytes()).decode_utf8_lossy()
                    }),
                    Component::ParentDir => {
                        result.pop();
                    }
                    _ => (),
                }

                result
            });

        self.root.join(rel_path)
    }
}
