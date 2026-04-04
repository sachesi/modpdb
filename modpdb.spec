%define _debugsource_template %{nil}
%define debug_package %{nil}
%global modpdb_userunitdir %{_prefix}/lib/systemd/user
Name:           modpdb
Version:        0.1.0
Release:        2%{?dist}
Summary:        Store every unique kernel module ever probed on the system
License:        MIT
URL:            https://github.com/sachesi/modpdb
%if ! 0%{?_build_in_place}
Source0:        %{url}/archive/refs/tags/%{version}/%{name}-%{version}.tar.gz
%endif
BuildRequires:  cargo
BuildRequires:  cargo-rpm-macros
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
%if 0%{?_build_in_place}
# Build directly from the current checkout when rpmbuild is called with --build-in-place.
%else
%autosetup -n %{name}-%{version}
%endif

%generate_buildrequires
%if ! 0%{?_build_in_place}
%cargo_generate_buildrequires
%endif

%build
%if 0%{?_build_in_place}
cargo build --release
%else
%cargo_build --release
%endif

%install
%if 0%{?_build_in_place}
install -Dm755 target/release/%{name} %{buildroot}%{_bindir}/%{name}
%else
%cargo_install
%endif
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

%post
echo "To enable the modpdb timer for your user, run:"
echo "  systemctl --user enable --now modpdb.timer"

%files
%license LICENSE
%doc README.md
%{_bindir}/%{name}
%{_datadir}/%{name}/
%{_mandir}/man8/%{name}.8*
%{modpdb_userunitdir}/%{name}.service
%{modpdb_userunitdir}/%{name}.timer
%{_datadir}/bash-completion/completions/%{name}
%{_datadir}/zsh/site-functions/_%{name}
%{_datadir}/fish/vendor_completions.d/%{name}.fish

%changelog
* Sat Apr 04 2026 modpdb packager <modpdb@sachesi> - 0.1.0-2
- Make spec COPR-friendly with remote Source0 and cargo-rpm macros
- Keep --build-in-place workflow using direct cargo build/install

* Fri Mar 20 2026 modpdb packager <modpdb@sachesi> - 0.1.0-1
- Initial RPM packaging for Fedora
