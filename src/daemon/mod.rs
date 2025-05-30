use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::{error::Error, path::Path};
use tokio::net::UnixListener;

pub mod api;
pub mod requests;

pub static MANAGED_RESOURCES_PREFIX: &str = "contain-";
pub static DEFAULT_SOCKET_PATH: &str = "/run/contain.sock";

pub async fn serve_api_on_unix_socket() -> Result<(), Box<dyn Error + Send + Sync>> {
    let socket_path = DEFAULT_SOCKET_PATH.to_string();
    let socket = create_socket(socket_path.clone())?;

    println!("Serving api at {}.", socket_path);

    axum::serve(socket, api::root()).await?;

    Ok(())
}

fn create_socket(socket: String) -> Result<UnixListener, Box<dyn Error + Send + Sync>> {
    let path = Path::new(&socket);
    if path.exists() {
        fs::remove_file(path)?;
    }
    let listener = UnixListener::bind(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o666))?;
    Ok(listener)
}
