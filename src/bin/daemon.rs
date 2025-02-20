use axum::response::IntoResponse;
use axum::{http::StatusCode, routing::post, Json, Router};
use rand::Rng;
use regex::Regex;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::{error::Error, path::Path};
use tokio::net::UnixListener;

use containd::*;

static MANAGED_RESOURCES_PREFIX: &str = "vm-";

fn app() -> Router {
    Router::new().nest("/api", api())
}

fn api() -> Router {
    Router::new().nest("/net", net())
}

fn net() -> Router {
    Router::new().route("/tap", post(tap_create).delete(tap_delete))
}

async fn tap_create(Json(req): Json<NetTapCreateRequest>) -> impl IntoResponse {
    let id = hex::encode(&rand::rng().random::<[u8; 6]>());
    let name = format!("{}{}", MANAGED_RESOURCES_PREFIX, id);
    let regex = Regex::new(r"^[a-zA-Z0-9\._-]+$").unwrap();
    let user = req.user;

    if !regex.is_match(user.as_str()) {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let out = Command::new("ip")
        .args([
            "tuntap",
            "add",
            "name",
            name.as_str(),
            "mode",
            "tap",
            "user",
            user.as_str(),
            "vnet_hdr",
            "multi_queue",
        ])
        .output();
    dbg!(out);

    let out = Command::new("ip")
        .args(["link", "set", name.as_str(), "up"])
        .output();
    dbg!(out);
    dbg!(name.clone());

    (StatusCode::CREATED, Json(NetTapCreateResponse { name })).into_response()
}

async fn tap_delete(Json(req): Json<NetTapDeleteRequest>) -> impl IntoResponse {
    let name = req.name;
    if !name.starts_with(MANAGED_RESOURCES_PREFIX) {
        return StatusCode::FORBIDDEN;
    }

    let out = Command::new("ip")
        .args(["link", "delete", name.as_str()])
        .output();
    dbg!(out);

    StatusCode::ACCEPTED
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let path = Path::new("/run/contain.sock");
    if path.exists() {
        match fs::remove_file(path) {
            Err(_) => {
                fs::remove_dir_all(path)?;
            }
            _ => {}
        }
    }
    let listener = UnixListener::bind(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o666))?;

    println!("Listening for connections at {}.", path.display());

    axum::serve(listener, app()).await?;

    Ok(())
}
