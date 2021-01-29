use crate::{api::InvokeRequest, handler::Handler};
use anyhow::Context;
use std::{convert::Infallible, net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc};
use warp::Filter;

#[derive(Debug, Clone)]
pub enum ListenAddress {
    Tcp(SocketAddr),
    Uds(PathBuf),
}

impl FromStr for ListenAddress {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let u = url::Url::parse(s).context("invalid url")?;
        match u.scheme() {
            "tcp" => {
                let addr = u.host_str().context("listen address missing")?;
                let ip = addr.parse().context("listen address is not IP")?;
                let port = u.port().context("listen port missing")?;
                let addr = SocketAddr::new(ip, port);
                Ok(ListenAddress::Tcp(addr))
            }
            "unix" => {
                let path = u.path().into();
                Ok(ListenAddress::Uds(path))
            }
            other => anyhow::bail!("unknown scheme {}, expected one of 'tcp', 'unix'", other),
        }
    }
}

type Resp = hyper::Response<hyper::Body>;

/// Handler for /exec requests
#[tracing::instrument(skip(handler, req), fields(request_id = %req.id.to_hyphenated()))]
async fn route_exec(handler: Arc<Handler>, req: InvokeRequest) -> Result<Resp, Infallible> {
    let res = handler.handle_invoke_request(&req).await;
    match res {
        Ok(response) => {
            let response = serde_json::to_vec(&response).expect("failed to serialize response");
            Ok(hyper::Response::builder()
                .status(200)
                .body(response.into())
                .expect("incorrect response"))
        }
        Err(err) => {
            let error_id = uuid::Uuid::new_v4();
            let err = format!("{:#}", err);

            tracing::error!(error = %err, error_id = %error_id.to_hyphenated(), "invocation request failed");

            Ok(hyper::Response::builder()
                .status(500)
                .header("Error-UUID", error_id.to_hyphenated().to_string())
                .body((&[] as &'static [u8]).into())
                .expect("incorrect response"))
        }
    }
}

/// Handler for /ready requests
async fn route_ready() -> Result<&'static str, Infallible> {
    Ok("OK")
}

/// Server HTTP API.
pub struct Server {
    handler: Arc<Handler>,
}

impl Server {
    pub fn new(handler: Handler) -> Self {
        Server {
            handler: Arc::new(handler),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn serve(self, addr: ListenAddress) -> anyhow::Result<()> {
        let handler = self.handler.clone();
        let r_exec = warp::path("exec")
            .and(warp::filters::body::json())
            .and_then(move |req| route_exec(handler.clone(), req));
        let r_ready = warp::path("ready").and_then(route_ready);
        #[cfg(debug_assertions)]
        let r_exec = r_exec.boxed();
        #[cfg(debug_assertions)]
        let r_ready = r_ready.boxed();

        let srv = r_exec.or(r_ready);
        let srv = warp::serve(srv);
        match addr {
            ListenAddress::Tcp(addr) => {
                srv.run(addr).await;
            }
            ListenAddress::Uds(path) => {
                let listener = tokio::net::UnixListener::bind(&path)
                    .with_context(|| format!("failed to attach to UDS {}", path.display()))?;
                let listener = tokio_stream::wrappers::UnixListenerStream::new(listener);
                srv.run_incoming(listener).await;
            }
        }

        Ok(())
    }
}
