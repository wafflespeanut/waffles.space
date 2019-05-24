use bytes::Bytes;
use futures_fs::FsPool;
use futures_preview::compat::*;
use futures_preview::future::FutureObj;
use http::{
    header::{self, HeaderMap},
    StatusCode,
};
use http_service::Body;
use httpdate::HttpDate;
use tide::{Context, Endpoint, Response};

use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;
use std::{fs, io};

const DEFAULT_4XX_BODY: &[u8] = b"Oops! I can't find what you're looking for..." as &[_];
const DEFAULT_5XX_BODY: &[u8] = b"I'm broken, apparently." as &[_];

lazy_static! {
    static ref FS_POOL: FsPool = FsPool::default();
}

/// Simple static file handler for Tide.
#[derive(Clone)]
pub struct StaticFile {
    // FIXME: These should be setters which also determine and set the MIME type.
    pub body_4xx: Vec<u8>,
    pub body_5xx: Vec<u8>,
    root: PathBuf,
}

impl<S> Endpoint<S> for StaticFile {
    type Fut = FutureObj<'static, Response>;

    fn call(&self, ctx: Context<S>) -> Self::Fut {
        let path = ctx.uri().path();
        let resp = match self.stream_bytes(path, ctx.headers()) {
            Ok(r) => r,
            Err(e) => {
                error!("{:?}", e);
                http::Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, mime::TEXT_HTML.to_string())
                    .body(Bytes::from(&self.body_5xx[..]).into())
                    .expect("failed to build static response?")
            }
        };

        FutureObj::new(Box::new(async move { resp }))
    }
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
    fn stream_bytes(&self, actual_path: &str, headers: &HeaderMap) -> Result<Response, io::Error> {
        let path = &self.get_path(actual_path);
        let mut response = http::Response::builder();
        let meta = fs::metadata(path).ok();
        // Check if the path exists and handle if it's a directory containing `index.html`
        if meta.is_some() && meta.as_ref().map(|m| !m.is_file()).unwrap_or(false) {
            // Redirect if path is a dir and URL doesn't end with "/"
            if !actual_path.ends_with("/") {
                return Ok(response
                    .status(StatusCode::MOVED_PERMANENTLY)
                    .header(header::LOCATION, String::from(actual_path) + "/")
                    .body(Body::empty())
                    .expect("failed to build redirect response?"));
            }

            let index = Path::new(actual_path).join("index.html");
            return self.stream_bytes(&*index.to_string_lossy(), headers);
        }

        // If the file doesn't exist, then bail out.
        let meta = match meta {
            Some(m) => m,
            None => {
                return Ok(response
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, mime::TEXT_HTML.to_string())
                    .body(Bytes::from(&self.body_4xx[..]).into())
                    .expect("failed to build static response?"))
            }
        };

        // Caching-related thingies.
        let mime = mime_guess::guess_mime_type(path);
        let mime_str = mime.to_string();
        let last_modified = meta.modified()?;
        let size = meta.len();
        let etag = format!(
            "{:x}-{:x}",
            last_modified
                .duration_since(UNIX_EPOCH)
                .expect("unix epoch is wrong?")
                .as_secs(),
            size
        );

        response
            .header(
                header::LAST_MODIFIED,
                httpdate::fmt_http_date(last_modified),
            )
            .header(header::ETAG, etag.clone())
            .header(header::CONTENT_DISPOSITION, {
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
                            percent_encoding::DEFAULT_ENCODE_SET
                        )
                    )
                    .into(),
                    None => String::from(ty),
                }
            });

        let if_modified_since = headers
            .get(header::IF_MODIFIED_SINCE)
            .and_then(|x| x.to_str().ok());
        let if_none_match = headers
            .get(header::IF_NONE_MATCH)
            .and_then(|x| x.to_str().ok());

        let respond_cache = if let Some(etags) = if_none_match {
            etags.split(',').map(str::trim).any(|x| x == etag)
        } else {
            if_modified_since
                .and_then(|x| x.parse::<HttpDate>().ok())
                .map(|x| x == HttpDate::from(last_modified))
                .unwrap_or(false)
        };

        if respond_cache {
            return Ok(response
                .status(StatusCode::NOT_MODIFIED)
                .body(Body::empty())
                .expect("failed to build cache response?"));
        }

        // We're done with the checks. Stream file!
        response
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime_str.as_str())
            .header(header::CONTENT_LENGTH, size);

        if size == 0 {
            return Ok(response
                .body(Body::empty())
                .expect("failed to build empty response?"));
        }

        let stream = FS_POOL.read(PathBuf::from(path), Default::default());
        Ok(response
            .body(Body::from_stream(stream.compat()))
            .expect("invalid request?"))
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
