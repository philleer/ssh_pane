//! Functional tests: remote operation planning scenarios (no live SSH).
//!
//! These exercises cover end-to-end *decision* flows the UI/worker rely on:
//! copy/move planning, overwrite sequences, mkdir validation, and quoting.

use ssh_pane::ops::{
    self, OpsError, copy_command, move_command, plan_mkdir, plan_transfer, validate_folder_name,
};
use ssh_pane::ssh_worker::OpMode;

#[test]
fn scenario_copy_file_between_folders() {
    let plan = plan_transfer("/data/a/report.pdf", "/data/b", OpMode::Copy, false)
        .expect("copy plan");
    assert_eq!(plan.dest, "/data/b/report.pdf");
    assert_eq!(plan.commands, vec![copy_command("/data/a/report.pdf", "/data/b")]);
}

#[test]
fn scenario_move_with_overwrite() {
    let plan = plan_transfer("/left/item", "/right", OpMode::Move, true).expect("move plan");
    assert_eq!(plan.commands.len(), 2);
    assert!(plan.commands[0].contains("rm -rf --"));
    assert_eq!(plan.commands[1], move_command("/left/item", "/right"));
}

#[test]
fn scenario_drag_within_same_folder_is_rejected() {
    let err = plan_transfer("/home/u/docs/x", "/home/u/docs", OpMode::Copy, false)
        .expect_err("same dir");
    assert_eq!(err, OpsError::SameDirectory);
}

#[test]
fn scenario_new_folder_happy_path() {
    let (path, cmd) = plan_mkdir("/projects", "sprint-42").unwrap();
    assert_eq!(path, "/projects/sprint-42");
    assert!(cmd.contains("mkdir --"));
    assert!(cmd.contains("'/projects/sprint-42'"));
}

#[test]
fn scenario_new_folder_rejects_traversal() {
    assert!(validate_folder_name("../etc").is_err());
    assert!(validate_folder_name("a/b").is_err());
    assert!(ops::plan_mkdir("/tmp", "..").is_err());
}

#[test]
fn scenario_special_chars_are_shell_quoted() {
    let plan = plan_transfer("/tmp/my file.txt", "/out dir", OpMode::Copy, false).unwrap();
    let cmd = &plan.commands[0];
    assert!(cmd.contains("'/tmp/my file.txt'"));
    assert!(cmd.contains("'/out dir'/"));
}
