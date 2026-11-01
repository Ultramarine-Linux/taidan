Name:           taidan
Version:        git~%{sub %gitcommit 1 7}
Release:        1%{?dist}
Summary:        Out-Of-Box-Experience (OOBE) and Welcome App
SourceLicense:  GPL-3.0-or-later
License:        (0BSD OR MIT OR Apache-2.0) AND Apache-2.0 AND (Apache-2.0 OR BSL-1.0) AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR MIT) AND (Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT) AND MIT AND (MIT OR Apache-2.0) AND (MIT OR Zlib OR Apache-2.0) AND Unicode-3.0 AND (Unlicense OR MIT) AND Zlib AND GPL-3.0-or-later
URL:            https://github.com/Ultramarine-Linux/taidan
%dnl Source0:        %{url}/archive/%{gitcommit}.tar.gz
Packager:       Terra Packaging Team <terra@fyralabs.com>
# distro configuration, e.g. taidan-ultramarine-configs, or the defaults below
Requires:       %{name}-configs
Suggests:       %{name}-default-configs
# the OOBE session: taidan-initial-setup.service and start-taidan
# provides kwin_wayland, Fedora has no separate kwin-wayland package anymore
Requires:       kwin
Requires:       dbus-daemon
Requires:       swaybg
Requires:       polkit
# installation steps
Requires:       (glib2 or (/usr/bin/plasma-apply-colorscheme and kf6-kconfig))
Requires:       shadow-utils
Requires:       systemd
Requires:       systemd-udev
Requires:       bash
Requires:       (dnf5 and dnf5-command(copr))
Requires:       flatpak
# required for running homectl
Requires:       util-linux-script
# network page, over its D-Bus API, and the browser for captive portals
Requires:       NetworkManager
Requires:       cog
# time zone page: /usr/share/zoneinfo/zone1970.tab and the zones themselves
Requires:       tzdata
# keeps the clock right, `timedatectl set-ntp`
Recommends:     chrony
# finds the kernel the NVIDIA driver is built for
Recommends:     grubby
Requires:       xdg-terminal-exec
Requires:       vercel-geist-fonts
# accessibility: the legible font option, the screen reader, and applying the rest to KWin
Requires:       atkinson-hyperlegible-next-fonts
Requires:       atkinson-hyperlegible-mono-fonts
Requires:       orca
Requires:       /usr/bin/kwriteconfig6
# dlopen()ed by Slint, so not picked up as automatic dependencies
Requires:       libxkbcommon
Requires:       libwayland-client
Requires:       libwayland-cursor
Requires:       libwayland-egl
Requires:       libglvnd-egl
BuildRequires:  anda-srpm-macros mold cargo rust-packaging perl systemd-rpm-macros
BuildRequires:  pkgconfig(openssl)
BuildRequires:  pkgconfig(fontconfig)
BuildRequires:  pkgconfig(libxcrypt)
# taidan_proc_macros: language names, locales and keyboard layouts are read at build time
BuildRequires:  pkgconfig(icu-i18n)
BuildRequires:  glibc-all-langpacks
BuildRequires:  xkeyboard-config
BuildRequires:  clang-libs
BuildRequires:  clang-devel

%description
Taidan is a GUI Out-Of-Box-Experience (OOBE) and Welcome App for Ultramarine
Linux, written in Rust and the Slint toolkit.

%package default-configs
Summary:        Default configurations for Taidan
Provides:       %{name}-configs
BuildArch:      noarch

%description default-configs
This package contains the default configuration files for Taidan.

%prep
%dnl %autosetup -n taidan-%gitcommit
%git_clone %url %gitcommit
%cargo_prep_online

%build
%{cargo_license_online} > LICENSE.dependencies

%install
%cargo_install
DESTDIR=%buildroot ./scripts/install.sh

%files
%doc README.md
%license LICENSE.md LICENSE.dependencies
%{_bindir}/taidan
%{_libexecdir}/start-taidan
%{_datadir}/polkit-1/rules.d/100-taidan.rules
%{_presetdir}/95-taidan.preset
%{_sysconfdir}/pam.d/taidan
%{_sysusersdir}/taidan.conf
%{_unitdir}/taidan-initial-setup.service
%{_unitdir}/taidan-initial-setup-reconfiguration.service
%dir %{_prefix}/lib/taidan/
%dir %{_datadir}/taidan/
%{_datadir}/taidan/avatars/

%files default-configs
%{_datadir}/taidan/tweaks/
