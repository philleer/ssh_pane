//! Background SSH worker: connect, SFTP list, remote cp/mv.

use std::fmt;
use std::io::Read;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use log::{debug, error, info, warn};
use ssh2::{RenameFlags, Session, Sftp};

use crate::ops;
use crate::pathutil::shell_quote;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpMode {
    Copy,
    Move,
}

impl OpMode {
    pub fn label(self) -> &'static str {
        match self {
            OpMode::Copy => "CP",
            OpMode::Move => "MV",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectParams {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub private_key: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct RemoteEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub enum WorkerRequest {
    Connect(ConnectParams),
    Disconnect,
    ListDir {
        side: PaneSide,
        path: String,
    },
    /// Check whether `dst_dir/basename(src)` already exists.
    CheckExists {
        src: String,
        dst_dir: String,
        mode: OpMode,
        /// If true, overwrite without asking again.
        overwrite: bool,
    },
    /// Perform cp/mv. Caller must have resolved overwrite policy.
    Transfer {
        src: String,
        dst_dir: String,
        mode: OpMode,
        overwrite: bool,
    },
    /// Create a directory under `parent` with `name`.
    Mkdir {
        side: PaneSide,
        parent: String,
        name: String,
    },
    /// Read a remote file for preview (capped by `max_bytes`).
    ReadFile {
        side: PaneSide,
        path: String,
        max_bytes: u64,
    },
    Shutdown,
}

/// File bytes from the worker; Debug only shows length.
#[derive(Clone)]
pub struct FileBytes(pub Vec<u8>);

impl fmt::Debug for FileBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileBytes")
            .field("len", &self.0.len())
            .finish()
    }
}

#[derive(Debug, Clone)]
pub enum WorkerEvent {
    Connected {
        home: String,
    },
    ConnectFailed(String),
    Disconnected,
    Listed {
        side: PaneSide,
        path: String,
        entries: Vec<RemoteEntry>,
    },
    ListFailed {
        side: PaneSide,
        path: String,
        error: String,
    },
    /// Target already exists; UI should ask user.
    NeedsOverwriteConfirm {
        src: String,
        dst_dir: String,
        mode: OpMode,
        existing_path: String,
    },
    TransferOk {
        src: String,
        dst_dir: String,
        mode: OpMode,
    },
    TransferFailed {
        error: String,
    },
    MkdirOk {
        side: PaneSide,
        path: String,
    },
    MkdirFailed {
        error: String,
    },
    FileReadOk {
        side: PaneSide,
        path: String,
        bytes: FileBytes,
    },
    FileReadFailed {
        side: PaneSide,
        path: String,
        error: String,
    },
    Busy(String),
}

pub struct WorkerHandle {
    pub tx: Sender<WorkerRequest>,
    pub rx: Receiver<WorkerEvent>,
}

impl WorkerHandle {
    pub fn spawn() -> Self {
        let (req_tx, req_rx) = mpsc::channel::<WorkerRequest>();
        let (ev_tx, ev_rx) = mpsc::channel::<WorkerEvent>();

        thread::Builder::new()
            .name("ssh-worker".into())
            .spawn(move || worker_loop(req_rx, ev_tx))
            .expect("spawn ssh worker");

        info!("ssh worker thread spawned");
        Self {
            tx: req_tx,
            rx: ev_rx,
        }
    }

    pub fn send(&self, req: WorkerRequest) {
        debug!("worker request: {req:?}");
        if let Err(e) = self.tx.send(req) {
            error!("failed to send worker request: {e}");
        }
    }

