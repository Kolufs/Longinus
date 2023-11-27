#![feature(async_fn_in_trait)]
#![feature(inherent_associated_types)]
#![allow(incomplete_features)]

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use axum::body::StreamBody;
use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{AppendHeaders, IntoResponse};
use axum::routing::{get, post};
use axum::{async_trait, Json};
use axum::{Extension, RequestPartsExt, Router};
use axum_macros;

use axum_server::tls_rustls::RustlsConfig;
use config::Config;
use tokio::fs::File;
use tokio_util;
use tokio_util::io::ReaderStream;

use serde::{Deserialize, Serialize};

use crate::broker::Node;
use crate::broker::SurrealBroker;
use crate::broker::SurrealId;

mod broker;
mod config;

const ID_HEADER_KEY: &str = "X-id";

type SharedBroker = Arc<SurrealBroker>;

struct AppState {
    broker: SharedBroker,
}

impl IntoResponse for SurrealId {
    fn into_response(self) -> axum::response::Response {
        String::into_response(self.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum Arch {
    X86_64,
    X86,
}

impl FromStr for Arch {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "X86_64" => Ok(Arch::X86_64),
            "X86" => Ok(Arch::X86),
            _ => Err(()),
        }
    }
}

impl ToString for Arch {
    fn to_string(&self) -> String {
        match self {
            Arch::X86_64 => "X86_64".to_string(),
            Arch::X86 => "X86".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ArchWrap {
    arch: Arch,
}

#[async_trait]
impl<S> FromRequestParts<S> for SurrealId
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let id: String = match parts.headers.get(ID_HEADER_KEY) {
            Some(id) => match id.to_str() {
                Ok(id) => id.to_string(),
                Err(_) => return Err(StatusCode::BAD_REQUEST),
            },
            None => return Err(StatusCode::BAD_REQUEST),
        };

        let broker = parts.extract::<Extension<SharedBroker>>().await.unwrap();

        let id = SurrealId(id);

        match broker.client_exist(id.clone()).await {
            true => Ok(id),
            false => Err(StatusCode::UNAUTHORIZED),
        }
    }
}

#[axum_macros::debug_handler]
async fn register(broker: Extension<SharedBroker>, headers: HeaderMap) -> SurrealId {
    let ip = headers
        .get("X-Forwarded-For")
        .and_then(|header| header.to_str().ok())
        .and_then(|ip| IpAddr::from_str(ip).ok());

    let node = Node::new(ip);

    broker.add_node(node).await
}

#[axum_macros::debug_handler]
async fn refresh(broker: Extension<SharedBroker>, id: SurrealId) {
    broker.refresh_client(id).await.unwrap();
}

#[axum_macros::debug_handler]
async fn message(broker: Extension<SharedBroker>, id: SurrealId) -> impl IntoResponse {
    broker.refresh_client(id.clone()).await.unwrap();
    match broker.pop_message(id).await {
        Some(message) => (StatusCode::OK, Json(message)).into_response(),
        None => (StatusCode::NO_CONTENT).into_response(),
    }
}

#[axum_macros::debug_handler]
async fn binary(
    Query(arch): Query<ArchWrap>,
    config: Extension<SharedHandlerConfig>,
) -> impl IntoResponse {
    let mut path = config.binary_folder.clone();
    path.push(arch.arch.to_string());
    let file = match File::open(path).await {
        Ok(file) => file,
        Err(err) => panic!("Error opening binary folder: {}", err),
    };

    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);

    let headers = AppendHeaders([
        (header::CONTENT_TYPE, "text/toml; charset=utf-8"),
        (header::CONTENT_DISPOSITION, "attachment; filename=\"bin\""),
    ]);

    (headers, body).into_response()
}

#[axum_macros::debug_handler()]
async fn version(config: Extension<SharedHandlerConfig>) -> impl IntoResponse {
    (StatusCode::OK, config.version.to_string()).into_response()
}

#[derive(Serialize, Deserialize, Clone)]
struct HandlerConfig {
    binary_folder: PathBuf,
    version: u64,
}

type SharedHandlerConfig = Arc<HandlerConfig>;

impl From<&Config> for HandlerConfig {
    fn from(value: &Config) -> Self {
        Self {
            binary_folder: value.server.binaries_folder.clone(),
            version: value.server.version.clone(),
        }
    }
}

#[tokio::main]
async fn main() {
    let config = Config::load();
    let handler_config = Arc::new(HandlerConfig::from(&config));

    let app_state = AppState {
        broker: Arc::new(
            SurrealBroker::new(
                &config.database.socket.to_string(),
                &config.database.ns,
                &config.database.db,
                &config.database.username,
                &config.database.password,
            )
            .await,
        ),
    };

    let axum_config = RustlsConfig::from_pem_file(config.server.cert, config.server.key)
        .await
        .unwrap();

    let app = Router::new()
        .route("/node/register", post(register))
        .route("/node/refresh", post(refresh))
        .route("/node/message", post(message))
        .route("/binary", get(binary))
        .route("/version", get(version))
        .layer(Extension(app_state.broker))
        .layer(Extension(handler_config));

    axum_server::bind_rustls(config.server.socket, axum_config)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap()
}
