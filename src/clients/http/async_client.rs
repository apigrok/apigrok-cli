use http_body_util::Empty;
use hyper::{
    Request, Response,
    body::{Bytes, Incoming},
};

pub trait AsyncHttpClient {
    async fn send(&self, req: Request<Empty<Bytes>>) -> Result<Response<Incoming>, hyper::Error>;
}
