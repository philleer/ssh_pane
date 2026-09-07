//! Pure remote-operation helpers (command building & validation).
//!
//! Kept free of SSH so unit/functional tests can cover behavior without a server.

use crate::pathutil::{self, shell_quote};
use crate::ssh_worker::OpMode;

/// Error while validating or planning a remote operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpsError {
    EmptyFolderName,
    InvalidFolderName(String),
    SameDirectory,
}

impl std::fmt::Display for OpsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpsError::EmptyFolderName => write!(f, "Folder name cannot be empty"),
            OpsError::InvalidFolderName(n) => write!(f, "Invalid folder name: {n}"),
            OpsError::SameDirectory => {
                write!(f, "Source and destination are the same directory")
            }
        }
    }
}

impl std::error::Error for OpsError {}

/// Validate a single path segment used as a new folder name.
pub fn validate_folder_name(name: &str) -> Result<&str, OpsError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(OpsError::EmptyFolderName);
    }
    if name.contains('/') || name.contains('\0') || name == "." || name == ".." {
        return Err(OpsError::InvalidFolderName(name.to_string()));
    }
    Ok(name)
}

/// True when transferring `src` into `dst_dir` would be a no-op same-dir drop.
pub fn is_same_directory_transfer(src: &str, dst_dir: &str) -> bool {
    let parent = pathutil::parent(src);
    let dst = dst_dir.trim_end_matches('/');
    parent == dst || parent == dst_dir
}

/// Destination path after copy/move into `dst_dir`.
pub fn destination_path(src: &str, dst_dir: &str) -> String {
    pathutil::join(dst_dir, pathutil::basename(src))
}

/// Shell command to create a directory.
pub fn mkdir_command(path: &str) -> String {
    format!("mkdir -- {}", shell_quote(path))
}

/// Shell command to remove a path recursively (used before overwrite).
pub fn remove_command(path: &str) -> String {
    format!("rm -rf -- {}", shell_quote(path))
}

/// Shell command for remote copy.
pub fn copy_command(src: &str, dst_dir: &str) -> String {
    format!(
        "cp -a -- {} {}/",
        shell_quote(src),
        shell_quote(dst_dir)
    )
}

/// Shell command for remote move.
pub fn move_command(src: &str, dst_dir: &str) -> String {
    format!(
        "mv -- {} {}/",
        shell_quote(src),
        shell_quote(dst_dir)
    )
}

/// Ordered shell commands that implement a transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferPlan {
    pub src: String,
    pub dst_dir: String,
    pub dest: String,
    pub mode: OpMode,
    pub overwrite: bool,
    pub commands: Vec<String>,
}

/// Build the remote command sequence for CP/MV.
pub fn plan_transfer(
    src: &str,
    dst_dir: &str,
    mode: OpMode,
    overwrite: bool,
) -> Result<TransferPlan, OpsError> {
    if is_same_directory_transfer(src, dst_dir) {
        return Err(OpsError::SameDirectory);
    }
    let dest = destination_path(src, dst_dir);
    let mut commands = Vec::new();
    if overwrite {
        commands.push(remove_command(&dest));
    }
    match mode {
        OpMode::Copy => commands.push(copy_command(src, dst_dir)),
        OpMode::Move => commands.push(move_command(src, dst_dir)),
    }
    Ok(TransferPlan {
        src: src.to_string(),
        dst_dir: dst_dir.to_string(),
        dest,
        mode,
        overwrite,
        commands,
    })
}

/// Full remote path for a newly created folder.
pub fn plan_mkdir(parent: &str, name: &str) -> Result<(String, String), OpsError> {
    let name = validate_folder_name(name)?;
    let path = pathutil::join(parent, name);
    Ok((path.clone(), mkdir_command(&path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_name_ok() {
        assert_eq!(validate_folder_name("  docs  ").unwrap(), "docs");
    }

    #[test]
    fn folder_name_rejects_slash() {
        assert!(matches!(
            validate_folder_name("a/b"),
            Err(OpsError::InvalidFolderName(_))
        ));
    }

    #[test]
    fn folder_name_rejects_empty() {
        assert_eq!(validate_folder_name("   "), Err(OpsError::EmptyFolderName));
    }

    #[test]
    fn same_dir_detects_parent() {
        assert!(is_same_directory_transfer("/tmp/a.txt", "/tmp"));
        assert!(is_same_directory_transfer("/tmp/a.txt", "/tmp/"));
        assert!(!is_same_directory_transfer("/tmp/a.txt", "/var"));
    }

    #[test]
    fn dest_path_joins_basename() {
        assert_eq!(
            destination_path("/home/u/file.txt", "/backup"),
            "/backup/file.txt"
        );
    }

    #[test]
    fn commands_quote_spaces() {
        let cp = copy_command("/a b/x", "/c d");
        assert!(cp.contains("'/a b/x'"));
        assert!(cp.contains("'/c d'/"));
    }

    #[test]
    fn plan_copy_without_overwrite() {
        let plan = plan_transfer("/src/f", "/dst", OpMode::Copy, false).unwrap();
        assert_eq!(plan.commands.len(), 1);
        assert!(plan.commands[0].starts_with("cp -a --"));
        assert_eq!(plan.dest, "/dst/f");
    }

    #[test]
    fn plan_move_with_overwrite_prepends_rm() {
        let plan = plan_transfer("/src/f", "/dst", OpMode::Move, true).unwrap();
        assert_eq!(plan.commands.len(), 2);
        assert!(plan.commands[0].starts_with("rm -rf --"));
        assert!(plan.commands[1].starts_with("mv --"));
    }

    #[test]
    fn plan_rejects_same_directory() {
        assert_eq!(
            plan_transfer("/tmp/x", "/tmp", OpMode::Copy, false),
            Err(OpsError::SameDirectory)
        );
    }

    #[test]
    fn plan_mkdir_builds_path_and_cmd() {
        let (path, cmd) = plan_mkdir("/data", "new_dir").unwrap();
        assert_eq!(path, "/data/new_dir");
        assert_eq!(cmd, "mkdir -- '/data/new_dir'");
    }
}
