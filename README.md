# 🏮 Taidan

> 🎵 _[**チョウチン少女** - Green wisteria](https://www.youtube.com/watch?v=1-wL7bb_l-o)_

Taidan is a GUI Out-Of-Box-Experience (OOBE) and Welcome App for Ultramarine
Linux, written in Rust and the [Slint] toolkit.

## 📦 Dependencies

For an up-to-date list, see `scripts/taidan.spec`.

```
fontconfig
vercel-geist-fonts
libxkbcommon, wayland and EGL libraries [loaded at runtime by Slint]
gsettings or (plasma-apply-colorscheme and kwriteconfig6)
shadow-utils
systemd-udev [for systemd-timesyncd.service]
sh
dnf5 and dnf5-command(copr)
flatpak
xkeyboard-config [for /usr/share/X11/xkb/rules/evdev.lst]
```

### 🛠️ Build Dependencies

```
pkgconfig(openssl)
pkgconfig(fontconfig)
pkgconfig(libxcrypt)
pkgconfig(xcb)
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
  hardware is present, to see the drivers list
- `TAIDAN_SIMULATE_ERROR=1` makes the simulated install fail halfway

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
skip_pages = ["theme", "browser"] # maybe you don't want taidan to setup themes / install a browser
org = "Fyra Labs"
```

[Slint]: https://slint.dev
