Name:           taidan
Version:        git~%{sub %gitcommit 1 7}
Release:        1%{?dist}
Summary:        Out-Of-Box-Experience (OOBE) and Welcome App
SourceLicense:  GPL-3.0-or-later
License:        %{sourcelicense} AND (BSD-3-Clause OR MIT OR Apache-2.0) AND Apache-2.0 AND MIT AND (MIT OR Apache-2.0 OR Zlib) AND (0BSD OR MIT OR Apache-2.0) AND BSD-2-Clause AND Zlib AND MIT AND (Apache-2.0 OR GPL-2.0-only) AND ((MIT OR Apache-2.0) AND Unicode-3.0) AND (Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT) AND GPL-3.0-or-later AND Apache-2.0 AND Unicode-3.0 AND (BSD-2-Clause OR Apache-2.0 OR MIT) AND (CC0-1.0 OR Apache-2.0) AND (BSD-3-Clause OR Apache-2.0) AND BSL-1.0 AND ISC AND (GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0) AND BSD-3-Clause AND (MIT OR Apache-2.0 OR LGPL-2.1-or-later) AND (Unlicense OR MIT)
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
# polkit reads /run/polkit-1/rules.d, where taidan-initial-setup.service puts its rules, since 126
Requires:       polkit >= 126
# installation steps
Requires:       (glib2 or (/usr/bin/plasma-apply-colorscheme and kf6-kconfig))
Requires:       shadow-utils
# runuser
Requires:       util-linux
Requires:       systemd
Requires:       systemd-udev
# chvt for the unit and start-taidan
Requires:       kbd
Requires:       bash
# config-manager enables the repos asked for in the settings
Requires:       (dnf5 and dnf5-command(copr) and dnf5-command(config-manager))
Requires:       flatpak
# homectl needs systemd-homed running when the distro config sets systemd_homed
Recommends:     systemd-homed
# lspci, to find the NVIDIA GPU's chip
Requires:       pciutils
# dbus-send, to tell KWin to reload the keyboard layout
Requires:       dbus-tools
# the network page talks to NetworkManager over D-Bus, cog is the captive portal browser
Requires:       NetworkManager
Requires:       cog
# setfacl, which lets the captive portal browser's user use the session's display
Requires:       acl
# time zone page: /usr/share/zoneinfo/zone1970.tab and the zones themselves
Requires:       tzdata
# for `timedatectl set-ntp`, which keeps the clock right
Recommends:     chrony
# finds the kernel the NVIDIA driver is built for
Recommends:     grubby
# the opt-in debug shell
Recommends:     xdg-terminal-exec
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
# the tweaks run `um tweaks enable`
Requires:       umcli
BuildArch:      noarch

%description default-configs
This package contains the default configuration files for Taidan.

%prep
%dnl %autosetup -n taidan-%gitcommit
%git_clone %url %gitcommit
%cargo_prep_online

%build
%{cargo_license_online} > LICENSE.dependencies
%cargo_license_summary_online
# the root helper is a binary of libtaidan, which %%cargo_install (the taidan package only) leaves
# out. its dependencies are a subset of taidan's, so the licenses above cover it
%{__cargo} build %{__cargo_common_opts} --profile rpm -p libtaidan --bin taidan-helper

%install
%cargo_install
# the only program the OOBE's polkit rule lets the taidan user run as root, by this exact path
install -Dpm755 target/rpm/taidan-helper %{buildroot}%{_libexecdir}/taidan-helper
DESTDIR=%buildroot ./scripts/install.sh
install -d %buildroot%{_sharedstatedir}/taidan

%post
%systemd_post taidan-initial-setup.service taidan-initial-setup-reconfiguration.service

%preun
%systemd_preun taidan-initial-setup.service taidan-initial-setup-reconfiguration.service

%postun
# no restart on upgrade, that would kill or rerun the OOBE
%systemd_postun taidan-initial-setup.service taidan-initial-setup-reconfiguration.service

%files
%doc README.md
%license LICENSE.md LICENSE.dependencies
%{_bindir}/taidan
%{_libexecdir}/start-taidan
%{_libexecdir}/taidan-helper
%{_datadir}/taidan/polkit/
%{_presetdir}/95-taidan.preset
%config(noreplace) %{_sysconfdir}/pam.d/taidan
%{_sysusersdir}/taidan.conf
%{_unitdir}/taidan-initial-setup.service
%{_unitdir}/taidan-initial-setup-reconfiguration.service
%dir %{_prefix}/lib/taidan/
# the records of an unfinished install, see libtaidan's passwd.rs and tweaks.rs
%dir %{_sharedstatedir}/taidan/
%dir %{_datadir}/taidan/
%{_datadir}/taidan/avatars/
%{_datadir}/taidan/po/
%{_datadir}/taidan/session-config/

%files default-configs
# installable without taidan, e.g. alongside another distro's configs
%dir %{_datadir}/taidan/
%{_datadir}/taidan/tweaks/

%changelog
%autochangelog
