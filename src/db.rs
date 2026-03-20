use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Read the database file and return a sorted, deduplicated set of module names.
/// Returns an empty set if the file does not yet exist.
pub fn read(db_path: &Path) -> Result<BTreeSet<String>, String> {
    if !db_path.exists() {
        return Ok(BTreeSet::new());
    }
    let content = fs::read_to_string(db_path)
        .map_err(|e| format!("Cannot read database {}: {e}", db_path.display()))?;
    Ok(content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}

/// Write a sorted, deduplicated set of module names to the database file,
/// one entry per line.  Creates parent directories as needed.
pub fn write(db_path: &Path, modules: &BTreeSet<String>) -> Result<(), String> {
    if let Some(parent) = db_path.parent()
        && !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create directory {}: {e}", parent.display()))?;
        }
    let mut content = modules
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if !content.is_empty() {
        content.push('\n');
    }
    fs::write(db_path, content)
        .map_err(|e| format!("Cannot write database {}: {e}", db_path.display()))?;
    Ok(())
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
}
