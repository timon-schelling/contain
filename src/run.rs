use rand::Rng;
use regex::Regex;
use serde_json::json;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use std::{env, fs, io, thread};
use thiserror::Error;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::watch;
use tokio::time::sleep;

use crate::client::{delete_tap_device, request_tap_device, RequestError};
use crate::config::*;

#[derive(Error, Debug)]
pub enum VmError {
    #[error("cannot read environment variable USER to determine current user")]
    UserEnvUnavailable(env::VarError),
    #[error("cannot read environment variable XDG_RUNTIME_DIR to determine runtime dir")]
    XDGRuntimeDirEnvUnavailable(env::VarError),
    #[error("cannot read environment variable WAYLAND_DISPLAY to determine wayland socket")]
    WaylandSocketEnvUnavailable(env::VarError),

    #[error("xdg runtime dir unavailable")]
    XDGRuntimeDirUnavailable(Option<io::Error>),

    #[error("wayland socket unavailable")]
    WaylandSocketUnavailable(Option<io::Error>),

    #[error("error while taking to daemon")]
    DaemonRequest(#[from] RequestError),

    #[error("failed to spawn vm process")]
    FailedToSpawnVMProcess(io::Error),
    #[error("failed to kill vm process")]
    FailedToKillVMProcess(io::Error),
    #[error("failed to wait on vm process")]
    FailedToWaitOnVMProcess(io::Error),

    #[error("failed to spawn support process")]
    FailedToSpawnSupportProcess(io::Error),
    #[error("failed to kill support process")]
    FailedToKillSupportProcess(io::Error),
    #[error("failed to wait on support process")]
    FailedToWaitOnSupportProcess(io::Error),

    #[error("failed to create vm dir")]
    FailedToCreateVmDir(io::Error),
    #[error("failed to delete vm dir")]
    FailedToDeleteVmDir(io::Error),

    #[error("failed to check for support socket")]
    FailedToCheckForSupportSocket(io::Error),

    #[error("invalid kernel path")]
    InvalidKernelPath(Option<io::Error>),
    #[error("invalid initrd path")]
    InvalidInitRDPath(Option<io::Error>),

    #[error("invalid share tag")]
    InvalidShareTag(IdentifierValidationError),
    #[error("invalid share tag")]
    InvalidShareSource(Option<io::Error>),
}

pub async fn run_vm(config: Config) -> Result<(), VmError> {
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let shutdown_tx_clone = shutdown_tx.clone();

    tokio::spawn(async move {
        let mut terminate = signal(SignalKind::terminate()).unwrap();
        let mut interrupt = signal(SignalKind::interrupt()).unwrap();
        let mut quit = signal(SignalKind::quit()).unwrap();
        let mut hangup = signal(SignalKind::hangup()).unwrap();
        select! {
            _ = terminate.recv() => {},
            _ = interrupt.recv() => {},
            _ = quit.recv() => {},
            _ = hangup.recv() => {},
        }
        shutdown_tx_clone
            .send(true)
            .expect("sending shutdown signal should work");
    });

    let runtime_dir_env =
        env::var("XDG_RUNTIME_DIR").map_err(|e| VmError::XDGRuntimeDirEnvUnavailable(e))?;
    let runtime_dir = PathBuf::from(runtime_dir_env);
    runtime_dir.try_exists().map_failure(|o| VmError::XDGRuntimeDirUnavailable(o))?;

    let vm_id = hex::encode(&rand::rng().random::<[u8; 16]>());
    let vm_dir = runtime_dir.join("contain").join(vm_id);
    fs::create_dir_all(vm_dir.clone()).map_err(|e| VmError::FailedToCreateVmDir(e))?;

    let tap_device_name = if config.network.assign_tap_device {
        let user = env::var("USER").map_err(|e| VmError::UserEnvUnavailable(e))?;
        Some(request_tap_device(user).await?)
    } else {
        None
    };

    let mut support_processes: Vec<Child> = vec![];

    let mut support_cmds: Vec<Vec<String>> = vec![];

    let mut support_sockets: Vec<PathBuf> = vec![];

    for share in config.filesystem.shares.clone() {
        let tag = share
            .tag
            .check_is_valid_identifier()
            .map_err(|e| VmError::InvalidShareTag(e))?;

        let source = fs::canonicalize(share.source).map_err(|e| VmError::InvalidShareSource(Some(e)))?;
        source
            .try_exists()
            .map_failure(|o| VmError::InvalidShareSource(o))?;
        let source = source.to_string_lossy();

        let socket = format!("virtio-fs-{}.sock", tag);

        let mut cmd = vec![
            format!("virtiofsd"),
            format!("--socket-path", ),
            format!("{}", socket),
            format!("--tag"),
            format!("{}", tag),
            format!("--shared-dir"),
            format!("{}", source),
        ];

        if !share.write {
            cmd.push("--readonly".to_string());
        }

        support_cmds.push(cmd);
        support_sockets.push(socket.into());
    }

    let virtio_gpu_socket = if config.graphics.virtio_gpu {
        let socket = "virtio-gpu.sock";

        let wayland_display =
            env::var("WAYLAND_DISPLAY").map_err(|e| VmError::WaylandSocketEnvUnavailable(e))?;

        let wayland_socket_path = runtime_dir.join(wayland_display);
        wayland_socket_path
            .try_exists()
            .map_failure(|o| VmError::WaylandSocketUnavailable(o))?;

        let wayland_socket = wayland_socket_path.as_os_str().to_string_lossy();

        let device_params_json = json!({
            "context-types": "virgl:virgl2:cross-domain",
            "displays": [{ "hidden":true }],
            "egl": true,
            "vulkan": true,
        });
        let device_params = serde_json::to_string(&device_params_json).expect("this is valid json");

        let cmd = vec![
            format!("crosvm"),
            format!("device"),
            format!("gpu"),
            format!("--socket={}", socket),
            format!("--wayland-sock={}", wayland_socket),
            format!("--params={}", device_params),
        ];

        support_cmds.push(cmd);
        support_sockets.push(socket.into());

        Some(socket)
    } else {
        None
    };

    for cmd in support_cmds.iter() {
        support_processes.push(
            cmd.spawn(vm_dir.clone())
                .map_err(|e| VmError::FailedToSpawnSupportProcess(e))?,
        );
    }

    'wait_for_support_sockets: loop {
        for socket in support_sockets.iter() {
            select! {
                _ = sleep(Duration::from_millis(100)) => {},
                _ = shutdown_rx.changed() => {
                    break 'wait_for_support_sockets;
                },
            };
            if !vm_dir.join(socket)
                .try_exists()
                .map_err(|e| VmError::FailedToCheckForSupportSocket(e))?
            {
                continue;
            }
            break 'wait_for_support_sockets;
        }
    }