    pub fn try_recv(&self) -> Option<WorkerEvent> {
        match self.rx.try_recv() {
            Ok(ev) => {
                debug!("worker event: {ev:?}");
                Some(ev)
            }
            Err(_) => None,
        }
    }
}

struct SessionState {
    session: Session,
    /// Kept open for the whole connection — opening SFTP per op is a major RTT cost.
    sftp: Sftp,
}

fn worker_loop(req_rx: Receiver<WorkerRequest>, ev_tx: Sender<WorkerEvent>) {
    let mut state: Option<SessionState> = None;

    while let Ok(req) = req_rx.recv() {
        match req {
            WorkerRequest::Shutdown => {
                info!("ssh worker shutting down");
                break;
            }
            WorkerRequest::Disconnect => {
                info!("disconnecting SSH session");
                state = None;
                let _ = ev_tx.send(WorkerEvent::Disconnected);
            }
            WorkerRequest::Connect(params) => {
                state = None;
                info!(
                    "connecting to {}@{}:{} (key={})",
                    params.user,
                    params.host,
                    params.port,
                    params.private_key.is_some()
                );
                match connect(&params) {
                    Ok((sess, home)) => {
                        info!("connected; remote home={home}");
                        state = Some(sess);
                        let _ = ev_tx.send(WorkerEvent::Connected { home });
                    }
                    Err(e) => {
                        warn!("connect failed: {e}");
                        let _ = ev_tx.send(WorkerEvent::ConnectFailed(e));
                    }
                }
            }
            WorkerRequest::ListDir { side, path } => {
                let Some(s) = state.as_mut() else {
                    let _ = ev_tx.send(WorkerEvent::ListFailed {
                        side,
                        path,
                        error: "Not connected".into(),
                    });
                    continue;
                };
                debug!("list_dir side={side:?} path={path}");
                match list_dir(&s.sftp, &path) {
                    Ok(entries) => {
                        debug!("listed {} entries in {path}", entries.len());
                        let _ = ev_tx.send(WorkerEvent::Listed {
                            side,
                            path,
                            entries,
                        });
                    }
                    Err(e) => {
                        warn!("list_dir failed for {path}: {e}");
                        let _ = ev_tx.send(WorkerEvent::ListFailed {
                            side,
                            path,
                            error: e,
                        });
                    }
                }
            }
            WorkerRequest::CheckExists {
                src,
                dst_dir,
                mode,
                overwrite,
            } => {
                let Some(s) = state.as_mut() else {
                    let _ = ev_tx.send(WorkerEvent::TransferFailed {
                        error: "Not connected".into(),
                    });
                    continue;
                };
                let dest = ops::destination_path(&src, &dst_dir);
                match path_exists(&s.sftp, &s.session, &dest) {
                    Ok(true) if !overwrite => {
                        info!("target exists, asking overwrite confirm: {dest}");
                        let _ = ev_tx.send(WorkerEvent::NeedsOverwriteConfirm {
                            src,
                            dst_dir,
                            mode,
                            existing_path: dest,
                        });
                    }
                    Ok(_) => {
                        let _ = ev_tx.send(WorkerEvent::Busy(format!(
                            "{} {} → {}",
                            mode.label(),
                            src,
                            dst_dir
                        )));
                        // Pass the real overwrite flag — previously always `true`, which
                        // forced an extra remote `rm -rf` before every cp/mv.
                        match transfer(s, &src, &dst_dir, mode, overwrite) {
                            Ok(()) => {
                                info!("{} ok: {src} → {dst_dir}", mode.label());
                                let _ = ev_tx.send(WorkerEvent::TransferOk {
                                    src,
                                    dst_dir,
                                    mode,
                                });
                            }
                            Err(e) => {
                                error!("transfer failed: {e}");
                                let _ = ev_tx.send(WorkerEvent::TransferFailed { error: e });
                            }
                        }
                    }
                    Err(e) => {
                        error!("exists check failed: {e}");
                        let _ = ev_tx.send(WorkerEvent::TransferFailed { error: e });
                    }
                }
            }
            WorkerRequest::Transfer {
                src,
                dst_dir,
                mode,
                overwrite,
            } => {
                let Some(s) = state.as_mut() else {
                    let _ = ev_tx.send(WorkerEvent::TransferFailed {
                        error: "Not connected".into(),
                    });
                    continue;
                };
                let _ = ev_tx.send(WorkerEvent::Busy(format!(
                    "{} {} → {}",
                    mode.label(),
                    src,
                    dst_dir
                )));
                match transfer(s, &src, &dst_dir, mode, overwrite) {
                    Ok(()) => {
                        info!("{} ok: {src} → {dst_dir}", mode.label());
                        let _ = ev_tx.send(WorkerEvent::TransferOk {
                            src,
                            dst_dir,
                            mode,
                        });
                    }
                    Err(e) => {
                        error!("transfer failed: {e}");
                        let _ = ev_tx.send(WorkerEvent::TransferFailed { error: e });
                    }
                }
            }
            WorkerRequest::Mkdir { side, parent, name } => {
                let Some(s) = state.as_mut() else {
                    let _ = ev_tx.send(WorkerEvent::MkdirFailed {
                        error: "Not connected".into(),
                    });
                    continue;
                };
                match ops::plan_mkdir(&parent, &name) {
                    Ok((path, _)) => {
                        let _ = ev_tx.send(WorkerEvent::Busy(format!("mkdir {path}")));
                        match mkdir_sftp(&s.sftp, &path) {
                            Ok(()) => {
                                info!("mkdir ok: {path}");
                                let _ = ev_tx.send(WorkerEvent::MkdirOk { side, path });
                            }
                            Err(e) => {
                                error!("mkdir failed: {e}");
                                let _ = ev_tx.send(WorkerEvent::MkdirFailed { error: e });
                            }
                        }
                    }
                    Err(e) => {
                        let _ = ev_tx.send(WorkerEvent::MkdirFailed {
                            error: e.to_string(),
                        });
                    }
                }
            }
            WorkerRequest::ReadFile {
                side,
                path,
                max_bytes,
            } => {
                let Some(s) = state.as_mut() else {
                    let _ = ev_tx.send(WorkerEvent::FileReadFailed {
                        side,
                        path,
                        error: "Not connected".into(),
                    });
                    continue;
                };
                debug!("read_file side={side:?} path={path} max_bytes={max_bytes}");
                match read_file(&s.sftp, &path, max_bytes) {
                    Ok(bytes) => {
                        debug!("read_file ok path={path} len={}", bytes.len());
                        let _ = ev_tx.send(WorkerEvent::FileReadOk {
                            side,
                            path,
                            bytes: FileBytes(bytes),
                        });
                    }
                    Err(e) => {
                        warn!("read_file failed for {path}: {e}");
                        let _ = ev_tx.send(WorkerEvent::FileReadFailed {
                            side,
                            path,
                            error: e,
                        });
                    }
                }
            }
        }
    }
}

fn connect(params: &ConnectParams) -> Result<(SessionState, String), String> {
    let addr = format!("{}:{}", params.host, params.port);
    let tcp = TcpStream::connect_timeout(
        &addr
            .parse()
            .map_err(|e| format!("Invalid address {addr}: {e}"))?,
        Duration::from_secs(15),
    )
    .map_err(|e| format!("TCP connect failed: {e}"))?;
    tcp.set_read_timeout(Some(Duration::from_secs(60)))
        .map_err(|e| e.to_string())?;
    tcp.set_write_timeout(Some(Duration::from_secs(60)))
        .map_err(|e| e.to_string())?;

    let mut session = Session::new().map_err(|e| format!("Session::new: {e}"))?;
    session.set_tcp_stream(tcp);
    session
        .handshake()
        .map_err(|e| format!("SSH handshake failed: {e}"))?;

    if let Some(key) = &params.private_key {
        session
            .userauth_pubkey_file(&params.user, None, key, None)
            .map_err(|e| format!("Public key auth failed: {e}"))?;
    } else {
        if params.password.is_empty() {
            return Err("Password is empty and no private key provided".into());
        }
        session
            .userauth_password(&params.user, &params.password)
            .map_err(|e| format!("Password auth failed: {e}"))?;
    }

    if !session.authenticated() {
        return Err("Authentication failed".into());
    }

    let sftp = session.sftp().map_err(|e| format!("SFTP open: {e}"))?;
    let home = resolve_home(&session, &sftp)?;
    Ok((SessionState { session, sftp }, home))
}

fn resolve_home(session: &Session, sftp: &Sftp) -> Result<String, String> {
    match sftp.realpath(std::path::Path::new(".")) {
        Ok(p) => Ok(p.to_string_lossy().replace('\\', "/")),
        Err(_) => {
            let (code, out, err) = exec(session, "pwd")?;
            if code == 0 {
                Ok(out.trim().to_string())
            } else {
                Err(format!("Cannot resolve home: {err}"))
            }
        }
    }
}

fn read_file(sftp: &Sftp, path: &str, max_bytes: u64) -> Result<Vec<u8>, String> {
    let mut remote = sftp
        .open(std::path::Path::new(path))
        .map_err(|e| format!("open {path}: {e}"))?;
    let mut buf = Vec::new();
    let mut limited = (&mut remote).take(max_bytes.saturating_add(1));
    limited
        .read_to_end(&mut buf)
        .map_err(|e| format!("read {path}: {e}"))?;
    if buf.len() as u64 > max_bytes {
        return Err(format!(
            "file too large for preview (max {} MB)",
            max_bytes / (1024 * 1024)
        ));
    }
    Ok(buf)
}

fn list_dir(sftp: &Sftp, path: &str) -> Result<Vec<RemoteEntry>, String> {
    let entries = sftp
        .readdir(std::path::Path::new(path))
        .map_err(|e| format!("readdir {path}: {e}"))?;

    let mut out = Vec::new();
    for (p, stat) in entries {
        let name = p
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| p.to_string_lossy().into_owned());
        if name == "." || name == ".." {
            continue;
        }
        let is_dir = stat.is_dir();
        out.push(RemoteEntry {
            name,
            is_dir,
            size: stat.size.unwrap_or(0),
        });
    }

