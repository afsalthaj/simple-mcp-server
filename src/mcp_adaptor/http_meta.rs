#[derive(Clone)]
pub struct HttpMeta {
    pub uri: http::Uri,
    pub headers: http::HeaderMap,
    pub version: http::Version,
}