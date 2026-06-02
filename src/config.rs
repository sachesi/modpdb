use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const SKEL: &str = "/usr/share/modpdb/modpdb.skel";

pub struct Config {
    pub db_path: PathBuf,
    pub ignore: Vec<String>,
}

impl Config {
    /// Load configuration from `$XDG_CONFIG_HOME/modpdb.conf`.
    ///
    /// Returns `Ok(Some(Config))` if loaded, `Ok(None)` if a fresh config was
    /// just created (caller should exit), or `Err` on failure.
    pub fn load() -> Result<Option<Self>, String> {
        let home_dir = get_home_dir()?;
        // Under sudo the ambient XDG_CONFIG_HOME belongs to root, not the target
        // user, so ignore it and derive the path from the resolved user home.
        let xdg_config_home = if env::var_os("SUDO_USER").is_some() {
            home_dir.join(".config")
        } else {
            env::var("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home_dir.join(".config"))
        };

        if !xdg_config_home.exists() {
            fs::create_dir_all(&xdg_config_home)
                .map_err(|e| format!("Cannot create {}: {e}", xdg_config_home.display()))?;
        }

        let cfg_file = xdg_config_home.join("modpdb.conf");

        if !cfg_file.exists() {
            create_initial_config(&cfg_file, &home_dir)?;
            println!("------------------------------------------------------------");
            println!(" No config file found so creating a fresh one in:");
            println!(" {}", cfg_file.display());
            println!();
            println!(" Consult the man page for setup instructions.");
            println!("------------------------------------------------------------");
            return Ok(None);
        }

        parse_config(&cfg_file).map(Some)
    }
}

/// Determine the real (non-root) user's home directory.
///
/// When invoked via `sudo`, `SUDO_USER` holds the original username.
/// Falls back to `HOME` env var, then to a `/etc/passwd` lookup by UID.
fn get_home_dir() -> Result<PathBuf, String> {
    let username = if let Ok(sudo_user) = env::var("SUDO_USER") {
        if sudo_user == "root" {
            return Err("Cannot determine your username (SUDO_USER=root). \
                 Run as a regular user."
                .to_string());
        }
        sudo_user
    } else {
        resolve_login_name()
    };

    get_home_for_user(&username)
}

/// Look up the home directory for a given username from `/etc/passwd`.
fn get_home_for_user(username: &str) -> Result<PathBuf, String> {
    let passwd =
        fs::read_to_string("/etc/passwd").map_err(|e| format!("Cannot read /etc/passwd: {e}"))?;

    for line in passwd.lines() {
        let fields: Vec<&str> = line.splitn(7, ':').collect();
        if fields.len() >= 6 && fields[0] == username {
            return Ok(PathBuf::from(fields[5]));
        }
    }

    // Fallback: trust HOME env var
    if let Ok(home) = env::var("HOME") {
        return Ok(PathBuf::from(home));
    }

    Err(format!(
        "Cannot locate home directory for user '{username}'"
    ))
}

/// Determine the current login name without requiring external crates.
fn resolve_login_name() -> String {
    if let Ok(u) = env::var("USER") {
        return u;
    }
    if let Ok(u) = env::var("LOGNAME") {
        return u;
    }
    // Last resort: look up by effective UID
    let uid = get_uid();
    get_username_for_uid(uid).unwrap_or_else(|| "nobody".to_string())
}

fn get_uid() -> u32 {
    unsafe extern "C" {
        fn getuid() -> u32;
    }
    unsafe { getuid() }
}

fn get_username_for_uid(uid: u32) -> Option<String> {
    let passwd = fs::read_to_string("/etc/passwd").ok()?;
    for line in passwd.lines() {
        let fields: Vec<&str> = line.splitn(7, ':').collect();
        if fields.len() >= 4 && fields[2].parse::<u32>().ok()? == uid {
            return Some(fields[0].to_string());
        }
    }
    None
}

/// Write the skeleton config to `cfg_file`, substituting `@HOME@`.
fn create_initial_config(cfg_file: &Path, home_dir: &Path) -> Result<(), String> {
    let skel_path = Path::new(SKEL);
    if !skel_path.exists() {
        return Err(format!("{SKEL} is missing, please reinstall this package."));
    }
    let skel_content =
        fs::read_to_string(skel_path).map_err(|e| format!("Cannot read skeleton config: {e}"))?;
    let content = skel_content.replace("@HOME@", &home_dir.to_string_lossy());
    fs::write(cfg_file, content)
        .map_err(|e| format!("Cannot write config file {}: {e}", cfg_file.display()))?;
    Ok(())
}

/// Parse a simple shell-style config file supporting:
/// - `DBPATH=/path`  or  `DBPATH="/path"`
/// - `IGNORE=(mod1 mod2 mod3)`
///
/// Lines beginning with `#`, blank lines, and unrecognized keys are ignored.
fn parse_config(cfg_file: &Path) -> Result<Config, String> {
    let content = fs::read_to_string(cfg_file)
        .map_err(|e| format!("Cannot read config file {}: {e}", cfg_file.display()))?;

    let mut db_path_str: Option<String> = None;
    let mut ignore: Vec<String> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(rest) = line.strip_prefix("DBPATH=") {
            db_path_str = Some(unquote(rest).to_string());
        } else if let Some(rest) = line.strip_prefix("IGNORE=") {
            ignore = parse_array(rest);
        }
        // Unrecognized keys (e.g. a legacy COLORS setting) are ignored.
    }