    out.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    Ok(out)
}

fn path_exists(sftp: &Sftp, session: &Session, path: &str) -> Result<bool, String> {
    match sftp.stat(std::path::Path::new(path)) {
        Ok(_) => Ok(true),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("NO_SUCH_FILE")
                || msg.contains("no such file")
                || msg.contains("No such file")
                || msg.contains("not found")
            {
                Ok(false)
            } else {
                // Rare fallback — avoid unless SFTP error is ambiguous.
                let cmd = format!("test -e {} && echo yes || echo no", shell_quote(path));
                let (code, out, err) = exec(session, &cmd)?;
                if code != 0 && !out.contains("no") {
                    return Err(format!("exists check failed: {err} ({msg})"));
                }
                Ok(out.trim() == "yes")
            }
        }
    }
}

fn mkdir_sftp(sftp: &Sftp, path: &str) -> Result<(), String> {
    sftp.mkdir(std::path::Path::new(path), 0o755)
        .map_err(|e| format!("mkdir {path}: {e}"))
}

fn transfer(
    state: &SessionState,
    src: &str,
    dst_dir: &str,
    mode: OpMode,
    overwrite: bool,
) -> Result<(), String> {
    let dest = ops::destination_path(src, dst_dir);

    // Fast path: same-filesystem move via SFTP rename (one RTT, no shell).
    if mode == OpMode::Move {
        let flags = if overwrite {
            Some(RenameFlags::OVERWRITE | RenameFlags::ATOMIC)
        } else {
            Some(RenameFlags::ATOMIC)
        };
        match state
            .sftp
            .rename(std::path::Path::new(src), std::path::Path::new(&dest), flags)
        {
            Ok(()) => {
                debug!("sftp rename ok: {src} → {dest}");
                return Ok(());
            }
            Err(e) => {
                debug!("sftp rename failed ({e}), falling back to shell mv");
            }
        }
    }

    let plan = ops::plan_transfer(src, dst_dir, mode, overwrite).map_err(|e| e.to_string())?;
    for cmd in &plan.commands {
        debug!("exec: {cmd}");
        let (code, _, err) = exec(&state.session, cmd)?;
        if code != 0 {
            if cmd.starts_with("rm ") && err.to_lowercase().contains("no such file") {
                continue;
            }
            if cmd.starts_with("rm ") {
                warn!("overwrite rm warning: {err}");
                continue;
            }
            return Err(if err.trim().is_empty() {
                format!("Command failed with exit code {code}")
            } else {
                err.trim().to_string()
            });
        }
    }
    Ok(())
}

