use std::error::Error;

use contain::daemon::serve_api_on_unix_socket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    serve_api_on_unix_socket().await?;
    Ok(())
}
