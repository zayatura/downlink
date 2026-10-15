# Downlink on Linux — Design Spec

- **Date:** 2026-10-15
- **Status:** Draft, awaiting review
- **Builds on:** [the v1 design](2026-10-13-downlink-design.md) (cited below as "v1 §n"), in particular
  v1 §10, "Linux-readiness rules"

## 1. Summary

Downlink gets a Linux port with the same features as the macOS app: the live layout window,
per-layer input-source switching, the tray menu, following the system Light/Dark appearance,
launching at login, and reconnecting by itself.

The first and only target is the author's setup: **Hyprland on Omarchy (Arch Linux), Wayland,
uwsm session, Omarchy's quickshell bar, fcitx5 running.** Other desktops are added later behind
the same interfaces.

## 2. Goals and success criteria

| # | Goal | Success looks like |
|---|------|--------------------|
| L1 | Feature parity | Every v1 goal (G1–G6) holds on the target setup, except the macOS-only app menu (§3). |
| L2 | Layout switching on Hyprland | Layer 0 selects `zd`; disconnecting or quitting selects the `us` fallback. Typing follows the switch in GTK, Qt and terminal apps. The bar's layout widget and `SUPER + Y` agree with Downlink. |
| L3 | Tray | The Downlink icon appears in the Omarchy bar. Its menu matches macOS, opens on a click, and survives a restart of the bar. |
| L4 | Stays running | Closing a window by any route (Meta+W included) leaves Downlink running. Only **Quit Downlink** quits. |
| L5 | No root at runtime | After a one-time udev rule, Downlink opens the keyboard as the logged-in user. |
| L6 | macOS unharmed | The macOS app builds and passes its checklist after the refactors this port needs. |

## 3. Non-goals

- GNOME, KDE, X11, other compositors, and IBus/fcitx5 backends.
- Per-device layout lists (for example, the Voyager on `zd,zd-hu,zd-ro` while the laptop keyboard
  keeps `zd,hu,ro`). Planned for later; see §10.
- AUR packaging. The udev rule and install script are shaped so a package can reuse them.
- Restarting Downlink after a crash. The macOS LaunchAgent does (`KeepAlive`); XDG autostart
  doesn't. Omarchy's crash watcher still reports a crash.
- Window shortcuts. On macOS, ⌘W/⌘Q are handled because the app menu would otherwise quit the
  process (v1 §8). On Linux the compositor closes windows (Meta+W), so Downlink adds no Ctrl+W/Ctrl+Q.
- A notification when the layout switches. macOS shows one itself; on Linux the bar's
  `omarchy.keyboard-layout` widget shows the active layout.

## 4. The target setup

What was observed on the target machine, and what the design relies on:

| Area | Observed | Consequence |
|------|----------|-------------|
| Session | Hyprland 0.56 (Lua config), Wayland, uwsm with `wayland-session-xdg-autostart@hyprland.desktop.target` | XDG autostart files in `~/.config/autostart/` start at login. |
| Layouts | Seat-wide `kb_layout = "zd,hu,ro"`, `kb_variant = ",102_qwerty_dot_nodead,std"`. `zd` is a custom layout in `~/.config/xkb/symbols/zd`: US with Hungarian and Romanian letters on AltGr. | The Voyager's HU and RO layers already send Right Alt + key, so ZD types every accent without a layout switch. |
| Layout cycling | `SUPER + Y` runs `keyboard-layout-cycle`, which applies one absolute index to every keyboard holding the seat's layout list, and reads the active one from the furthest-advanced typed keyboard | Downlink follows the same rules (§6), so all three stay in agreement. |
| fcitx5 | Running, profile has only `keyboard-us`. Its virtual keyboard `hl-virtual-keyboard-fcitx5` is Hyprland's "main" keyboard and carries the seat keymap. After a config reload its `layout` string can stay stale until the next keymap change. | Switches include the virtual keyboard when its list matches (§6.3). When it doesn't, it still follows the physical keyboards through fcitx5 (Phase 0, P2). |
| Hyprland IPC | `$XDG_RUNTIME_DIR/hypr/<instance>/.socket.sock`. A long-lived shell's `HYPRLAND_INSTANCE_SIGNATURE` can name a dead instance. | Socket discovery falls back to scanning (§6.1). |
| Tray | quickshell owns `org.kde.StatusNotifierWatcher`; `omarchy.tray` is in the bar | A StatusNotifierItem tray shows up. |
| Appearance | `org.gnome.desktop.interface color-scheme` is set by Omarchy themes | iced's portal-based detection follows it. |
| hidraw | `/dev/hidraw*` are `root:root 0600` without a rule. Arch's `extra/zsa-udev` package ships `50-oryx.rules` with the same `uaccess` rule as ours, and it was already installed here. | `zsa-udev` or our udev rule is needed (§8.3). |

