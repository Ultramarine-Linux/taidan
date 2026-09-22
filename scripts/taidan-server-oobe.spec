Name:           taidan-server-oobe
Version:        0.1.0
Release:        1%?dist
Summary:        Ultramarine Server web first-run experience
License:        GPL-3.0-or-later
URL:            https://github.com/Ultramarine-Linux/taidan

Requires:       nodejs
Requires:       tetra
BuildRequires:  nodejs
BuildRequires:  pnpm

%description
Optional web-based first-run experience for Ultramarine Server. This package
is independent of the desktop Taidan application and is not required there.

%prep
%setup -q -n taidan-%{version}/server-oobe

%build
pnpm install --frozen-lockfile
pnpm build

%install
mkdir -p %{buildroot}%{_datadir}/ultramarine-server-oobe
cp -a build/* %{buildroot}%{_datadir}/ultramarine-server-oobe/
mkdir -p %{buildroot}%{_unitdir}
install -m 0644 ultramarine-server-oobe.service %{buildroot}%{_unitdir}/

%files
%license LICENSE
%doc README.md
%{_datadir}/ultramarine-server-oobe/
%{_unitdir}/ultramarine-server-oobe.service
