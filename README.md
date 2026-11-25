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
clang
glibc-all-langpacks [for `localedef --list-archive`]
xkeyboard-config [for /usr/share/X11/xkb/rules/evdev.lst]
```

### Root helper

Taidan runs as the unprivileged `taidan` user. Everything it does as root goes through
`/usr/libexec/taidan-helper` (`libtaidan/helper.rs`), the only path the polkit rule lets `taidan`
`pkexec` while the OOBE runs. The helper takes a fixed set of requests with checked arguments, and
once the system is marked configured it refuses everything but a few cleanup ones.

This limits what a bug in Taidan can do as root, but it isn't a boundary against other code running
as `taidan`: the OOBE exists to make an admin with a chosen password, so anything that can make
those requests can get root. Nothing untrusted may run as `taidan`, which is why Orca's help and
links open in an empty sandbox and the captive portal browser runs as its own user. Packaging has
to install the helper:

```sh
cargo build --release -p libtaidan --bin taidan-helper
install -Dpm755 target/release/taidan-helper /usr/libexec/taidan-helper
```

## tdnx

`tdnx` is a small libtaidan bridge used by
[Ultramarine Server OOBE](https://github.com/Ultramarine-Linux/server-oobe) to
validate setup operations. It isn't part of the desktop build or the taidan
package. `server-oobe` builds it from this repo:

```sh
cargo build --release -p tdnx --no-default-features --features server-oobe
```

The `desktop-install` backend goes through `taidan-helper` like Taidan, so the helper has to be
installed. Custom repos, coprs and scripts in `actions`, and packages Taidan doesn't install itself,
are only taken when `tdnx` runs as root.

## Testing

```sh
TAIDAN_LOG=trace cargo r
```

- `TAIDAN_SIMULATE_HW=nvidia,broadcom-wifi` (or `all`) fakes that hardware for the drivers list
  (with `nvidia`, the simulated install asks for a reboot)
- `TAIDAN_SIMULATE_ERROR=1` makes the simulated install fail halfway
- `TAIDAN_SIMULATE_WARNING=1` opens a non-fatal warning dialog at application startup
- `TAIDAN_SIMULATE_UPDATES=1` fakes downloaded updates, for the done page's restart button
- `TAIDAN_SIMULATE_DRIVER_ERROR=1` fakes a failed driver install, for the done page's note about it

On a real machine, boot with `taidan.debug_shell` on the kernel command line, then press
<kbd>Alt</kbd>+<kbd>Shift</kbd>+<kbd>T</kbd> for a root terminal.

Debug builds run the root helper from `TAIDAN_HELPER` if it's set.

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

`GPL-2.0-or-later` applies to portions borrowed from / referencing [_Initial
Setup_ from Red Hat](https://github.com/rhinstaller/initial-setup), including
but not limited to the systemd service files.

## 🕶️ Hacking

Taidan reads an optional configuration file at `/etc/taidan.toml`:

```toml
edition = "plasma"
gui_env = "plasma"
skip_pages = ["location", "drivers"] # see `SkipPage` in libtaidan/cfg.rs for the names
org = "Fyra Labs"
```

`gui_env` specifies the **target** desktop/window manager (`gnome`, `plasma`, `budgie`,
`xfce`, or any other name, e.g. `niri`). Desktop behavior is selected by `gui_env`.
An `atomic-` prefix in `gui_env` is accepted, e.g. `atomic-gnome` selects GNOME.
When `gui_env` is unset, Taidan picks Plasma if `/usr/bin/plasmashell` exists, and otherwise
falls back to `edition` (`VARIANT_ID` from os-release by default). An unrecognized `gui_env`
uses generic desktop behavior.

Installation stages always run in their fixed order. Optional `steps` entries configure
individual stages, without enabling, disabling, or reordering them. For niri/Noctalia:

```toml
gui_env = "niri"

[[steps]]
stage = "apply_system_settings"

