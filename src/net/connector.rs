use rustls::{ClientConfig, RootCertStore, ServerName};
use std::pin::Pin;
use std::{net::ToSocketAddrs, sync::Arc};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, client::TlsStream};

pub enum ConnectionStream {
    Plain(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
}

pub type BoxedStream = Pin<Box<dyn AsyncRead + AsyncWrite + Send + Sync + Unpin>>;

impl ConnectionStream {
    pub fn into_boxed(self) -> BoxedStream {
        match self {
            ConnectionStream::Plain(s) => Box::new(s),
            ConnectionStream::Tls(t) => t,
        }
    }
}

pub struct GrokkerConnector {
    tls_config: Arc<ClientConfig>,
}

impl GrokkerConnector {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut root_store = RootCertStore::empty();
        for cert in rustls_native_certs::load_native_certs()? {
            root_store.add(&rustls::Certificate(cert.0))?;
        }

        let mut config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        Ok(Self {
            tls_config: Arc::new(config),
        })
    }

    pub async fn connect(
        &self,
        host: &str,
        port: u16,
        use_tls: bool,
    ) -> Result<BoxedStream, Box<dyn std::error::Error>> {
        let addr = (host, port)
            .to_socket_addrs()?
            .next()
            .ok_or("Invalid address")?;
        let stream = TcpStream::connect(addr).await?;

        if use_tls {
            let connector = TlsConnector::from(self.tls_config.clone());
            let server_name = ServerName::try_from(host)?;
            let tls_stream = connector.connect(server_name, stream).await?;
            Ok(Box::new(tls_stream))
        } else {
            Ok(Box::new(stream))
        }
    }
}
