use http_body_util::{BodyExt, Full};
use hyper::{body::Bytes, Request};
use hyper_util::client::legacy::Client;
use hyperlocal::{UnixClientExt, UnixConnector, Uri};
use std::error::Error;
use tokio::io::{self, AsyncWriteExt as _};

use containd::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    let command = &args[1];
    let arg = &args[2];

    let socket = "/run/contain.sock";

    let route = "/api/net/tap";

    let url = Uri::new(socket, route);

    let client: Client<UnixConnector, Full<Bytes>> = Client::unix();

    let req = match command.as_str() {
        "create" => {
            let data = NetTapCreateRequest {
                user: arg.to_owned(),
            };

            let json_string = serde_json::to_string(&data)?;

            Request::post(url)
                .header("Content-Type", "application/json")
                .body(Bytes::from(json_string).into())?
        }
        "delete" => {
            let data = NetTapDeleteRequest {
                name: arg.to_owned(),
            };

            let json_string = serde_json::to_string(&data)?;

            Request::delete(url)
                .header("Content-Type", "application/json")
                .body(Bytes::from(json_string).into())?
        }
        _ => panic!("unsuported subcommand"),
    };

    let mut res = client.request(req).await?;

    while let Some(frame_result) = res.frame().await {
        let frame = frame_result?;

        if let Some(segment) = frame.data_ref() {
            io::stdout().write_all(segment.iter().as_slice()).await?;
        }
    }

    Ok(())
}