fn exec(session: &Session, command: &str) -> Result<(i32, String, String), String> {
    let mut channel = session
        .channel_session()
        .map_err(|e| format!("channel_session: {e}"))?;
    channel
        .exec(command)
        .map_err(|e| format!("exec: {e}"))?;

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    // Interleave reads so a full stderr pipe cannot stall stdout EOF.
    loop {
        let mut buf = [0u8; 8192];
        let n_out = channel.read(&mut buf).unwrap_or(0);
        if n_out > 0 {
            stdout.extend_from_slice(&buf[..n_out]);
        }
        let n_err = channel.stderr().read(&mut buf).unwrap_or(0);
        if n_err > 0 {
            stderr.extend_from_slice(&buf[..n_err]);
        }
        if n_out == 0 && n_err == 0 {
            break;
        }
    }

    channel
        .wait_close()
        .map_err(|e| format!("wait_close: {e}"))?;
    let code = channel.exit_status().map_err(|e| format!("exit_status: {e}"))?;
    Ok((
        code,
        String::from_utf8_lossy(&stdout).into_owned(),
        String::from_utf8_lossy(&stderr).into_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_mode_labels() {
        assert_eq!(OpMode::Copy.label(), "CP");
        assert_eq!(OpMode::Move.label(), "MV");
    }

    #[test]
    fn pane_side_equality() {
        assert_eq!(PaneSide::Left, PaneSide::Left);
        assert_ne!(PaneSide::Left, PaneSide::Right);
    }
}
