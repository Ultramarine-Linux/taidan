Name:           taidan-server-oobe
Version:        0.2.7
Release:        1%{?dist}
Summary:        Ultramarine Server web first-run experience
License:        GPL-3.0-or-later
URL:            https://github.com/Ultramarine-Linux/taidan
Source0:        %{url}/archive/refs/tags/v%{version}.tar.gz

Requires:       nodejs
Requires:       tetra
BuildRequires:  nodejs
BuildRequires:  pnpm
BuildRequires:  cargo-rpm-macros

%description
Web-based first-run experience for Ultramarine Server.

%prep
%autosetup -n taidan-%{version}
%cargo_prep_online

%build
%__pnpm --dir server-oobe install --frozen-lockfile
%__pnpm --dir server-oobe build
%cargo_build -p tdnx --no-default-features --features server-oobe
%cargo_license_summary_online
%{cargo_license_online} > LICENSE.dependencies

%install
mkdir -p %{buildroot}%{_datadir}/ultramarine-server-oobe
cp -a server-oobe/build/* %{buildroot}%{_datadir}/ultramarine-server-oobe/
mkdir -p %{buildroot}%{_libexecdir}/ultramarine-server-oobe
install -Dm0755 target/rpm/tdnx %{buildroot}%{_libexecdir}/ultramarine-server-oobe/tdnx
mkdir -p %{buildroot}%{_unitdir}
install -m 0644 server-oobe/ultramarine-server-oobe.service %{buildroot}%{_unitdir}/

%files
%license LICENSE.md
%license LICENSE.dependencies
%doc README.md
%{_datadir}/ultramarine-server-oobe/
%{_libexecdir}/ultramarine-server-oobe/tdnx
%{_unitdir}/ultramarine-server-oobe.service

%changelog
* Tue Sep 23 2026 Cypress Reed <cypress@fyralabs.com>
- initial package
