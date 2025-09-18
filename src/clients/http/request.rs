use hyper::body::Bytes;

pub struct Request {
    pub url: String,
    pub method: hyper::Method,
    pub headers: Option<hyper::HeaderMap>,
    pub body: Option<Bytes>,
}
