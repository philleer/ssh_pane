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
    if path.is_empty() {
        return "/";
    }
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

/// Normalize a user-entered remote path to absolute POSIX form.
pub fn normalize(p: &str) -> String {
    let p = p.trim().trim_end_matches('/');
    if p.is_empty() {
        return "/".into();
    }
    let s = if p.starts_with('/') {
        p.to_string()
    } else {
        format!("/{p}")
    };
    let mut out = String::with_capacity(s.len());
    let mut prev_slash = false;
    for ch in s.chars() {
        if ch == '/' {
            if !prev_slash {
                out.push('/');
            }
            prev_slash = true;
        } else {
            out.push(ch);
            prev_slash = false;
        }
    }
    if out.is_empty() {
        "/".into()
    } else {
        out
    }
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
        assert_eq!(parent("relative"), "/");
    }

    #[test]
    fn basename_paths() {
        assert_eq!(basename("/home/user/file.txt"), "file.txt");
        assert_eq!(basename("/home/user/"), "user");
        assert_eq!(basename("/"), "/");
    }

    #[test]
    fn quote_simple() {
        assert_eq!(shell_quote("abc"), "'abc'");
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
    }

    #[test]
    fn normalize_paths() {
        assert_eq!(normalize(""), "/");
        assert_eq!(normalize("tmp"), "/tmp");
        assert_eq!(normalize("/tmp//a/"), "/tmp/a");
        assert_eq!(normalize("///"), "/");
    }
}
