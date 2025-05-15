use url::Url;

pub struct Client {}

pub struct ClientBuilder {
    base_url: Option<Url>,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder { base_url: None }
    }
}
