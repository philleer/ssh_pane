//! Background SSH worker: connect, SFTP list, remote cp/mv.

use std::io::Read;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use ssh2::Session;

use crate::pathutil::{self, shell_quote};

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
    Shutdown,
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

        Self {
            tx: req_tx,
            rx: ev_rx,
        }
    }

    pub fn send(&self, req: WorkerRequest) {
        let _ = self.tx.send(req);
    }

    pub fn try_recv(&self) -> Option<WorkerEvent> {
        self.rx.try_recv().ok()
    }
}

struct SessionState {
    session: Session,
}

fn worker_loop(req_rx: Receiver<WorkerRequest>, ev_tx: Sender<WorkerEvent>) {
    let mut state: Option<SessionState> = None;

    while let Ok(req) = req_rx.recv() {
        match req {
            WorkerRequest::Shutdown => break,
            WorkerRequest::Disconnect => {
                state = None;
                let _ = ev_tx.send(WorkerEvent::Disconnected);
            }
            WorkerRequest::Connect(params) => {
                state = None;
                match connect(&params) {
                    Ok((sess, home)) => {
                        state = Some(sess);
                        let _ = ev_tx.send(WorkerEvent::Connected { home });
                    }
                    Err(e) => {
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
                match list_dir(&s.session, &path) {
                    Ok(entries) => {
                        let _ = ev_tx.send(WorkerEvent::Listed {
                            side,
                            path,
                            entries,
                        });
                    }
                    Err(e) => {
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
                let dest = pathutil::join(&dst_dir, pathutil::basename(&src));
                match path_exists(&s.session, &dest) {
                    Ok(true) if !overwrite => {
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
                        match transfer(&s.session, &src, &dst_dir, mode, true) {
                            Ok(()) => {
                                let _ = ev_tx.send(WorkerEvent::TransferOk {
                                    src,
                                    dst_dir,
                                    mode,
                                });
                            }
                            Err(e) => {
                                let _ = ev_tx.send(WorkerEvent::TransferFailed { error: e });
                            }
                        }
                    }
                    Err(e) => {
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
                match transfer(&s.session, &src, &dst_dir, mode, overwrite) {
                    Ok(()) => {
                        let _ = ev_tx.send(WorkerEvent::TransferOk {
                            src,
                            dst_dir,
                            mode,
                        });
                    }
                    Err(e) => {
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
                let path = pathutil::join(&parent, &name);
                let _ = ev_tx.send(WorkerEvent::Busy(format!("mkdir {path}")));
                match mkdir(&s.session, &path) {
                    Ok(()) => {
                        let _ = ev_tx.send(WorkerEvent::MkdirOk { side, path });
                    }
                    Err(e) => {
                        let _ = ev_tx.send(WorkerEvent::MkdirFailed { error: e });
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

    let home = resolve_home(&session)?;
    Ok((SessionState { session }, home))
}

fn resolve_home(session: &Session) -> Result<String, String> {
    // Prefer SFTP realpath of "."
    let sftp = session.sftp().map_err(|e| format!("SFTP open: {e}"))?;
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

fn list_dir(session: &Session, path: &str) -> Result<Vec<RemoteEntry>, String> {
    let sftp = session.sftp().map_err(|e| format!("SFTP open: {e}"))?;
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

fn path_exists(session: &Session, path: &str) -> Result<bool, String> {
    let sftp = session.sftp().map_err(|e| format!("SFTP open: {e}"))?;
    match sftp.stat(std::path::Path::new(path)) {
        Ok(_) => Ok(true),
        Err(e) => {
            // LIBSSH2_ERROR_SFTP_PROTOCOL with NO_SUCH_FILE is expected for missing
            let msg = e.to_string();
            if msg.contains("NO_SUCH_FILE")
                || msg.contains("no such file")
                || msg.contains("No such file")
            {
                Ok(false)
            } else {
                // Some libssh2 builds just return a generic code; treat as missing if code-like.
                // Fallback: test -e via shell.
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

fn mkdir(session: &Session, path: &str) -> Result<(), String> {
    let cmd = format!("mkdir -- {}", shell_quote(path));
    let (code, _, err) = exec(session, &cmd)?;
    if code != 0 {
        return Err(if err.trim().is_empty() {
            format!("mkdir failed with exit code {code}")
        } else {
            err.trim().to_string()
        });
    }
    Ok(())
}

fn transfer(
    session: &Session,
    src: &str,
    dst_dir: &str,
    mode: OpMode,
    overwrite: bool,
) -> Result<(), String> {
    let dest = pathutil::join(dst_dir, pathutil::basename(src));
    if overwrite {
        // Remove existing target first so cp/mv don't nest unexpectedly for dirs.
        let rm = format!("rm -rf -- {}", shell_quote(&dest));
        let (code, _, err) = exec(session, &rm)?;
        if code != 0 {
            // Ignore missing; fail on real errors
            if !err.to_lowercase().contains("no such file") {
                // Continue anyway — cp/mv may still work
            }
        }
    }

    let cmd = match mode {
        OpMode::Copy => format!(
            "cp -a -- {} {}/",
            shell_quote(src),
            shell_quote(dst_dir)
        ),
        OpMode::Move => format!(
            "mv -- {} {}/",
            shell_quote(src),
            shell_quote(dst_dir)
        ),
    };

    let (code, _, err) = exec(session, &cmd)?;
    if code != 0 {
        return Err(if err.trim().is_empty() {
            format!("Command failed with exit code {code}")
        } else {
            err.trim().to_string()
        });
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

    let mut stdout = String::new();
    channel
        .read_to_string(&mut stdout)
        .map_err(|e| format!("read stdout: {e}"))?;

    let mut stderr = String::new();
    channel
        .stderr()
        .read_to_string(&mut stderr)
        .map_err(|e| format!("read stderr: {e}"))?;

    channel
        .wait_close()
        .map_err(|e| format!("wait_close: {e}"))?;
    let code = channel.exit_status().map_err(|e| format!("exit_status: {e}"))?;
    Ok((code, stdout, stderr))
}
