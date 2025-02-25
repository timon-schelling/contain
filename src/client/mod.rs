use http::{Method, Request, Response};
use http_body_util::{BodyExt, Full};
use hyper::body::{Buf, Bytes, Incoming};
use hyper_util::client::legacy::Client;
use hyperlocal::{UnixClientExt, UnixConnector, Uri};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

use crate::daemon::{requests::*, DEFAULT_SOCKET_PATH};

pub async fn request_tap_device(user: String) -> Result<String, RequestError> {
    let body = NetTapCreateRequest { user };
    let NetTapCreateResponse { name } = json_request("/api/net/tap", Method::POST, body).await?;
    Ok(name)
}

pub async fn delete_tap_device(name: String) -> Result<(), RequestError>{
    let body = NetTapDeleteRequest { name };
    json_request("/api/net/tap", Method::DELETE, body).await
}

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("failed in serde")]
    Serde(#[from] serde_json::Error),
    #[error("request invalid")]
    RequestInvalid(#[from] http::Error),
    #[error("request failed")]
    RequestFailed(#[from] hyper::Error),
    #[error("client error")]
    ClientError(#[from] hyper_util::client::legacy::Error),
}

async fn json_request<B: Serialize, R: DeserializeOwned>(route: &str, method: Method, body: B) -> Result<R, RequestError> {
    let url = Uri::new(DEFAULT_SOCKET_PATH, &route);
    let client: Client<UnixConnector, Full<Bytes>> = Client::unix();

    let json: String = serde_json::to_string(&body)?;
    let bytes = Full::new(Bytes::from_owner(json));

    let req = Request::builder()
        .uri(url)
        .method(method)
        .header("Content-Type", "application/json")
        .body(bytes)?;

    let res: Response<Incoming> = client.request(req).await?;

    let reader = res.into_body().collect().await?.aggregate().reader();
    let json = serde_json::from_reader::<_, R>(reader)?;
    Ok(json)
}
