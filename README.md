# modpdb

**modpdb** is a utility for Linux users who want to build a minimal kernel using
`make localmodconfig`. It logs every kernel module ever probed on the system to a
plain-text database. This database can then be passed directly to
`make localmodconfig` so that only the modules your system has actually ever
needed are compiled in — dramatically reducing kernel build time and footprint.

## How it works

`modpdb` reads the currently loaded modules from `/proc/modules` and keeps a
cumulative log of every unique module name ever seen. The database lives at
`$DBPATH/modpdb.db` (default: `~/.config/modpdb.db`).

When building a kernel, pass the database to make:

```sh
make LSMOD=$HOME/.config/modpdb.db localmodconfig
```

## Installation

### From source (Rust / Cargo)

```sh
cargo build --release
sudo install -Dm755 target/release/modpdb /usr/bin/modpdb
sudo install -Dm644 share/modpdb.skel /usr/share/modpdb/modpdb.skel
sudo install -Dm644 doc/modpdb.8 /usr/share/man/man8/modpdb.8
sudo install -Dm644 init/modpdb.service /usr/lib/systemd/user/modpdb.service
sudo install -Dm644 init/modpdb.timer   /usr/lib/systemd/user/modpdb.timer
sudo install -Dm644 completions/bash-completion \
    /usr/share/bash-completion/completions/modpdb
sudo install -Dm644 completions/zsh-completion \
    /usr/share/zsh/site-functions/_modpdb
```

### Fedora RPM

A `.spec` file is provided to build an RPM package:

```sh
# Pull sources automatically (from crates.io, so it also works if GitHub is private)
spectool -g -R modpdb.spec

# Build the RPM
rpmbuild -ba modpdb.spec
```

## First run & configuration

On the first run, `modpdb` creates a configuration file at
`~/.config/modpdb.conf`. Edit it before using the tool:

- **`DBPATH`** — absolute path to the directory where `modpdb.db` will be stored
- **`COLORS`** — `dark` or `light` depending on your terminal background
- **`IGNORE`** — modules to exclude (e.g. out-of-tree / proprietary drivers)

## Usage

```
modpdb [command]

  list         Show all modules currently in the database
  store        Store any new modules from the running system into the database
  storesilent  Same as store but quieter (used by the systemd timer)
  debug        Show which modules are in the DB but not loaded, and vice versa
  recall       Load all modules in the database via modprobe  [needs root]
  rebuild      Reload all modules then rebuild the database from scratch  [needs root]
```

## Automation with systemd

Enable the included user timer to run `modpdb storesilent` at boot and every
six hours:

```sh
systemctl --user enable --now modpdb.timer
```

Check status:

```sh
systemctl --user status modpdb
systemctl --user list-timers
```

## Tips

- Boot a default/distribution kernel periodically and run `modpdb store` after
  major kernel updates to capture any new modules that have become available.
- Run `sudo modpdb rebuild` occasionally to drop stale entries that can no
  longer be loaded.
- The database is plain text (one module per line, alphabetically sorted) — you
  can view and edit it directly with any text editor.

## License

MIT — see [LICENSE](LICENSE).
