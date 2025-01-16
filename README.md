# 🏮 Taidan

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

## Testing

```sh
TAIDAN_CATALOGUE_DIR=./catalogue TAIDAN_LOG=trace cargo r
```

Also, to test if a catalogue category file can be parsed:

```sh
cargo run -p taidan_catalogue_parser -- catalogue/browser.yml
```

## 📃 License

    Copyright © 2024  Fyra Labs & Ultramarine Linux Contributors

    This program is free software; you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation; either version 2 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License along
    with this program; if not, write to the Free Software Foundation, Inc.,
    51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
