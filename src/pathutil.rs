//! Remote path helpers and shell-safe quoting.

/// Join a remote directory and a name into a POSIX path.
pub fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() || dir == "/" {
        format!("/{}", name.trim_start_matches('/'))
    } else {
        format!(
            "{}/{}",
            dir.trim_end_matches('/'),
            name.trim_start_matches('/')
        )
    }
}

/// Parent directory of a remote path. Root stays `/`.
pub fn parent(path: &str) -> String {
    let path = path.trim_end_matches('/');
    if path.is_empty() || path == "/" {
        return "/".to_string();
    }
    match path.rfind('/') {
        Some(0) => "/".to_string(),
        Some(i) => path[..i].to_string(),
        None => "/".to_string(),
    }
}

/// Basename of a remote path.
pub fn basename(path: &str) -> &str {
    let path = path.trim_end_matches('/');
    path.rsplit('/').next().unwrap_or(path)
}

/// Quote a string for a POSIX shell using single quotes.
pub fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_root() {
        assert_eq!(join("/", "tmp"), "/tmp");
        assert_eq!(join("", "tmp"), "/tmp");
    }

    #[test]
    fn join_nested() {
        assert_eq!(join("/home/user", "docs"), "/home/user/docs");
        assert_eq!(join("/home/user/", "/docs"), "/home/user/docs");
    }

    #[test]
    fn parent_paths() {
        assert_eq!(parent("/"), "/");
        assert_eq!(parent("/tmp"), "/");
        assert_eq!(parent("/home/user"), "/home");
        assert_eq!(parent("/home/user/"), "/home");
    }

    #[test]
    fn quote_simple() {
        assert_eq!(shell_quote("abc"), "'abc'");
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
    }
}
