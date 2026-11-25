# Changelog

## Unreleased

- Fixed automatic dialog sizing using laid-out content in `TdnDialog`. Warning/error
  bodies grow to fit wrapped text and scroll only when the window's available height is exceeded.
- Added Rust warning-dialog support and `TAIDAN_SIMULATE_WARNING` at application startup. A non-Plasma
  `gui_env` with explicit IMF `start = "kwinrc"` produces a configuration warning.
- Added a Fcitx5 environment.d drop-in for input-method environment variables, including
  skeleton setup and marker-based cleanup.
- Accept an `atomic-` prefix in `gui_env`, independently of edition metadata.
- Configuration parse failures now bypass normal startup/account checks so the error
  dialog can open even on an already-configured system.
- Added Fcitx5 XDG autostart setup for niri/Noctalia and other non-Plasma sessions,
  including systemd-homed skeleton setup and marker-based retry cleanup.
- Added optional `gui_env` to select the target desktop/window manager. Desktop behavior
  is no longer inferred from `edition`, `/etc/os-release` variant IDs, or the presence of
  `plasmashell`. Maintainers should add `gui_env` to `/etc/taidan.toml`; `edition` remains
  distribution metadata.
- Added per-step IMF configuration with optional `imf` and `start` fields. The existing
  `[i18n].imf` setting is supported, with stage-specific `imf` taking precedence.
- Moved step configuration into the step structs. Stages and system-setting sub-steps
  implement `InstallStep`, with static async dispatch through `enum_dispatch`. Omitted
  operations use defaults; configuration does not change execution order.
- Deprecated the global `[i18n]` and `[taidan0]` tables. IMF selection belongs to the
  `Imf` sub-step; `continue_if_user_exists` and `systemd_homed` belong to `user_add`;
  `skip_dnf` and `persist_no_internet_btn` belong to `system_setup`. Legacy tables remain
  fallbacks, but explicitly configured step fields take precedence, including `false`.
  Parsing either legacy table logs a deprecation warning; direct use of the Rust
  `Config.i18n` or `Config.taidan0` fields also produces a compiler deprecation warning.

## GTK-to-Slint migration notes

- `skip_pages = ["devicename"]` is ignored with a warning; the device name is on the
  network page. Skipping `network` keeps the system's hostname.
- `skip_pages = ["keyboard"]` also hides the input method picker, which moved onto the
  keyboard page. Input methods are then picked from the locale. To set up none, skip
  `inputmethod`.
- Codecs follow the drivers toggle. `skip_pages = ["codecs"]` is the same as `drivers`,
  which hides the page but still installs drivers and codecs.
- `internet_timeout` and `internet_retry_interval` are ignored with a warning, and
  `/etc/com.fyralabs.Taidan/detect-internet` is no longer run. Connectivity comes from
  NetworkManager.
- `/usr/share/taidan/oobe.sh` is no longer run. Use a tweak instead.
- The analytics page sends the Plausible pageview `detect-internet` used to send, once,
  at the end of setup, and only if the user leaves it on. Skip `analytics` to disable it.
- Taidan no longer ships a systemd preset. Distribution presets need to enable
  `taidan-initial-setup-reconfiguration.service`.
