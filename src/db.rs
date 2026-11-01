use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// Read the database file and return a sorted, deduplicated set of module names.
/// Returns an empty set if the file does not yet exist.
pub fn read(db_path: &Path) -> Result<BTreeSet<String>, String> {
    let content = match read_without_links(db_path) {
        Ok(content) => content,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(e) => return Err(format!("Cannot read database {}: {e}", db_path.display())),
    };
    let content = String::from_utf8(content)
        .map_err(|e| format!("Cannot read database {}: {e}", db_path.display()))?;
    Ok(content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter(|l| crate::modules::is_valid_module_name(l))
        .map(str::to_string)
        .collect())
}

/// Write a sorted, deduplicated set of module names to the database file,
/// one entry per line.  Creates parent directories as needed.
pub fn write(db_path: &Path, modules: &BTreeSet<String>) -> Result<(), String> {
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create directory {}: {e}", parent.display()))?;
        }
    }

    let mut content = modules
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if !content.is_empty() {
        content.push('\n');
    }

    // Atomic write: write to a pid-suffixed temp file first, then rename. The
    // pid suffix keeps concurrent writers (e.g. the systemd timer firing during
    // a manual store) from colliding on the same temp path.
    let temp_path = db_path.with_extension(format!("tmp.{}", std::process::id()));
    create_new_file(&temp_path, content.as_bytes())
        .map_err(|e| format!("Cannot write to temporary database {}: {e}", temp_path.display()))?;

    fs::rename(&temp_path, db_path)
        .map_err(|e| format!("Cannot rename {} to {}: {e}", temp_path.display(), db_path.display()))?;

    Ok(())
}

/// Copy the database to `backup_path`, which must not exist yet.
pub fn backup(db_path: &Path, backup_path: &Path) -> io::Result<()> {
    create_new_file(backup_path, &read_without_links(db_path)?)
}

/// Create `path` with `content`. This fails on anything already at the path, a symbolic
/// link included: under sudo these paths lie in a folder the user owns, and root must not
/// be led to write through a link planted there.
pub fn create_new_file(path: &Path, content: &[u8]) -> io::Result<()> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(content)
}

/// Read `path` unless it is a symbolic link. The link is looked for after opening, on the
/// same file that was opened, so swapping one in between the two steps does not help.
fn read_without_links(path: &Path) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let opened = file.metadata()?;
    let named = fs::symlink_metadata(path)?;
    if !named.is_file() || (named.dev(), named.ino()) != (opened.dev(), opened.ino()) {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "not a regular file, or it changed while being opened",
        ));
    }
    let mut content = Vec::new();
    file.read_to_end(&mut content)?;
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_write_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("modpdb.db");

        let mut modules = BTreeSet::new();
        modules.insert("ext4".to_string());
        modules.insert("ahci".to_string());
        modules.insert("xhci_hcd".to_string());

        write(&db_path, &modules).unwrap();
        let read_back = read(&db_path).unwrap();
        assert_eq!(modules, read_back);
    }

    #[test]
    fn test_read_nonexistent_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("nonexistent.db");
        assert!(read(&db_path).unwrap().is_empty());
    }

    #[test]
    fn test_write_is_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("modpdb.db");

        let mut modules = BTreeSet::new();
        modules.insert("zzz".to_string());
        modules.insert("aaa".to_string());
        modules.insert("mmm".to_string());

        write(&db_path, &modules).unwrap();
        let content = std::fs::read_to_string(&db_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines, vec!["aaa", "mmm", "zzz"]);
    }

    #[test]
    fn test_write_empty_db() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("modpdb.db");

        write(&db_path, &BTreeSet::new()).unwrap();
        assert_eq!(std::fs::read_to_string(&db_path).unwrap(), "");
    }

    #[test]
    fn test_read_ignores_blank_lines() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("modpdb.db");
        std::fs::write(&db_path, "ext4\n\nahci\n\n").unwrap();

        let modules = read(&db_path).unwrap();
        assert_eq!(modules.len(), 2);
        assert!(modules.contains("ext4"));
        assert!(modules.contains("ahci"));
    }

    #[test]
    fn test_write_refuses_link_at_temp_path() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("modpdb.db");
        let victim = dir.path().join("victim");
        std::fs::write(&victim, "untouched").unwrap();
        let temp_path = db_path.with_extension(format!("tmp.{}", std::process::id()));
        std::os::unix::fs::symlink(&victim, &temp_path).unwrap();

        assert!(write(&db_path, &BTreeSet::from(["ext4".to_string()])).is_err());
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "untouched");
    }

    #[test]
    fn test_backup_refuses_link_at_backup_path() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("modpdb.db");
        std::fs::write(&db_path, "ext4\n").unwrap();
        let victim = dir.path().join("victim");
        std::fs::write(&victim, "untouched").unwrap();
        let backup_path = dir.path().join("modpdb.db.backup");
        std::os::unix::fs::symlink(&victim, &backup_path).unwrap();

        assert!(backup(&db_path, &backup_path).is_err());
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "untouched");
    }

    #[test]
    fn test_backup_copies_the_database() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("modpdb.db");
        std::fs::write(&db_path, "ext4\n").unwrap();
        let backup_path = dir.path().join("modpdb.db.backup");

        backup(&db_path, &backup_path).unwrap();
        assert_eq!(std::fs::read_to_string(&backup_path).unwrap(), "ext4\n");
    }

    #[test]
    fn test_backup_and_read_refuse_a_linked_database() {
        let dir = tempfile::tempdir().unwrap();
        let secret = dir.path().join("secret");
        std::fs::write(&secret, "ext4\n").unwrap();
        let db_path = dir.path().join("modpdb.db");
        std::os::unix::fs::symlink(&secret, &db_path).unwrap();
        let backup_path = dir.path().join("modpdb.db.backup");

        assert!(read(&db_path).is_err());
        assert!(backup(&db_path, &backup_path).is_err());
        assert!(!backup_path.exists());
    }

    #[test]
    fn test_read_treats_a_dangling_link_as_missing() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("modpdb.db");
        std::os::unix::fs::symlink(dir.path().join("gone"), &db_path).unwrap();

        assert!(read(&db_path).unwrap().is_empty());
    }
}