    config
        .kernel_path
        .try_exists()
        .map_failure(|o| VmError::InvalidKernelPath(o))?;
    let kernel_path = config.kernel_path.to_string_lossy();

    config
        .initrd_path
        .try_exists()
        .map_failure(|o| VmError::InvalidInitRDPath(o))?;
    let initrd_path = config.initrd_path.to_string_lossy();

    let mut vm_cmd = vec![
        format!("cloud-hypervisor"),
        format!("--kernel"),
        format!("{}", kernel_path),
        format!("--initramfs"),
        format!("{}", initrd_path),
        format!("--cmdline"),
        format!("{}", config.cmdline),
        format!("--seccomp=true"),
        format!(
            "--memory=mergeable=on,shared=on,size={}M",
            config.memory.size
        ),
        format!("--cpus"),
        format!("boot={}", config.cpu.cores),
        format!("--watchdog"),
        format!("--console"),
        match config.console.mode {
            console::Mode::On => format!("tty"),
            _ => format!("off"),
        },
        format!("--serial"),
        match config.console.mode {
            console::Mode::Serial => format!("tty"),
            _ => format!("off"),
        },
    ];
    if let Some(virtio_gpu_socket) = virtio_gpu_socket {
        vm_cmd.push(format!("--gpu"));
        vm_cmd.push(format!("socket={}", virtio_gpu_socket));
    }
    if !config.filesystem.shares.is_empty() {
        vm_cmd.push(format!("--fs"));
    }
    for share in config.filesystem.shares {
        vm_cmd.push(format!(
            "socket=virtio-fs-{}.sock,tag={}",
            share.tag, share.tag
        ));
    }
    if let Some(tap_device) = tap_device_name.clone() {
        vm_cmd.push(format!(
            "--net=num_queues={},tap={}",
            config.cpu.cores, tap_device
        ))
    }

