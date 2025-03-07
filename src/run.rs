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
use qcow2_rs::meta::Qcow2Header;
use qcow2_rs::error::Qcow2Error;
use std::io::{BufRead, Write};

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

    #[error("failed to start disk creation process")]
    FailedToStartDiskCreationProcess(io::Error),
    #[error("failed to wait on disk creation process")]
    FailedToWaitOnDiskCreationProcess(io::Error),

    #[error("failed to check for support socket")]
    FailedToCheckForSupportSocket(io::Error),

    #[error("failed to create disk empty dir")]
    FailedToCreateDiskEmptyDir(io::Error),
    #[error("failed to delete disk empty dir")]
    FailedToDeleteDiskEmptyDir(io::Error),

    #[error("invalid kernel path")]
    InvalidKernelPath(Option<io::Error>),
    #[error("invalid initrd path")]
    InvalidInitRDPath(Option<io::Error>),

    #[error("invalid share tag")]
    InvalidShareTag(IdentifierValidationError),
    #[error("invalid share source")]
    InvalidShareSource(Option<io::Error>),


    #[error("invalid disk tag")]
    InvalidDiskTag(IdentifierValidationError),
    #[error("invalid disk tag")]
    InvalidDiskSource(Option<io::Error>),

    #[error("failed to create disk")]
    FailedToCreateDisk(Qcow2Error),
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
        env::var("XDG_RUNTIME_DIR").map_err(VmError::XDGRuntimeDirEnvUnavailable)?;
    let runtime_dir = PathBuf::from(runtime_dir_env);
    runtime_dir
        .try_exists()
        .map_failure(VmError::XDGRuntimeDirUnavailable)?;

    let vm_id = hex::encode(&rand::rng().random::<[u8; 16]>());
    let vm_dir = runtime_dir.join("contain").join(vm_id);
    fs::create_dir_all(vm_dir.clone()).map_err(VmError::FailedToCreateVmDir)?;

    let tap_device_name = if config.network.assign_tap_device {
        let user = env::var("USER").map_err(VmError::UserEnvUnavailable)?;
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
            .map_err(VmError::InvalidShareTag)?;

        let source =
            fs::canonicalize(share.source).map_err(|e| VmError::InvalidShareSource(Some(e)))?;
        source
            .try_exists()
            .map_failure(VmError::InvalidShareSource)?;
        let source = source.to_string_lossy();

        let socket = format!("virtio-fs-{}.sock", tag);

        let mut cmd = vec![
            format!("virtiofsd"),
            format!("--socket-path"),
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

    for disk in config.filesystem.disks.clone() {
        if !disk.create {
            continue;
        }
        let path = disk.source;
        if path.try_exists().map_err(|e| VmError::InvalidDiskSource(Some(e)))? {
            continue;
        }
        if let Some(parrent) = path.parent() {
            fs::create_dir_all(parrent).map_err(|e| VmError::InvalidDiskSource(Some(e)))?;
        }
        let mut file = std::fs::File::create(path).map_err(|e| VmError::InvalidDiskSource(Some(e)))?;
        let size = disk.size * 1024 * 1024;
        let cluster_bits = 16;
        let refcount_order = 4;
        let bs_shift = 9_u8;
        let bs = 1 << bs_shift;
        let (rc_t, rc_b, _) =
            Qcow2Header::calculate_meta_params(size, cluster_bits, refcount_order, bs);
        let clusters = 1 + rc_t.1 + rc_b.1;
        let img_size = ((clusters as usize) << cluster_bits) + 512;
        let mut buf = vec![0u8; img_size];
        Qcow2Header::format_qcow2(&mut buf, size, cluster_bits, refcount_order, bs).map_err(VmError::FailedToCreateDisk)?;
        file.write_all(&buf).map_err(|e| VmError::InvalidDiskSource(Some(e)))?;
    }

    let virtio_gpu_socket = if config.graphics.virtio_gpu {
        let socket = "virtio-gpu.sock";

        let wayland_display =
            env::var("WAYLAND_DISPLAY").map_err(VmError::WaylandSocketEnvUnavailable)?;

        let wayland_socket_path = runtime_dir.join(wayland_display);
        wayland_socket_path
            .try_exists()
            .map_failure(VmError::WaylandSocketUnavailable)?;

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
                .map_err(VmError::FailedToSpawnSupportProcess)?,
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
            if !vm_dir
                .join(socket)
                .try_exists()
                .map_err(VmError::FailedToCheckForSupportSocket)?
            {
                continue;
            }
            break 'wait_for_support_sockets;
        }
    }

    config
        .kernel_path
        .try_exists()
        .map_failure(VmError::InvalidKernelPath)?;
    let kernel_path = config.kernel_path.to_string_lossy();

    config
        .initrd_path
        .try_exists()
        .map_failure(VmError::InvalidInitRDPath)?;
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
            console::Mode::On | console::Mode::Log => format!("tty"),
            _ => format!("null"),
        },
        format!("--serial"),
        match config.console.mode {
            console::Mode::Serial => format!("tty"),
            _ => format!("null"),
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
    if !config.filesystem.disks.is_empty() {
        vm_cmd.push(format!("--disk"));
    }
    for disk in config.filesystem.disks {
        let path =
            fs::canonicalize(disk.source).map_err(|e| VmError::InvalidDiskSource(Some(e)))?;
        let path_str = path.to_string_lossy();
        let readonly = if !disk.write { "on" } else { "off" };
        let id = disk.tag;
        vm_cmd.push(format!(
            "path={},serial={},readonly={}",
            path_str, id, readonly
        ));
    }
    if let Some(tap_device) = tap_device_name.clone() {
        vm_cmd.push(format!("--net"));
        vm_cmd.push(format!(
            "num_queues={},tap={}",
            config.cpu.cores, tap_device
        ));
    }

    let vm_process = shared_child::SharedChild::new(
        match config.console.mode {
            console::Mode::Off => vm_cmd.spawn(vm_dir.clone()),
            console::Mode::Log => vm_cmd.spawn_log(vm_dir.clone()),
            console::Mode::On | console::Mode::Serial => vm_cmd.spawn_piped(vm_dir.clone()),
        }
        .map_err(VmError::FailedToSpawnVMProcess)?,
    )
    .map_err(VmError::FailedToSpawnVMProcess)?;
    let vm_process_arc = Arc::new(vm_process);

    let vm_process_arc_clone = vm_process_arc.clone();
    _ = thread::spawn(move || {
        _ = vm_process_arc_clone.wait();
        shutdown_tx.send(true)
    });
    
    if config.console.mode == console::Mode::Log {
        let vm_process_arc_clone = vm_process_arc.clone();
        _ = thread::spawn(move || {
            let stdout = vm_process_arc_clone.take_stdout().unwrap();
            let mut reader = io::BufReader::new(stdout);
            let mut line = Vec::new();
            loop {
                line.clear();
                match reader.read_until(b'\n', &mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => (),
                }
                let line = String::from_utf8_lossy(&*line);
                print!("{}", line);
            }
        });
    }
    
    _ = shutdown_rx.wait_for(|b| *b).await;

    vm_process_arc
        .kill()
        .map_err(VmError::FailedToKillVMProcess)?;

    vm_process_arc
        .wait()
        .map_err(VmError::FailedToWaitOnVMProcess)?;

    for process in support_processes.iter_mut() {
        process
            .kill()
            .map_err(VmError::FailedToKillSupportProcess)?;
    }

    for process in support_processes.iter_mut() {
        process
            .wait()
            .map_err(VmError::FailedToWaitOnSupportProcess)?;
    }

    if let Some(name) = tap_device_name {
        delete_tap_device(name).await?;
    }

    fs::remove_dir_all(vm_dir).map_err(VmError::FailedToDeleteVmDir)?;

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
    fn spawn_log(&self, path: PathBuf) -> Result<std::process::Child, std::io::Error>;
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
    fn spawn_log(&self, path: PathBuf) -> Result<std::process::Child, std::io::Error> {
        let mut iter = self.iter();
        Command::new(iter.next().unwrap())
            .args(iter.collect::<Vec<&String>>())
            .current_dir(path)
            .stdout(Stdio::piped())
            .stdin(Stdio::null())
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
