use async_std::fs::{self, File, Metadata};
use async_std::io::BufReader;
use futures::future::{BoxFuture, FutureExt};
use futures::io::Cursor;
use http::{header, StatusCode};
use httpdate::HttpDate;
use tide::{Request, Response};

use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

const DEFAULT_4XX_BODY: &[u8] = b"Oops! I can't find what you're looking for..." as &[_];
const DEFAULT_5XX_BODY: &[u8] = b"I'm broken, apparently." as &[_];

/// Simple static file handler for Tide.
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

/// Responder to serve a file request.
pub struct Responder<'a> {
    actual_path: &'a str,
    state: &'a StaticFile,
    path: PathBuf,
    resp: Response,
    if_modified_since: Option<&'a str>,
    if_none_match: Option<&'a str>,
}

impl<'a> Responder<'a> {
    /// Create an instance from an incoming request.
    pub fn from(req: &'a Request<StaticFile>) -> Self {
        let actual_path = req.uri().path();
        let state = req.state();
        Responder {
            actual_path,
            state: &state,
            path: state.get_path(actual_path),
            resp: Response::new(StatusCode::OK.as_u16()),
            if_none_match: req.header(header::IF_NONE_MATCH.as_str()),
            if_modified_since: req.header(header::IF_MODIFIED_SINCE.as_str()),
        }
    }

    /// Stream path (if any)...
    pub fn stream(self) -> BoxFuture<'a, Response> {
        async move {
            let state = self.state;
            match self.stream_().await {
                Ok(r) => r,
                Err(e) => {
                    error!("{:?}", e);
                    Response::new(StatusCode::INTERNAL_SERVER_ERROR.as_u16())
                        .body(Cursor::new(state.body_5xx.clone()))
                        .set_header(header::CONTENT_DISPOSITION.as_str(), "inline")
                        .set_header(header::CONTENT_LENGTH.as_str(), state.body_5xx.len().to_string())
                        .set_header(header::CONTENT_TYPE.as_str(), mime::TEXT_HTML.as_ref())
                }
            }
        }
        .boxed()
    }

    async fn stream_(self) -> Result<Response, io::Error> {
        let meta = fs::metadata(&self.path).await.ok();
        // Check if the path exists and handle if it's a directory containing `index.html`
        if meta.is_some() && meta.as_ref().map(|m| !m.is_file()).unwrap_or(false) {
            // Redirect if path is a dir and URL doesn't end with "/"
            if !self.actual_path.ends_with("/") {
                return Ok(self
                    .resp
                    .set_status(StatusCode::MOVED_PERMANENTLY)
                    .set_header(
                        header::LOCATION.as_str(),
                        String::from(self.actual_path) + "/",
                    )
                    .body(futures::io::empty()));
            }

            let index = Path::new(self.actual_path).join("index.html");
            let actual_path = &*index.to_string_lossy();
            return Ok(Responder {
                actual_path,
                path: self.state.get_path(actual_path),
                ..self
            }
            .stream()
            .await);
        }

        match meta {
            Some(m) => Ok(self.stream_using_meta(m).await?),
            None => Ok(self
                .resp
                .set_status(StatusCode::NOT_FOUND)
                .body(Cursor::new(self.state.body_4xx.clone()))
                .set_header(header::CONTENT_DISPOSITION.as_str(), "inline")
                .set_header(header::CONTENT_LENGTH.as_str(), self.state.body_4xx.len().to_string())
                .set_header(header::CONTENT_TYPE.as_str(), mime::TEXT_HTML.as_ref())),
        }
    }

    async fn stream_using_meta(mut self, meta: Metadata) -> Result<Response, io::Error> {
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

        let mime = mime_guess::from_path(&self.path).first_or_octet_stream();
        self.resp = self
            .resp
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

                match self
                    .path
                    .file_name()
                    .expect("already normalized path?")
                    .to_str()
                {
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

        let respond_cache = if let Some(etags) = self.if_none_match {
            etags.split(',').map(str::trim).any(|x| x == etag)
        } else {
            self.if_modified_since
                .and_then(|x| x.parse::<HttpDate>().ok())
                .map(|x| x == HttpDate::from(last_modified))
                .unwrap_or(false)
        };

        if respond_cache {
            return Ok(self
                .resp
                .set_status(StatusCode::NOT_MODIFIED)
                .body(futures::io::empty()));
        }

        // We're done with the checks. Stream file!
        self.resp = self
            .resp
            .set_status(StatusCode::OK)
            .set_header(header::CONTENT_LENGTH.as_str(), size.to_string());

        if size == 0 {
            return Ok(self.resp.body(futures::io::empty()).set_mime(mime));
        }

        let fd = BufReader::new(File::open(self.path).await?);
        Ok(self.resp.body(fd).set_mime(mime))
    }
}
