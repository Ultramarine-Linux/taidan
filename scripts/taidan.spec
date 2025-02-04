Name:           taidan
Version:        git~%{shortcommit}
Release:        1%?dist
Summary:        Out-Of-Box-Experience (OOBE) and Welcome App
SourceLicense:  GPL-3.0-or-later AND GPL-2.0-or-later
License:        (0BSD OR MIT OR Apache-2.0) AND Apache-2.0 AND (Apache-2.0 OR BSL-1.0) AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR MIT) AND (Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT) AND MIT AND (MIT OR Apache-2.0) AND (MIT OR Zlib OR Apache-2.0) AND Unicode-3.0 AND (Unlicense OR MIT) AND Zlib AND GPL-3.0-or-later AND GPL-2.0-or-later
URL:            https://github.com/Ultramarine-Linux/taidan
Source0:        %{url}/archive/%{gitcommit}.tar.gz
Requires:       (glib2 or (/usr/bin/plasma-apply-colorscheme and kf6-kconfig))
Requires:       shadow-utils
Requires:       systemd-udev
Requires:       bash
Requires:       (dnf5 and dnf5-command(copr))
Requires:       flatpak
Requires:       libwebp
Requires:       webp-pixbuf-loader
Requires:       xhost
Requires:       taidan-gui-backend
BuildRequires:  anda-srpm-macros mold cargo rust-packaging perl
BuildRequires:  pkgconfig(libhelium-1)
BuildRequires:  pkgconfig(openssl)
BuildRequires:  clang-libs
BuildRequires:  pkgconfig(libacl)
BuildRequires:  pkgconfig(libattr)
BuildRequires:  pkgconfig(gnome-desktop-4)

%description
Taidan is a GUI Out-Of-Box-Experience (OOBE) and Welcome App for Ultramarine
Linux, written in Rust and the Helium toolkit.

%package guiweston
Summary:        Taidan with weston GUI backend
Provides:       taidan-gui-backend
Conflicts:      taidan-gui-backend
RemovePathPostFixes: .guiweston

%description guiweston
Taidan with weston GUI backend.

%package guixorg
Summary:        Taidan with Xorg backend
Provides:       taidan-gui-backend
Conflicts:      taidan-gui-backend
RemovePathPostFixes: .guixorg

%description guixorg
Taidan with Xorg backend.

%prep
%autosetup -n taidan-%gitcommit
%cargo_prep_online

%build
%cargo_license_summary_online
%{cargo_license_online} > LICENSE.dependencies

%install
%cargo_install
DESTDIR=%buildroot ./scripts/install.sh

%find_lang com.fyralabs.Taidan

%files -f com.fyralabs.Taidan.lang
%doc README.md
%license LICENSE.md LICENSE.dependencies
%license scripts/libexec/COPYING
%_bindir/taidan
%_datadir/polkit-1/rules.d/100-taidan.rules
%_datadir/taidan/
%_libexecdir/taidan/
%_libexecdir/taidan/firstboot-windowmanager
%_libexecdir/taidan/initial-setup-graphical
%_libexecdir/taidan/reconfiguration-mode-enabled
%_libexecdir/taidan/run-initial-setup
%_sysconfdir/com.fyralabs.Taidan/
%_sysusersdir/taidan.conf
%_unitdir/taidan-initial-setup.service
%_unitdir/taidan-initial-setup-reconfiguration.service
%_sysconfdir/pam.d/taidan

%files guiweston
%_libexecdir/taidan/run-gui-backend.guiweston

%files guixorg
%_libexecdir/taidan/run-gui-backend.guixorg