**The test configuration.** For this port, the seat-wide list becomes `zd,hu,ro,us`. That's four
entries, the limit of the XKB text format Wayland clients receive. Downlink maps layer 0 → `zd`
and uses `us` as the fallback. Other layers stay unmapped. The macOS setup used separate
Hungarian and Romanian copies of the English layout, to tag the language; on Wayland, apps get a
keymap without a language, so those copies would only change the displayed name. They wait for
per-device lists.

## 5. Architecture

v1 §10's rule holds: OS-specific code lives in `input-source` backends and in `downlink`'s
platform modules. The tray becomes a third such seam.

| Unit | Change |
|------|--------|
| `input-source` | New `hyprland` module, `cfg(target_os = "linux")`. `system_backend()` returns it on Linux. `UnsupportedBackend` stays for other OSes. |
| `downlink/src/tray/` | `tray.rs` becomes `tray/mod.rs` (shared: menu IDs, `MenuAction`, `menu_action`, the menu-event channel), `tray/menu_model.rs` (the menu as data, built from `TrayView`), `tray/macos.rs` (today's `tray-icon`/`muda` code and `AppMenu`) and `tray/linux.rs` (`ksni`, and an `AppMenu` that does nothing). Both export `Tray` with `new()` and `apply(&TrayView, dark: bool)`. `App` uses them without `cfg`. |
| `downlink/src/platform/` | `other.rs` becomes `linux.rs` (autostart, plus the no-op `hide_dock_icon`, `activate_app` and `system_mode`). New `autostart.rs` holds the pure `.desktop` logic. Both it and `launch_agent.rs` are pure and build on every OS. A new `window_platform_specific()` gives each OS its iced window settings. |
| `downlink/src/app.rs` | Passes the effective appearance to the tray and refreshes it when the system appearance changes. Windows take `platform::window_platform_specific()`, which sets the Wayland `app_id` `downlink` on Linux. |
| `oryx-device` | `describe_open_error` recognises a permission error (§8.3). Nothing else is expected to change. |
| `oryx-protocol`, `oryx-layout`, `companion-core` | No changes. |
| `packaging/linux/` | `50-zsa.rules`, `downlink.desktop` (launcher entry), `install.sh`. |
| README, `docs/testing/manual-hardware-checklist.md` | A Linux section in each. |

New Linux-only dependencies:

- `ksni`, with its `blocking` API on the `async-io` backend (`default-features = false`): the
  StatusNotifierItem tray, pure Rust over D-Bus, no GTK. The default `tokio` backend can't be
  used: its blocking calls start a tokio runtime, and iced runs `update` inside its own tokio
  runtime, so tokio would panic ("Cannot start a runtime from within a runtime").
- `xkbcommon`: layout display names, linking the system libxkbcommon.
- `serde_json`, already a workspace dependency, parses Hyprland's replies. The socket uses only `std`.

Building needs a Rust toolchain installed through `rustup`. The version is pinned by
`rust-toolchain.toml`.

## 6. The Hyprland input-source backend

`HyprlandBackend` implements the existing `InputSourceBackend` trait (`list`, `current`, `select`).
`companion-core` and `App` don't change.

### 6.1 The IPC client

- **Socket.** Use `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock` if that file
  exists. Otherwise, use the one instance directory under `$XDG_RUNTIME_DIR/hypr/` that holds a
  `.socket.sock` that accepts a connection. Zero or several candidates give
  `Platform("Hyprland is not running")` or `Platform("several Hyprland instances are running")`.
- **Requests.** One connection per request: write the command, read the reply to EOF. Downlink
  sends `j/devices` (JSON) and `[[BATCH]]`-joined `switchxkblayout <device> <index>` commands.
- The socket path is resolved on each request. It's cheap, and it keeps working across a
  Hyprland restart.

### 6.2 Layout IDs and names

The layout list comes from a keyboard's `layout` and `variant` fields, split on commas and paired
up by position.

- **ID** = `layout` when the variant is empty, else `layout(variant)`. Examples: `zd`,
  `hu(102_qwerty_dot_nodead)`, `ro(std)`, `us`. This is what `config.toml` stores; it doesn't
  change when `kb_layout` is reordered.
- **Name** = the XKB layout name, read by compiling a keymap from the keyboard's
  rules/model/layout/variant/options with `xkbcommon` and asking for each layout's name. For
  example, "ZD (US with HU/RO alt keys)". If compilation fails, the name is the ID.
- `list()` returns the layouts of the seat's list, deduplicated by ID and sorted by name.

### 6.3 Which keyboards count

- **Untyped keyboards** are those whose name matches
  `^(hl-virtual-keyboard|power-button|sleep-button|lid-switch|video-bus)`, the same pattern as
  `keyboard-layout-cycle`.
- **Reading** uses typed keyboards only. `current()` returns the ID at the `active_layout_index`
  of the typed keyboard with the highest index. If there are no typed keyboards, it reads the
  keyboard Hyprland marks `main`. If there is no keyboard at all, it returns
  `Platform("no keyboards")`.
- **Switching** applies to every keyboard, typed or not, whose `layout` and `variant` equal the
  reading keyboard's. Including the untyped ones keeps them from drifting.

### 6.4 `select(id)`

1. Find the ID's index in the list. If it isn't there: `NotFound(id)`. Settings already marks a
   missing source, and the engine logs it once (v1 §8).
2. Send one batch of `switchxkblayout <name> <index>`, one per matching keyboard.
3. Every reply in the batch must be `ok`. Anything else becomes `Platform(<reply>)`.

## 7. The tray (`ksni`)

- **State.** `DownlinkTray` holds the `TrayView` and whether the effective appearance is dark. It
  implements `ksni::Tray`. `Tray::new()` spawns it with the blocking API, which runs the D-Bus
  service on its own thread. `Tray::apply(&TrayView, dark)` calls `handle.update(…)` only when
  something changed. iced reports system appearance changes on Linux even with no window open,
  so the icon follows the appearance at all times.
- **Labels.** DBusMenu treats `_` as a mnemonic marker, so status and detail text (which can hold a
  layer title) double every `_`.
- **Menu.** It matches macOS:
  - the status row and the detail row, both disabled
  - separator
  - Show Layout
  - Settings…
  - separator
  - Pause Input Switching (checkmark)
  - separator
  - Quit Downlink

  Each item's `activate` sends its existing menu ID (`show-layout`, `settings`, `pause`, `quit`) into
  the shared menu channel, so `menu_action` and `App::on_menu` are unchanged. There are no
  accelerator hints.

  Changed in Phase 1: the Linux menu leaves the separators out. Omarchy's bar draws each one
  1 logical pixel thick, which lands on 1 or 2 physical pixels at a 1.25× scale. The bar also
  reads `ItemsPropertiesUpdated` as an item's full property set, so every update reshapes the
  menu with a hidden trailing entry, and the bar reloads it through `LayoutUpdated`.
- **Clicks.** `MENU_ON_ACTIVATE = true`: a left click opens the menu, as on macOS.
- **Icon.** SNI has no template images, so `menubar-icon.svg` is rendered as ARGB32 pixmaps at
  22 px and 44 px in a solid foreground colour: light when the effective appearance is dark, dark
  when it is light. The connected and disconnected variants come from the existing
  `menubar_rgba(px, connected)`. The icon is redrawn when the appearance or the connection changes.
- **Resilience.** The tray is spawned with `assume_sni_available(true)`. If no
  StatusNotifierWatcher exists yet, for example when autostart beats the bar at login, the item
  registers as soon as one appears. `watcher_offline` logs and returns `true`, so the item
  registers again when the bar restarts. Any other spawn error is logged, and Downlink runs
  without a tray, as on macOS when the tray fails.

## 8. Platform glue

### 8.1 Launch at login

- Turning **Launch at login** on writes `$XDG_CONFIG_HOME/autostart/downlink.desktop`:
  `Type=Application`, `Name=Downlink`, `Exec=` the current executable (quoted per the Desktop
  Entry spec), `Icon=downlink`, `NoDisplay=true`. Turning it off removes the file. A missing file
  counts as removed.
- **Refresh at startup**, the Linux version of v1's "only from an app bundle" rule. When
  `launch_at_login` is on, rewrite the file if it is missing or its contents differ from the file
  for the current executable. Skip this when the executable sits in a cargo build directory
  (`…/target/debug/` or `…/target/release/`), so `cargo run` never takes over the login item.

### 8.2 Install script

`packaging/linux/install.sh` is the counterpart of `bundle.sh`. It is per-user and needs no sudo:

1. `cargo build --release -p downlink`.
2. Install the binary to `~/.local/bin/downlink`.
3. Render PNG icons with the existing `make-icons` bin into
   `~/.local/share/icons/hicolor/<size>x<size>/apps/downlink.png`.
4. Install `packaging/linux/downlink.desktop` (the launcher entry) to
   `~/.local/share/applications/`, with `Exec=` set to the installed binary's absolute path, so the
   entry works even when `~/.local/bin` isn't on the launcher's `PATH`.

Windows set the Wayland `app_id` `downlink` through iced's `platform_specific.application_id`, so
Hyprland and the launcher match them to that entry and its icon.

### 8.3 hidraw access

- `packaging/linux/50-zsa.rules`:
  `KERNEL=="hidraw*", ATTRS{idVendor}=="3297", TAG+="uaccess"`.
  This grants the logged-in user access to every ZSA board's hidraw nodes, matching the scope of
  ZSA's own rule. The `50-` prefix sorts it ahead of `73-seat-late.rules`, which applies `uaccess`.
- On Arch, the README recommends `sudo pacman -S zsa-udev`, which provides the same rule.
  Otherwise it installs ours with
  `sudo install -m644 packaging/linux/50-zsa.rules /etc/udev/rules.d/`, then
  `sudo udevadm control --reload` and `sudo udevadm trigger`.
- `describe_open_error` maps a permission error from hidapi to "No permission to open the keyboard
  (install the udev rule, see README)". The menu shows it, and the existing open backoff retries,
  so Downlink connects as soon as the rule is installed, without a restart.

### 8.4 Already Linux-ready (unchanged)

- Paths (v1 §7): `$XDG_CONFIG_HOME/downlink/config.toml`, `$XDG_CACHE_HOME/downlink/`, logs in
  `$XDG_STATE_HOME/downlink/`.
- The single-instance lock in `$XDG_RUNTIME_DIR`.
- Selecting the fallback on Quit and in the exit backstop.
- Window close: Downlink runs as an iced `daemon`, which keeps running with no windows open.
- Appearance: `platform::system_mode()` returns `Mode::None`, and iced's portal watcher supplies
  the mode. On Wayland, iced also reports `Mode::None` each time a window opens (winit can't read
  a window's theme there); `App` already ignores `Mode::None`, and must keep doing so.

### 8.5 Starting from the launcher, and starting again

Added during Phase 1, after Omarchy's launcher showed "Launching Downlink…" for up to 15 s: it
waits for a new window, and Downlink starts with only a tray icon.

- The launcher entry passes `--show-layout`, so a start from the launcher opens the Layout window.
  The autostart entry doesn't, so login stays silent.
- The instance holding the single-instance lock listens on a Unix socket next to the lock
  (`$XDG_RUNTIME_DIR/downlink.sock`). A second launch sends it `show-layout` and exits; the
  running instance opens its Layout window as the tray item does. `std` only, so it builds on
  macOS too, where Finder never starts a second copy.
- Hyprland ignores a focus request from Downlink itself (the launcher passes no activation
  token), so bringing an already-open window forward is left to the desktop: on Omarchy,
  `install.sh` writes the launcher entry as
  `omarchy-launch-or-focus "^downlink\\$" "<exe> --show-layout"`, which focuses an open Downlink
  window (matched by its class) and otherwise launches. Elsewhere the entry runs the executable
  directly. Downlink's code has nothing Hyprland-specific for this.

## 9. Delivery

### Phase 0 — Feasibility spike (throwaway code, not merged)

| # | Question | Probe |
|---|----------|-------|
| P1 | Does hidraw pairing work? | With the udev rule installed, `cargo run -p oryx-device --example watch` finds the `0xFF60/0x61` interface (hidapi must report the usage page from the report descriptor), pairs, prints LAYER and KEYDOWN events, and reconnects after a replug and after suspend/resume. |
| P2 | Does a layout switch reach typing? | A batch of `switchxkblayout` on every matching keyboard, with fcitx5 running. Type in a GTK app, a Qt app and a terminal. The bar widget and `SUPER + Y` agree afterwards. |
| P3 | Do iced and a `ksni` tray coexist? | A minimal iced `daemon` with a `ksni` tray: the menu opens on a left click, clicks reach iced, the icon is readable in a light and a dark Omarchy theme, and the item returns after the bar restarts. |
| P4 | Do windows behave? | Meta+W closes a window and the daemon keeps running. The `app_id` matches the launcher entry. Changing `color-scheme` re-themes an open window and a newly opened one. |

**Exit criteria:** each question has an answer, and this spec is updated where an answer changes
it. If P2 fails because of fcitx5, the fallback is a fcitx5 D-Bus backend
(`org.fcitx.Fcitx5.Controller1`) behind the same trait, brought back for review as a design change.

### Phase 1 — Linux port

Sections 5–8, built test-first. The plan breaks this into tasks and ends with a run of the macOS
checklist on a Mac (L6).

## 10. Later

- **Per-device layout lists.** A Hyprland `device` block gives the Voyager its own list
  (`zd,zd-hu,zd-ro`), with `zd-hu` and `zd-ro` as renamed copies of ZD, and Downlink switches only
  the Voyager's keyboards. This also lifts the four-layout limit on the seat.
- AUR packaging, possibly relying on an existing ZSA udev-rules package.
- Other desktops (GNOME, KDE, IBus/fcitx5) as more `InputSourceBackend`s.
- Restarting after a crash through a systemd user service.

## 11. Testing

- **Unit tests (pure functions):**
  - Hyprland: parsing `j/devices` JSON (captured fixtures from the target machine), pairing
    layouts with variants into IDs, choosing the reading keyboard, choosing the keyboards to
    switch, building the batch, interpreting batch replies.
  - Socket discovery against temporary directories.
  - Layout names from `xkbcommon`, using a stock layout.
  - The tray menu model (labels, enabled flags, the checkmark, IDs in order) and the RGBA → ARGB32
    big-endian pixmap conversion.
  - The autostart `.desktop` text, `Exec` quoting, and the refresh decision.
  - `describe_open_error` for the permission case.
- **Smoke test:** `hyprland_smoke`, `#[ignore]` like `macos_smoke`, lists, reads and re-selects
  the current layout on the live compositor.
- **Workspace:** `cargo test --workspace` and
  `cargo clippy --workspace --all-targets -- -D warnings` pass on Linux. Tests of macOS-only
  modules (`launch_agent`) are `cfg`-gated.
- **Manual:** a Linux section in `docs/testing/manual-hardware-checklist.md`, mirroring the macOS
  one. It covers connecting, switching on a layer change, the fallback on unplug and on Quit, the
  live view, Light/Dark, the tray after a bar restart, and launching after a reboot.
- **macOS:** the existing checklist, run on a Mac after Phase 1 (L6).
