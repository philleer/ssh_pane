//! Functional tests: path utilities as used by navigation / DnD.

use ssh_pane::pathutil::{basename, join, normalize, parent, shell_quote};

#[test]
fn scenario_navigate_up_from_nested_path() {
    let cwd = "/var/log/nginx";
    assert_eq!(parent(cwd), "/var/log");
    assert_eq!(parent(&parent(cwd)), "/var");
    assert_eq!(parent("/"), "/");
}

#[test]
fn scenario_enter_subdir_via_join() {
    let cwd = "/home/alice";
    let next = join(&cwd, "Downloads");
    assert_eq!(next, "/home/alice/Downloads");
    assert_eq!(basename(&next), "Downloads");
}

#[test]
fn scenario_user_types_messy_path() {
    assert_eq!(normalize("  //opt///tools/  "), "/opt/tools");
    assert_eq!(normalize("relative/dir"), "/relative/dir");
}

#[test]
fn scenario_quote_preserves_spaces_and_quotes() {
    assert_eq!(shell_quote("a b"), "'a b'");
    assert_eq!(shell_quote("o'reilly"), "'o'\\''reilly'");
}
