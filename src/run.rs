use regex::Regex;
use serde_json::json;
use tokio::time::sleep;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::LazyLock;
use std::time::Duration;
use std::{env, io};
use thiserror::Error;

use crate::client::{delete_tap_device, request_tap_device, RequestError};
use crate::config::Config;

#[derive(Error, Debug)]
pub enum VmError {
    #[error("cannot read environment variable USER to determine current user")]
    UserEnvUnavailable(env::VarError),
    #[error("cannot read environment variable XDG_RUNTIME_DIR to determine runtime dir")]
    XDGRuntimeDirEnvUnavailable(env::VarError),
    #[error("cannot read environment variable WAYLAND_DISPLAY to determine wayland socket")]
    WaylandSocketEnvUnavailable(env::VarError),
    #[error("wayland socket unavailable")]
    WaylandSocketUnavailable(Option<io::Error>),
    #[error("error while taking to daemon")]
    DaemonRequest(#[from] RequestError),
    #[error("failed to spawn subprocess")]
    FailedToSpawnSubprocess(io::Error),
    #[error("failed to kill subprocess")]
    FailedToKillSubprocess(io::Error),
    #[error("failed to wait on subprocess")]
    FailedToWaitOnSubprocess(io::Error),
    #[error("invalid share tag")]
    InvalidShareTag(IdentifierValidationError),
    #[error("invalid share tag")]
    InvalidShareSource(Option<io::Error>),
    #[error("failed to check for support socket")]
    FailedToCheckForSupportSocket(io::Error),
}

pub async fn run_vm(config: Config) -> Result<(), VmError> {
    let tap_device_name = if config.network.assign_tap_device {
        let user = env::var("USER").map_err(|e| VmError::UserEnvUnavailable(e))?;
        Some(request_tap_device(user).await?)
    } else {
        None
    };

    let mut support_processes: Vec<Child> = vec![];

    let mut support_cmds: Vec<Vec<String>> = vec![];

    let mut support_sockets: Vec<PathBuf> = vec![];

    for share in config.filesystem.shares {
        let tag = validate_identifier(share.tag).map_err(|e| VmError::InvalidShareTag(e))?;
        let source = if share
            .source
            .try_exists()
            .map_err(|e| VmError::InvalidShareSource(Some(e)))?
        {
            share.source.to_string_lossy()
        } else {
            return Err(VmError::InvalidShareSource(None));
        };
        let socket = format!("virtio-fs-{}.sock", tag);

        let mut cmd = vec![
            "virtiofsd".to_string(),
            format!("--socket-path={}", socket),
            format!("--tag={}", tag),
            format!("--shared-dir={}", source),
        ];

        if !share.write {
            cmd.push("--readonly".to_string());
        }

        support_cmds.push(cmd);
        support_sockets.push(socket.into());
    }

    {
        let socket = "virtio-gpu.sock";

        let runtime_dir = env::var("XDG_RUNTIME_DIR").map_err(|e| VmError::XDGRuntimeDirEnvUnavailable(e))?;
        let wayland_display = env::var("WAYLAND_DISPLAY").map_err(|e| VmError::WaylandSocketEnvUnavailable(e))?;

        let wayland_socket_path = PathBuf::from(format!("{}/{}", runtime_dir, wayland_display));
        if !wayland_socket_path.try_exists().map_err(|e| VmError::WaylandSocketUnavailable(Some(e)))? {
            return Err(VmError::WaylandSocketUnavailable(None));
        }
        let wayland_socket = wayland_socket_path.as_os_str().to_string_lossy();


        let device_params_json = json!({
            "context-types": "virgl:virgl2:cross-domain",
            "displays": [{ "hidden":true }],
            "egl": true,
            "vulkan": true,
        });
        let device_params = serde_json::to_string(&device_params_json).expect("this is valid json");

        let cmd = vec![
            "crosvm".to_string(),
            "device".to_string(),
            "gpu".to_string(),
            format!("--socket={}", socket),
            format!("--wayland-sock={}", wayland_socket),
            format!("--params={}", device_params),
        ];

        support_cmds.push(cmd);
        support_sockets.push(socket.into());
    }

    'wait_for_support_sockets: loop {
        for socket in support_sockets.iter() {
            sleep(Duration::from_millis(100)).await;
            if !socket.try_exists().map_err(|e| VmError::FailedToCheckForSupportSocket(e))? {
                continue;
            }
            break 'wait_for_support_sockets;
        }
    }

    for cmd in support_cmds.iter() {
        support_processes.push(
            cmd.spawn()
                .map_err(|e| VmError::FailedToSpawnSubprocess(e))?,
        );
    }

    for process in support_processes.iter_mut() {
        process
            .kill()
            .map_err(|e| VmError::FailedToKillSubprocess(e))?;
    }

    for process in support_processes.iter_mut() {
        process
            .wait()
            .map_err(|e| VmError::FailedToWaitOnSubprocess(e))?;
    }

    if let Some(name) = tap_device_name {
        delete_tap_device(name).await?;
    }
    Ok(())
}

trait Cmd {
    fn spawn(&self) -> Result<std::process::Child, std::io::Error>;
}
impl Cmd for Vec<String> {
    fn spawn(&self) -> Result<std::process::Child, std::io::Error> {
        let mut iter = self.iter();
        Command::new(iter.next().unwrap())
            .args(iter.collect::<Vec<&String>>())
            .spawn()
    }
}

static IDENTIFIER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9\._-]+$").unwrap());

#[derive(Error, Debug)]
pub struct IdentifierValidationError(String);

impl Display for IdentifierValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "\"{}\" is not a valid identifier, should match {}",
                self.0, &*IDENTIFIER_REGEX,
            )
            .as_str(),
        )
    }
}

fn validate_identifier(name: String) -> Result<String, IdentifierValidationError> {
    let regex = &*IDENTIFIER_REGEX;
    if regex.is_match(name.as_str()) {
        Ok(name)
    } else {
        Err(IdentifierValidationError(name))
    }
}
