# 🏮 Taidan

> 🎵 _[**チョウチン少女** - Green wisteria](https://www.youtube.com/watch?v=1-wL7bb_l-o)_

Taidan is a GUI Out-Of-Box-Experience (OOBE) and Welcome App for Ultramarine
Linux, written in Rust and the [Slint] toolkit.

## 📦 Dependencies

For an up-to-date list, see `scripts/taidan.spec`.

```
kwin, dbus-daemon, swaybg [the OOBE session]
polkit [running the setup as root]
fontconfig
vercel-geist-fonts, atkinson-hyperlegible-next-fonts and atkinson-hyperlegible-mono-fonts
orca [screen reader]
libxkbcommon, wayland and EGL libraries [loaded at runtime by Slint]
NetworkManager
cog [browser for captive portals]
tzdata [time zones]
gsettings or (plasma-apply-colorscheme and kwriteconfig6)
kwriteconfig6
shadow-utils
systemd and systemd-udev
kbd and procps-ng [chvt and pkill for the session]
bash, util-linux-script
dnf5 and dnf5-command(copr)
flatpak
xdg-terminal-exec [the debug terminal, see Testing]
umcli [tweaks, through taidan-default-configs]
chrony and grubby [recommended, the clock and NVIDIA drivers]
```

### 🛠️ Build Dependencies

```
pkgconfig(openssl)
pkgconfig(fontconfig)
pkgconfig(libxcrypt)
pkgconfig(icu-i18n)
clang
glibc-all-langpacks [for `localedef --list-archive`]
xkeyboard-config [for /usr/share/X11/xkb/rules/evdev.lst]
```

## tdnx

`tdnx` is a small libtaidan bridge used by
[Ultramarine Server OOBE](https://github.com/Ultramarine-Linux/server-oobe) to
validate setup operations. It isn't part of the desktop build or the taidan
package. `server-oobe` builds it from this repo:

```sh
cargo build --release -p tdnx --no-default-features --features server-oobe
```

## Testing

```sh
TAIDAN_LOG=trace cargo r
```

- `TAIDAN_SIMULATE_HW=nvidia,broadcom-wifi,broadcom-bluetooth` (or `all`) pretends that
  hardware is present, to see the drivers list (with `nvidia`, the simulated install asks for a
  reboot)
- `TAIDAN_SIMULATE_ERROR=1` makes the simulated install fail halfway

To debug the OOBE on a real machine, boot with `taidan.debug_shell` on the kernel command line,
then press <kbd>Alt</kbd>+<kbd>Shift</kbd>+<kbd>T</kbd> to open a root terminal.

## 📃 License

`GPL-3.0-or-later`

    Copyright © 2024~2025  Fyra Labs & Ultramarine Linux Contributors

    This program is free software; you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation; either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License along
    with this program; if not, write to the Free Software Foundation, Inc.,
    51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.

## 🕶️ Hacking

Taidan reads an optional configuration file at `/etc/taidan.toml`:

```toml
edition = "plasma"
skip_pages = ["location", "drivers"] # hide pages you don't want, see `SkipPage` in src/cfg.rs for the names
org = "Fyra Labs"
```

### Branding

Other distros can swap Ultramarine's look out from the same file. Every key is optional.
Colors follow `#rrggbb` or `#aarrggbb`.

```toml
logo = "/usr/share/pixmaps/my-distro.svg"  # header and window icon
font = "Inter"                             # needs to be installed, Geist if unset
accent_color = "#00078f"                   # main buttons and selections, light mode
accent_color_dark = "#4a56e2"              # the same in dark mode
accent_foreground_color = "#ffffff"        # text and icons on the accent
accent_foreground_color_dark = "#ffffff"   # dark mode, accent_foreground_color if unset

[welcome]
t0 = "My Distro 42"                        # caption on the picture beside the pages, none if unset
t0_color = "#ffffff"                       # caption color
t1_color = "#ffffff"                       # Taidan version color
hero_image = "/usr/share/backgrounds/my-distro.png"       # the picture, light mode
hero_image_dark = "/usr/share/backgrounds/my-distro-dark.png"
```

The picture falls back to `/usr/share/backgrounds/default.png` (or `default-dark.png`), then to
Ultramarine's. The distro name comes from `NAME=` in `/etc/os-release`. Choosing the legible font in
the accessibility settings always uses Atkinson Hyperlegible Next instead of `font`.

[Slint]: https://slint.dev
