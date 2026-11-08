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
bash, util-linux [runuser and lastlog2]
dnf5, dnf5-command(copr) and dnf5-command(config-manager)
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
- `TAIDAN_SIMULATE_UPDATES=1` pretends updates were downloaded, to see the done page's restart button
- `TAIDAN_SIMULATE_DRIVER_ERROR=1` pretends a driver failed to install, to see the done page's note about it

To debug the OOBE on a real machine, boot with `taidan.debug_shell` on the kernel command line,
then press <kbd>Alt</kbd>+<kbd>Shift</kbd>+<kbd>T</kbd> to open a root terminal.

## 📃 License

`GPL-3.0-or-later`

    Copyright © 2024~2026  Fyra Labs & Ultramarine Linux Contributors

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

A few things work differently from the GTK version of Taidan:

- `skip_pages = ["devicename"]` is ignored with a warning, since the device name moved onto the
  network page. Skipping `network` keeps the system's hostname.
- `skip_pages = ["keyboard"]` also hides the input method picker, which is part of the keyboard
  page now. Input methods are then picked from the locale. To set up none at all, skip
  `inputmethod`.
- Codecs follow the drivers toggle, there's no separate codecs page. `skip_pages = ["codecs"]` is
  the same as `drivers`. Skipping it hides the page, but drivers and codecs are still installed.
- `internet_timeout` and `internet_retry_interval` are ignored with a warning, and
  `/etc/com.fyralabs.Taidan/detect-internet` isn't run anymore. Whether the machine is online comes
  from NetworkManager's connectivity check.
- `/usr/share/taidan/oobe.sh` isn't run anymore. Use a tweak instead (see below).

### Tweaks

Tweaks are optional settings the user can turn on from the summary page. Each one is a directory in
`/usr/share/taidan/tweaks/`, whose name is the tweak's id, with an executable `up` script and an
optional `tweak.toml` (see `Tweak` in `libtaidan/tweaks.rs` for its fields). `up 1` turns the tweak
on. `up 0` only runs for a tweak that an earlier, unfinished install turned on and the user has
since turned off. Both run as root.

`up` gets the user's choices as JSON on stdin (`Settings` in `libtaidan/settings.rs`), roughly:

```jsonc
{
  "skipconfig": false,
  "nointernet": false,            // setting up offline
  "kb_layout": "us",
  "kb_variant": null,
  "device_name": "Cypress's Laptop",
  "hostname": "cypresss-laptop",
  "timezone": "America/Denver",   // "" to keep the system's
  "locale": "en_US.UTF-8",        // "" to keep the system's
  "fullname": "Cypress",
  "username": "cypress",
  "avatar": null,                 // path to the user's icon
  "install_codecs_drivers": true,
  "download_updates": true,
  "location": true,               // null when the location page was skipped
  "a11y": { "screen_reader": false, "zoom": false, "larger_text": false, ... },
  "dark_mode": true,              // null to keep the distro's default
  "accent": "blue",               // or teal, green, yellow, orange, red, pink, purple, slate, or null
  "proxy": null,                  // or { "url": "http://proxy:3128", "no_proxy": "localhost" }
  "proxy_persist": true,          // whether the installed system keeps using the proxy
  "ims": ["Pinyin"],
  "ims_picked": false,
  "imf": "fcitx5",                // or "ibus"
  "actions": [[], [], [], [], []],
  "tweaks": ["my_tweak"]          // ids of the tweaks to turn on
}
```

The proxy URL on stdin never has the username and password in it. `up` gets the whole proxy,
credentials and all, in `http_proxy` and friends for its own downloads. A tweak that sets the proxy
up anywhere that outlives the install should only do so when `proxy_persist` is `true`, Taidan
removes its own proxy config otherwise.

With `proxy_persist` on, Taidan writes the proxy to `/etc/environment` (without credentials) and
`/etc/dnf/libdnf5.conf.d/50-taidan-proxy.conf` (root only). libdnf5 has no `no_proxy`, so repos
whose `baseurl`, `metalink` or `mirrorlist` host is in the bypass list get `proxy=` in
`/etc/dnf/repos.override.d/50-taidan-proxy-bypass.repo`. This is a best effort: the mirrors a
metalink or mirrorlist hands out are only known at runtime and still go through the proxy.

Compared to the GTK version:

- `langlocale` is now `locale`
- `theme_is_dark` is now `dark_mode`, which can be `null`
- `accent` is lowercase (`"blue"` instead of `"Blue"`)
- `tweaks` is a list of the ids to turn on instead of a `bool` per tweak
- `passwd` is gone
- `timezone`, `avatar`, `download_updates`, `location`, `a11y`, `proxy`, `proxy_persist`,
  `ims_picked` and `imf` are new

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
t1_color = "#ffffff"                       # Taidan version color, only shown in dev sessions
hero_image = "/usr/share/backgrounds/my-distro.png"       # the picture, light mode
hero_image_dark = "/usr/share/backgrounds/my-distro-dark.png"
```

The picture falls back to `/usr/share/backgrounds/default.png` (or `default-dark.png`), then to
Ultramarine's. The distro name comes from `NAME=` in `/etc/os-release`. Choosing the legible font in
the accessibility settings always uses Atkinson Hyperlegible Next instead of `font`.

[Slint]: https://slint.dev
