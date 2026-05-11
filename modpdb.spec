%define _debugsource_template %{nil}
%define debug_package %{nil}
%global modpdb_userunitdir %{_prefix}/lib/systemd/user

Name:           modpdb
Version:        0.1.1
Release:        1%{?dist}
Summary:        Store every unique kernel module ever probed on the system
License:        GPL-3.0-or-later
URL:            https://github.com/sachesi/modpdb
Source0:        %{url}/archive/refs/tags/v%{version}.tar.gz#/%{name}-%{version}.tar.gz
Source1:        %{name}-%{version}-vendor.tar.zst

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc
Requires:       kmod

%description
modpdb is a utility for users who want to build a minimal kernel via
"make localmodconfig". It logs every kernel module ever probed on the system
to a plain-text database file. This database can be passed directly to
"make localmodconfig" so that only the modules your system has actually needed
are compiled in, significantly reducing kernel build time and the resulting
kernel footprint.

The database is stored at $DBPATH/modpdb.db (default: ~/.config/modpdb.db).

%prep
%autosetup -n %{name}-%{version}
tar -xaf %{SOURCE1}

mkdir -p .cargo
cat > .cargo/config.toml <<'EOF'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

%build
export CARGO_HOME=$PWD/.cargo-home
cargo build --release --frozen --offline

%install
install -Dm755 target/release/%{name} \
    %{buildroot}%{_bindir}/%{name}

install -Dm644 share/%{name}.skel \
    %{buildroot}%{_datadir}/%{name}/%{name}.skel
install -Dm644 doc/%{name}.8 \
    %{buildroot}%{_mandir}/man8/%{name}.8
install -Dm644 init/%{name}.service \
    %{buildroot}%{modpdb_userunitdir}/%{name}.service
install -Dm644 init/%{name}.timer \
    %{buildroot}%{modpdb_userunitdir}/%{name}.timer
install -Dm644 completions/bash-completion \
    %{buildroot}%{_datadir}/bash-completion/completions/%{name}
install -Dm644 completions/zsh-completion \
    %{buildroot}%{_datadir}/zsh/site-functions/_%{name}
install -Dm644 completions/fish-completion \
    %{buildroot}%{_datadir}/fish/vendor_completions.d/%{name}.fish

%files
%license LICENSE
%doc README.md
%{_bindir}/%{name}
%{_datadir}/%{name}/%{name}.skel
%{_mandir}/man8/%{name}.8*
%{modpdb_userunitdir}/%{name}.service
%{modpdb_userunitdir}/%{name}.timer
%{_datadir}/bash-completion/completions/%{name}
%{_datadir}/zsh/site-functions/_%{name}
%{_datadir}/fish/vendor_completions.d/%{name}.fish

%changelog
* Thu Apr 23 2026 sachesi <xsachesi@pm.me> - 0.1.1-3
- Switch to vendored offline COPR build
- Change license to GPL-3.0

* Sat Apr 04 2026 sachesi <xsachesi@pm.me> - 0.1.0-2
- Make spec COPR-friendly with remote Source0 and cargo-rpm macros
- Keep --build-in-place workflow using direct cargo build/install

* Fri Mar 20 2026 sachesi <xsachesi@pm.me> - 0.1.0-1
- Initial RPM packaging for Fedora