    let dbpath = match db_path_str {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => {
            return Err(format!("DBPATH is not set in {}", cfg_file.display()));
        }
    };

    if !dbpath.exists() {
        fs::create_dir_all(&dbpath)
            .map_err(|e| format!("Cannot create DBPATH {}: {e}", dbpath.display()))?;
    }

    let db_path = dbpath.join("modpdb.db");

    Ok(Config { db_path, ignore })
}

/// Strip surrounding single or double quotes from a value string.
fn unquote(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Parse a bash-style array literal such as `(item1 item2 item3)`.
fn parse_array(s: &str) -> Vec<String> {
    let s = s.trim();
    let inner = if s.starts_with('(') && s.ends_with(')') {
        &s[1..s.len() - 1]
    } else {
        s
    };
    inner
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| unquote(t).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_unquote_double_quotes() {
        assert_eq!(unquote("\"hello\""), "hello");
    }

    #[test]
    fn test_unquote_single_quotes() {
        assert_eq!(unquote("'hello'"), "hello");
    }

    #[test]
    fn test_unquote_no_quotes() {
        assert_eq!(unquote("/home/user/.config"), "/home/user/.config");
    }

    #[test]
    fn test_parse_array_basic() {
        let v = parse_array("(nvidia nvidia_drm vboxdrv)");
        assert_eq!(v, vec!["nvidia", "nvidia_drm", "vboxdrv"]);
    }

    #[test]
    fn test_parse_array_empty() {
        assert!(parse_array("()").is_empty());
    }

    #[test]
    fn test_parse_array_no_parens() {
        let v = parse_array("nvidia vboxdrv");
        assert_eq!(v, vec!["nvidia", "vboxdrv"]);
    }

    #[test]
    fn test_parse_config_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let dbpath = dir.path().join("config");
        std::fs::create_dir_all(&dbpath).unwrap();

        let cfg_file = dir.path().join("modpdb.conf");
        let mut f = std::fs::File::create(&cfg_file).unwrap();
        writeln!(f, "# comment").unwrap();
        writeln!(f, "DBPATH=\"{}\"", dbpath.display()).unwrap();
        writeln!(f, "COLORS=dark").unwrap();
        writeln!(f, "IGNORE=(nvidia vboxdrv)").unwrap();

        let cfg = parse_config(&cfg_file).unwrap();
        assert_eq!(cfg.db_path, dbpath.join("modpdb.db"));
        assert_eq!(cfg.ignore, vec!["nvidia", "vboxdrv"]);
    }

    #[test]
    fn test_parse_config_missing_dbpath() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_file = dir.path().join("modpdb.conf");
        let mut f = std::fs::File::create(&cfg_file).unwrap();
        writeln!(f, "COLORS=dark").unwrap();

        assert!(parse_config(&cfg_file).is_err());
    }
}
