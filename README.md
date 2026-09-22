# 🏮 Taidan

> 🎵 _[**チョウチン少女** - Green wisteria](https://www.youtube.com/watch?v=1-wL7bb_l-o)_

Taidan is a GUI Out-Of-Box-Experience (OOBE) and Welcome App for Ultramarine
Linux, written in Rust and the [Helium] toolkit.

## 📦 Dependencies

For an up-to-date list, see `scripts/taidan.spec`.

```
libhelium
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
pkgconfig(libhelium-1)
```

## Desktop and server builds

The repository contains two independent applications:

- `taidan`: the native desktop OOBE. It does not require Node.js, pnpm, or the web OOBE.
- `server-oobe`: the optional Ultramarine Server web OOBE.

Build the desktop application only:

```sh
cargo build --release -p taidan
```

Build or check the optional server application:

```sh
pnpm --dir server-oobe install --frozen-lockfile
pnpm --dir server-oobe build
```

The `justfile` provides equivalent `build-desktop`, `build-server-oobe`, and
`build-all` recipes.

## Testing

```sh
TAIDAN_LOG=trace cargo r
```

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

[Helium]: https://developer.fyralabs.com/helium/hig
