use url::Url;
#[allow(unused)]
pub struct Client {}
#[allow(unused)]
pub struct ClientBuilder {
    base_url: Option<Url>,
}
#[allow(unused)]
impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder { base_url: None }
    }
}
