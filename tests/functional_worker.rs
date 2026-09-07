//! Functional tests: worker public API / messaging contracts (no live SSH).

use std::time::Duration;

use ssh_split_pane::ssh_worker::{
    ConnectParams, OpMode, PaneSide, WorkerEvent, WorkerHandle, WorkerRequest,
};

#[test]
fn scenario_worker_rejects_list_when_disconnected() {
    ssh_split_pane::logging::init();
    let worker = WorkerHandle::spawn();
    worker.send(WorkerRequest::ListDir {
        side: PaneSide::Left,
        path: "/tmp".into(),
    });

    let ev = recv_event(&worker, Duration::from_secs(2)).expect("event");
    match ev {
        WorkerEvent::ListFailed { side, path, error } => {
            assert_eq!(side, PaneSide::Left);
            assert_eq!(path, "/tmp");
            assert!(error.to_lowercase().contains("not connected"));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    worker.send(WorkerRequest::Shutdown);
}

#[test]
fn scenario_worker_connect_failure_is_reported() {
    ssh_split_pane::logging::init();
    let worker = WorkerHandle::spawn();
    worker.send(WorkerRequest::Connect(ConnectParams {
        host: "127.0.0.1".into(),
        port: 1, // almost certainly closed
        user: "nobody".into(),
        password: "bad".into(),
        private_key: None,
    }));

    let ev = recv_event(&worker, Duration::from_secs(20)).expect("event");
    match ev {
        WorkerEvent::ConnectFailed(msg) => {
            assert!(!msg.is_empty());
        }
        other => panic!("expected ConnectFailed, got {other:?}"),
    }

    worker.send(WorkerRequest::Shutdown);
}

#[test]
fn scenario_mode_labels_stable_for_ui() {
    assert_eq!(OpMode::Copy.label(), "CP");
    assert_eq!(OpMode::Move.label(), "MV");
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
