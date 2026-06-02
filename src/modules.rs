use std::collections::BTreeSet;
use std::fs;

/// Read currently loaded kernel modules from `/proc/modules` and return a
/// sorted, deduplicated set of module names, excluding any names present in
/// the `ignore` list.
pub fn loaded_modules(ignore: &[String]) -> Result<BTreeSet<String>, String> {
    let content = fs::read_to_string("/proc/modules")
        .map_err(|e| format!("Cannot read /proc/modules: {e}"))?;

    let ignore_set: BTreeSet<&str> = ignore.iter().map(String::as_str).collect();

    // Each line in /proc/modules: <name> <size> <refcount> <deps> <state> <offset>
    let modules = content
        .lines()
        .filter_map(|line| line.split_whitespace().next().map(str::to_string))
        .filter(|name| is_valid_module_name(name))
        .filter(|name| !ignore_set.contains(name.as_str()))
        .collect();

    Ok(modules)
}

/// Check if a string is a valid kernel module name to prevent flag injection.
pub fn is_valid_module_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_names(proc_content: &str, ignore: &[String]) -> BTreeSet<String> {
        let ignore_set: BTreeSet<&str> = ignore.iter().map(String::as_str).collect();
        proc_content
            .lines()
            .filter_map(|line| line.split_whitespace().next().map(str::to_string))
            .filter(|name| is_valid_module_name(name))
            .filter(|name| !ignore_set.contains(name.as_str()))
            .collect()
    }

    #[test]
    fn test_parse_proc_modules_basic() {
        let content = "ext4 921600 1 - Live 0xffffffffc0a00000\n";
        let result = parse_names(content, &[]);
        assert!(result.contains("ext4"));
    }

    #[test]
    fn test_ignore_filters_out_modules() {
        let content = "nvidia 123 0 - Live 0x0\next4 456 1 - Live 0x1\nvboxdrv 789 0 - Live 0x2\n";
        let ignore = vec!["nvidia".to_string(), "vboxdrv".to_string()];
        let result = parse_names(content, &ignore);

        assert!(result.contains("ext4"));
        assert!(!result.contains("nvidia"));
        assert!(!result.contains("vboxdrv"));
    }

    #[test]
    fn test_result_is_sorted() {
        let content = "zzz 1 0 - Live 0x0\naaa 1 0 - Live 0x1\nmmm 1 0 - Live 0x2\n";
        let result = parse_names(content, &[]);
        let sorted: Vec<&String> = result.iter().collect();
        assert_eq!(sorted, vec!["aaa", "mmm", "zzz"]);
    }

    #[test]
    fn test_is_valid_module_name() {
        assert!(is_valid_module_name("ext4"));
        assert!(is_valid_module_name("nvidia_drm"));
        assert!(is_valid_module_name("xhci-hcd"));
        assert!(!is_valid_module_name(""));
        assert!(!is_valid_module_name("-a"));
        assert!(!is_valid_module_name("module;rm -rf /"));
        assert!(!is_valid_module_name("module name"));
    }
}
