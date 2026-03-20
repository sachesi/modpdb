mod config;
mod db;
mod modules;

use std::process;

use clap::{Parser, Subcommand};

use config::Config;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(
    name = "modpdb",
    version = VERSION,
    about = "Store every unique kernel module ever probed on the system",
    long_about = None,
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show all modules currently in the database
    List,
    /// Store any new module(s) to the database
    Store,
    /// Store any new module(s) to the database (less verbose, for systemd timer)
    Storesilent,
    /// Diff loaded modules against the database
    Debug,
    /// Modprobe to load all modules in the database (must be root)
    Recall,
    /// Modprobe to refresh and rebuild the database (must be root)
    Rebuild,
}

fn main() {
    let cli = Cli::parse();

    let cfg = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("==> ERROR: {e}");
            process::exit(1);
        }
    };

    let result = match &cli.command {
        Some(Commands::List) => commands::list(&cfg),
        Some(Commands::Store) => commands::store(&cfg, false),
        Some(Commands::Storesilent) => commands::store(&cfg, true),
        Some(Commands::Debug) => commands::debug(&cfg),
        Some(Commands::Recall) => commands::recall(&cfg),
        Some(Commands::Rebuild) => commands::rebuild(&cfg),
        None => commands::default_view(&cfg),
    };

    if let Err(e) = result {
        eprintln!("==> ERROR: {e}");
        process::exit(1);
    }
}

mod commands {
    use std::collections::BTreeSet;
    use std::fs;
    use std::io::{self, Write};
    use std::process::Command;

    use crate::config::Config;
    use crate::db;
    use crate::modules;