[[steps.apply_system_settings]]
[steps.apply_system_settings.imf]
imf = "fcitx5"          # or "ibus"; omit to use the default
start = "xdg_autostart" # or "kwinrc"; omit to use the default
```

The default IMF is IBus for GNOME, Fcitx5 otherwise. The default Fcitx5 startup method is
`kwinrc` for Plasma, XDG autostart otherwise; `start` does not change IBus setup.
XDG autostart writes `~/.config/autostart/org.fcitx.Fcitx5.desktop`, and requires the target session
to process XDG autostart entries. Both Fcitx5 startup methods use the same input-method
profile, including in systemd-homed skeletons. Retries clean up Taidan-owned stale files.
Fcitx5 also writes `~/.config/environment.d/90-taidan-fcitx5.conf`. With `kwinrc` it only sets
`XMODIFIERS` to `@im=fcitx` for XWayland, since Plasma apps use the Wayland text-input protocol.
With XDG autostart it also sets `INPUT_METHOD`, `GTK_IM_MODULE`, and `QT_IM_MODULE` to `fcitx`.
The file uses environment.d assignments, not shell `export` commands.
Explicit `start = "kwinrc"` outside Plasma opens a configuration warning dialog.
IMF selection uses the step's `imf`, then `[i18n].imf` as a compatibility fallback,
then the desktop default.
Omitted configurations use defaults; duplicate stage or system-setting entries are rejected.

Each stage owns its configuration and implements `InstallStep`, with static dispatch through
the Serde-tagged `steps::Module` enum (`enum_dispatch`). `Stage` is the lightweight progress
identity. `ApplySystemSettings` owns a `SystemSetting` enum for its sub-steps; the IMF options and
default selection live alongside IMF setup in `libtaidan/steps/_08_apply_system_settings/imf.rs`.
Both levels fill omitted operations with defaults and retain their fixed execution order.

Account and installation policy are configured on their owning stages:

```toml
[[steps]]
stage = "user_add"
continue_if_user_exists = false
systemd_homed = false

[[steps]]
stage = "system_setup"
skip_dnf = false
persist_no_internet_btn = false
```

`skip_dnf` is installation-wide and applies to all DNF stages. `persist_no_internet_btn`
controls the network page's offline continuation button. These policies are available to the
UI before any stage executes. Resolution is per field: explicit step option, then the
corresponding `[taidan0]` value, then `false`. Explicit `false` overrides a legacy `true`.
The compatibility table and its deprecation are described in [CHANGELOG.md](CHANGELOG.md).

### Page configuration

- Skipping `network` keeps the system's hostname.
- Skipping `keyboard` hides the input method picker; input methods are picked from the locale.
  Skip `inputmethod` to set up none.
- Codecs follow the drivers toggle. Skipping `drivers` hides the page but still installs drivers
  and codecs.
- Skip `analytics` to disable analytics.

Distribution presets must enable `taidan-initial-setup-reconfiguration.service`.

For migration notes, see [CHANGELOG.md](CHANGELOG.md).

### Tweaks

Tweaks are optional settings the user can turn on from the summary page. Each is a directory in
`/usr/share/taidan/tweaks/` named after the tweak's id, with an executable `up` script and an
optional `tweak.toml` (fields in `Tweak`, `libtaidan/tweaks.rs`). `up 1` turns the tweak on. `up 0`
only runs when an earlier, unfinished install turned it on and the user has since turned it off.
Both run as root.

An optional `condition` script determines whether a tweak applies to this system.
The tweak is enabled if the exit code is 0; nonzero, failure to run, or a five-second timeout hides it.
`TAIDAN_X86_64_LEVEL` (1–4 on x86-64, 0 elsewhere) and `TAIDAN_SECURE_BOOT`
(`0` for disabled, `1` for enforced, `unknown` if detection failed) are available.

`up` gets the user's choices as JSON on stdin (`Settings` in `libtaidan/settings.rs`):

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
  "analytics": true,              // null when the analytics page was skipped
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

The proxy URL on stdin never has credentials in it. For its own downloads, `up` gets the full proxy
in `http_proxy` and friends. A tweak should only set the proxy up somewhere that outlives the install
when `proxy_persist` is `true`, otherwise Taidan removes its own proxy config.

With `proxy_persist` on, Taidan writes the proxy to `/etc/environment` (without credentials) and
`/etc/dnf/libdnf5.conf.d/50-taidan-proxy.conf` (root only). libdnf5 has no `no_proxy`, so repos
whose `baseurl`, `metalink` or `mirrorlist` host is in the bypass list get `proxy=` in
`/etc/dnf/repos.override.d/50-taidan-proxy-bypass.repo`. Mirrors handed out by a metalink or
mirrorlist are only known at runtime, so they still go through the proxy.

Changes from the GTK version:

- `langlocale` is now `locale`
- `theme_is_dark` is now `dark_mode`, which can be `null`
- `accent` is lowercase (`"blue"` instead of `"Blue"`)
- `tweaks` is a list of the ids to turn on instead of a `bool` per tweak
- `passwd` is gone
- `timezone`, `avatar`, `download_updates`, `location`, `a11y`, `proxy`, `proxy_persist`,
  `ims_picked` and `imf` are new

### Branding

Other distros can replace Ultramarine's look from the same file. Every key is optional. Colors are `#rrggbb` or `#aarrggbb`.

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
Ultramarine's. The distro name comes from `NAME=` in `/etc/os-release`. The legible font option in
the accessibility settings overrides `font` with Atkinson Hyperlegible Next.

[Slint]: https://slint.dev
