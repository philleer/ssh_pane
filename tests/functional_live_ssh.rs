//! Optional live SSH functional test.
//!
//! Ignored by default. Enable with:
//! ```bash
//! SSH_PANE_TEST_HOST=127.0.0.1 \
//! SSH_PANE_TEST_PORT=22 \
//! SSH_PANE_TEST_USER=me \
//! SSH_PANE_TEST_PASSWORD=secret \
//! cargo test --test functional_live_ssh -- --ignored --nocapture
//! ```

use std::time::Duration;

use ssh_pane::ssh_worker::{
    ConnectParams, PaneSide, WorkerEvent, WorkerHandle, WorkerRequest,
};

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.is_empty())
}

#[test]
#[ignore = "requires live SSH; set SSH_PANE_TEST_* env vars"]
fn scenario_live_list_home_and_mkdir_cleanup() {
    ssh_pane::logging::init();

    let host = env("SSH_PANE_TEST_HOST").expect("SSH_PANE_TEST_HOST");
    let port: u16 = env("SSH_PANE_TEST_PORT")
        .unwrap_or_else(|| "22".into())
        .parse()
        .expect("port");
    let user = env("SSH_PANE_TEST_USER").expect("SSH_PANE_TEST_USER");
    let password = env("SSH_PANE_TEST_PASSWORD").unwrap_or_default();
    let key = env("SSH_PANE_TEST_KEY").map(std::path::PathBuf::from);

    let worker = WorkerHandle::spawn();
    worker.send(WorkerRequest::Connect(ConnectParams {
        host,
        port,
        user,
        password,
        private_key: key,
    }));

    let home = match recv_event(&worker, Duration::from_secs(30)).expect("connect event") {
        WorkerEvent::Connected { home } => home,
        WorkerEvent::ConnectFailed(e) => panic!("connect failed: {e}"),
        other => panic!("unexpected: {other:?}"),
    };

    worker.send(WorkerRequest::ListDir {
        side: PaneSide::Left,
        path: home.clone(),
    });
    match recv_event(&worker, Duration::from_secs(15)).expect("list event") {
        WorkerEvent::Listed { path, entries, .. } => {
            assert_eq!(path, home);
            let _ = entries;
        }
        WorkerEvent::ListFailed { error, .. } => panic!("list failed: {error}"),
        other => panic!("unexpected: {other:?}"),
    }

    let folder = format!("ssh_pane_test_{}", std::process::id());
    worker.send(WorkerRequest::Mkdir {
        side: PaneSide::Left,
        parent: home.clone(),
        name: folder.clone(),
    });
    // Busy then MkdirOk
    let mut saw_ok = false;
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while std::time::Instant::now() < deadline {
        if let Some(ev) = worker.try_recv() {
            match ev {
                WorkerEvent::Busy(_) => {}
                WorkerEvent::MkdirOk { path, .. } => {
                    assert!(path.ends_with(&folder));
                    saw_ok = true;
                    break;
                }
                WorkerEvent::MkdirFailed { error } => panic!("mkdir failed: {error}"),
                other => panic!("unexpected: {other:?}"),
            }
        } else {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    assert!(saw_ok, "timed out waiting for MkdirOk");

    worker.send(WorkerRequest::Shutdown);
}

fn recv_event(worker: &WorkerHandle, timeout: Duration) -> Option<WorkerEvent> {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if let Some(ev) = worker.try_recv() {
            return Some(ev);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}