    let vm_process = shared_child::SharedChild::new(
        match config.console.mode {
            console::Mode::Off => vm_cmd.spawn(vm_dir.clone()),
            _ => vm_cmd.spawn_piped(vm_dir.clone()),
        }.map_err(|e| VmError::FailedToSpawnVMProcess(e))?
    )
    .map_err(|e| VmError::FailedToSpawnVMProcess(e))?;
    let vm_process_arc = Arc::new(vm_process);

    let vm_process_arc_clone = vm_process_arc.clone();
    _ = thread::spawn(move || {
        _ = vm_process_arc_clone.wait();
        shutdown_tx.send(true)
    });

    _ = shutdown_rx.wait_for(|b| *b).await;

    vm_process_arc
        .kill()
        .map_err(|e| VmError::FailedToKillVMProcess(e))?;

    vm_process_arc
        .wait()
        .map_err(|e| VmError::FailedToWaitOnVMProcess(e))?;

    for process in support_processes.iter_mut() {
        process
            .kill()
            .map_err(|e| VmError::FailedToKillSupportProcess(e))?;
    }

    for process in support_processes.iter_mut() {
        process
            .wait()
            .map_err(|e| VmError::FailedToWaitOnSupportProcess(e))?;
    }

    if let Some(name) = tap_device_name {
        delete_tap_device(name).await?;
    }

    fs::remove_dir_all(vm_dir).map_err(|e| VmError::FailedToDeleteVmDir(e))?;

    Ok(())
}

trait MapFailure<T, E, M: Fn(Option<E>) -> T> {
    fn map_failure(self, m: M) -> Result<(), T>;
}

impl<T, E, M: Fn(Option<E>) -> T> MapFailure<T, E, M> for Result<bool, E> {
    fn map_failure(self, m: M) -> Result<(), T> {
        match self {
            Ok(b) => {
                if b {
                    Ok(())
                } else {
                    Err(m(None))
                }
            }
            Err(e) => Err(m(Some(e))),
        }
    }
}

trait Cmd {
    fn spawn(&self, path: PathBuf) -> Result<std::process::Child, std::io::Error>;
    fn spawn_piped(&self, path: PathBuf) -> Result<std::process::Child, std::io::Error>;
}

impl Cmd for Vec<String> {
    fn spawn(&self, path: PathBuf) -> Result<std::process::Child, std::io::Error> {
        let mut iter = self.iter();
        Command::new(iter.next().unwrap())
            .args(iter.collect::<Vec<&String>>())
            .current_dir(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    }
    fn spawn_piped(&self, path: PathBuf) -> Result<std::process::Child, std::io::Error> {
        let mut iter = self.iter();
        Command::new(iter.next().unwrap())
            .args(iter.collect::<Vec<&String>>())
            .current_dir(path)
            .spawn()
    }
}

trait CheckIsValidIdentifier {
    fn check_is_valid_identifier(self) -> Result<String, IdentifierValidationError>;
}

impl CheckIsValidIdentifier for String {
    fn check_is_valid_identifier(self) -> Result<String, IdentifierValidationError> {
        let regex = &*IDENTIFIER_REGEX;
        if regex.is_match(self.as_str()) {
            Ok(self)
        } else {
            Err(IdentifierValidationError(self))
        }
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
