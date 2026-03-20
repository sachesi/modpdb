Name:           modpdb
Version:        1.0.0
Release:        1%{?dist}
Summary:        Store every unique kernel module ever probed on the system

License:        MIT
URL:            https://github.com/sachesi/modpdb
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust >= 1.74

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
%autosetup

%build
cargo build --release

%install
install -Dm755 target/release/%{name} %{buildroot}%{_bindir}/%{name}

# Skeleton config
install -Dm644 share/%{name}.skel \
    %{buildroot}%{_datadir}/%{name}/%{name}.skel

# Man page
install -Dm644 doc/%{name}.8 \
    %{buildroot}%{_mandir}/man8/%{name}.8

# Systemd user units
install -Dm644 init/%{name}.service \
    %{buildroot}%{_userunitdir}/%{name}.service
install -Dm644 init/%{name}.timer \
    %{buildroot}%{_userunitdir}/%{name}.timer

# Shell completions
install -Dm644 completions/bash-completion \
    %{buildroot}%{_datadir}/bash-completion/completions/%{name}
install -Dm644 completions/zsh-completion \
    %{buildroot}%{_datadir}/zsh/site-functions/_%{name}

%post
# Inform the user about the service
echo "To enable the modpdb timer for your user, run:"
echo "  systemctl --user enable --now modpdb.timer"

%files
%license LICENSE
%doc README.md
%{_bindir}/%{name}
%{_datadir}/%{name}/
%{_mandir}/man8/%{name}.8*
%{_userunitdir}/%{name}.service
%{_userunitdir}/%{name}.timer
%{_datadir}/bash-completion/completions/%{name}
%{_datadir}/zsh/site-functions/_%{name}

%changelog
* %(date "+%a %b %d %Y") modpdb packager <modpdb@sachesi> - 1.0.0-1
- Initial RPM packaging for Fedora