    pub fn list(cfg: &Config) -> Result<(), String> {
        let db = db::read(&cfg.db_path)?;
        let stdout = io::stdout();
        let mut out = stdout.lock();
        for module in &db {
            writeln!(out, "{module}").map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn store(cfg: &Config, silent: bool) -> Result<(), String> {
        if !silent {
            announce(cfg)?;
        }

        if !cfg.db_path.exists() {
            let in_mem = modules::loaded_modules(&cfg.ignore)?;
            db::write(&cfg.db_path, &in_mem)?;
            if !silent {
                println!(
                    "\x1b[1mNew database created: \x1b[01;33m{}\x1b[00m",
                    cfg.db_path.display()
                );
                println!();
                println!(
                    "\x1b[1m{} modules are now saved in \x1b[01;33m{}\x1b[00m",
                    in_mem.len(),
                    cfg.db_path.display()
                );
            } else {
                println!(
                    "{} modules are now saved in {}",
                    in_mem.len(),
                    cfg.db_path.display()
                );
            }
            return Ok(());
        }

        let existing = db::read(&cfg.db_path)?;
        let in_mem = modules::loaded_modules(&cfg.ignore)?;

        let mut merged: BTreeSet<String> = existing.clone();
        merged.extend(in_mem.iter().cloned());

        if merged == existing {
            if !silent {
                println!("\x1b[1mNo new modules detected. Taking no action.\x1b[00m");
            } else {
                println!("No new modules detected");
            }
        } else {
            if !silent {
                let new_mods: BTreeSet<&String> = merged.difference(&existing).collect();
                println!("\x1b[01;33mNew module(s) detected:\x1b[00m");
                for m in &new_mods {
                    println!("\x1b[1m{m}\x1b[00m");
                }
            }
            db::write(&cfg.db_path, &merged)?;
            if !silent {
                println!();
                println!(
                    "\x1b[1m{} modules are now saved in \x1b[01;33m{}\x1b[00m",
                    merged.len(),
                    cfg.db_path.display()
                );
            } else {
                println!(
                    "{} modules are now saved in {}",
                    merged.len(),
                    cfg.db_path.display()
                );
            }
        }
        Ok(())
    }

    pub fn debug(cfg: &Config) -> Result<(), String> {
        announce(cfg)?;
        let db = db::read(&cfg.db_path)?;
        let in_mem = modules::loaded_modules(&cfg.ignore)?;

        println!("\x1b[1mThe following are in the database but not loaded:\x1b[00m");
        for m in db.difference(&in_mem) {
            println!("{m}");
        }
        println!();
        println!("\x1b[1mThe following are loaded but not in the database:\x1b[00m");
        for m in in_mem.difference(&db) {
            println!("{m}");
        }
        Ok(())
    }

    pub fn recall(cfg: &Config) -> Result<(), String> {
        require_root()?;
        announce(cfg)?;
        let db = db::read(&cfg.db_path)?;
        let modules_list: Vec<&str> = db.iter().map(String::as_str).collect();

        println!(
            "\x1b[1mAttempting to modprobe all modules from \x1b[01;33m{}\x1b[00m",
            cfg.db_path.display()
        );

        if !modules_list.is_empty() {
            let status = Command::new("modprobe")
                .arg("-a")
                .args(&modules_list)
                .status()
                .map_err(|e| format!("Failed to run modprobe: {e}"))?;
            if !status.success() {
                eprintln!("Warning: modprobe exited with a non-zero status");
            }
        }

        let loaded = modules::loaded_modules(&cfg.ignore)?;
        println!();
        println!(
            "\x1b[01;31m{}\x1b[00m\x1b[1m modules are now loaded per \x1b[01;33m/proc/modules\x1b[00m",
            loaded.len()
        );
        Ok(())
    }

    pub fn rebuild(cfg: &Config) -> Result<(), String> {
        require_root()?;

        let db = db::read(&cfg.db_path)?;
        let modules_list: Vec<&str> = db.iter().map(String::as_str).collect();

        println!(
            "\x1b[1mRefreshing the contents of \x1b[01;33m{}\x1b[00m",
            cfg.db_path.display()
        );

        // Attempt to load all modules (suppress errors — some may not exist)
        if !modules_list.is_empty() {
            let _ = Command::new("modprobe")
                .arg("-a")
                .args(&modules_list)
                .output();
        }

        // Back up the old database with a timestamp suffix
        let backup_path = {
            let timestamp = current_timestamp();
            let mut p = cfg.db_path.clone();
            let name = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            p.set_file_name(format!("{name}.{timestamp}"));
            p
        };
        fs::copy(&cfg.db_path, &backup_path)
            .map_err(|e| format!("Failed to backup database: {e}"))?;
        println!(
            "\x1b[1mOld database saved to \x1b[01;33m{}\x1b[00m",
            backup_path.display()
        );

        // Rebuild from currently loaded modules only
        let in_mem = modules::loaded_modules(&cfg.ignore)?;
        db::write(&cfg.db_path, &in_mem)?;

        println!();
        println!(
            "\x1b[1m{} modules are now saved in \x1b[01;33m{}\x1b[00m",
            in_mem.len(),
            cfg.db_path.display()
        );
        Ok(())
    }

    pub fn default_view(cfg: &Config) -> Result<(), String> {
        announce(cfg)?;
        println!("\x1b[1mmodpdb\x1b[00m \x1b[01;32m[option]\x1b[00m");
        println!("   \x1b[01;32mlist\x1b[00m\x1b[1m\t\tShow all modules currently in the database.\x1b[00m");
        println!("   \x1b[01;32mstore\x1b[00m\x1b[1m\t\tStore any new module(s) to the database.\x1b[00m");
        println!("   \x1b[01;32mstoresilent\x1b[00m\x1b[1m\tStore any new module(s) to the database more quietly.\x1b[00m");
        println!("   \x1b[01;32mdebug\x1b[00m\x1b[1m\t\tDiff loaded modules from the database.\x1b[00m");
        println!("   \x1b[01;32mrecall\x1b[00m\x1b[1m\tModprobe every module in the database.  \x1b[00m\x1b[01;31mMUST be called as root!\x1b[00m");
        println!("   \x1b[01;32mrebuild\x1b[00m\x1b[1m\tRefresh and rebuild the database.       \x1b[00m\x1b[01;31mMUST be called as root!\x1b[00m");
        println!();
        println!("\x1b[1mSee manpage for additional details\x1b[00m");
        Ok(())
    }

    fn announce(cfg: &Config) -> Result<(), String> {
        println!("\x1b[01;31mmodpdb v{}\x1b[00m", crate::VERSION);
        println!();

        let in_mem = modules::loaded_modules(&cfg.ignore)?;
        let db_size = if cfg.db_path.exists() {
            db::read(&cfg.db_path)?.len()
        } else {
            0
        };

        println!(
            "\x1b[1m{} modules currently loaded per \x1b[01;33m/proc/modules\x1b[00m",
            in_mem.len()
        );
        println!(
            "\x1b[1m{db_size} modules are in \x1b[01;33m{}\x1b[00m",
            cfg.db_path.display()
        );
        println!();
        Ok(())
    }

    fn require_root() -> Result<(), String> {
        if get_uid() != 0 {
            return Err("This function must be called as root!".to_string());
        }
        Ok(())
    }

    fn get_uid() -> u32 {
        unsafe extern "C" {
            fn getuid() -> u32;
        }
        unsafe { getuid() }
    }

    fn current_timestamp() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format_utc(secs)
    }

    fn format_utc(secs: u64) -> String {
        let s = secs % 60;
        let m = (secs / 60) % 60;
        let h = (secs / 3600) % 24;
        let days = secs / 86400;
        let (year, month, day) = days_to_ymd(days);
        format!("{year:04}{month:02}{day:02}_{h:02}{m:02}{s:02}")
    }

    /// Convert days since Unix epoch to (year, month, day) using the
    /// civil-from-days algorithm.
    fn days_to_ymd(days: u64) -> (u64, u64, u64) {
        let z = days + 719468;
        let era = z / 146097;
        let doe = z % 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let mo = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if mo <= 2 { y + 1 } else { y };
        (y, mo, d)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_format_utc_known() {
            // 2020-01-01 00:00:00 UTC = 1577836800 seconds
            assert_eq!(format_utc(1_577_836_800), "20200101_000000");
        }

        #[test]
        fn test_format_utc_another() {
            // 2023-06-15 12:30:45 UTC
            assert_eq!(format_utc(1_686_832_245), "20230615_123045");
        }
    }
}
