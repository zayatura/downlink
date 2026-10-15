# Downlink on Linux Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port Downlink to Linux, for Hyprland on Omarchy, with the macOS feature set: live layout window, per-layer layout switching, tray menu, Light/Dark, launch at login, and reconnecting by itself.

**Architecture:**
- The platform-neutral crates (`oryx-protocol`, `oryx-device`, `oryx-layout`, `companion-core`) already build and pass on Linux.
- New work goes behind three existing or new seams:
  - a Hyprland `InputSourceBackend` in `input-source` (IPC over Hyprland's Unix socket, layout names from libxkbcommon),
  - a `ksni` StatusNotifierItem tray beside the macOS `tray-icon` one in `downlink/src/tray/`,
  - Linux glue in `downlink/src/platform/` (XDG autostart, Wayland `app_id`).
- Phase 0 is a throwaway spike that answers the four open questions (spec §9) before production code.

**Tech Stack:** Rust 1.99.0 (pinned in `rust-toolchain.toml`, edition 2024), iced 0.14 (`canvas`, `tokio`), ksni 0.3 (`blocking` + `async-io`, no default features), xkbcommon 0.9 (no default features), serde 1 / serde_json 1, hidapi 2.6 (hidraw backend via its default `linux-static-hidraw`), tray-icon 0.26 (macOS only), tempfile 3 (tests).

**Spec:** `docs/superpowers/specs/2026-10-15-linux-support-design.md`. It builds on `docs/superpowers/specs/2026-10-13-downlink-design.md` (cited as "v1 §n"). Read both before starting.

## Global Constraints

- **Toolchain:** Rust `1.99.0` from `rust-toolchain.toml`, installed through `rustup` (already installed). MSRV `rust-version = "1.90"`. `edition = "2024"`, workspace `resolver = "3"`.
- **Target:** Hyprland on Omarchy (Arch Linux), Wayland, uwsm session, Omarchy's quickshell bar, fcitx5 running. No GNOME, KDE, X11 or other compositors.
- **Platform isolation:** `cfg(target_os = …)` code lives only in `crates/input-source/src/lib.rs` and its backends (`macos.rs`, `unsupported.rs`, `hyprland/`), `crates/downlink/src/platform/` and `crates/downlink/src/tray/`. Everything else is platform-neutral.
- **macOS keeps working:** macOS code may be moved and re-wired, but its behaviour must not change. Task 16 checks it on a Mac.
- **License:** MIT. Never copy code or comments from ZSA's GPL-2.0 sources.
- **Identity:** commits are authored as `zayatura <zayatura@mailbox.org>`, already set in the repo-local git config; don't change it. No real name, username, work email or absolute home path (`/home/<name>/…`, `/Users/<name>/…`) in any committed file, commit message, test, fixture or log format. Test paths use `/opt/…`, `/usr/local/…` or temp dirs.
- **Keyboard facts (v1 §4.2):** vendor ID `0x3297`, Voyager product ID `0x1977`, raw-HID usage page `0xFF60`, usage `0x61`.
- **Hyprland IPC:** socket `$XDG_RUNTIME_DIR/hypr/<instance>/.socket.sock`. Requests used: `j/devices`, and `[[BATCH]]` + commands joined by ` ; `. A batch reply joins one reply per command with `\n\n\n`; success is `ok` (verified on Hyprland 0.56.2).
- **Untyped keyboards** (spec §6.3): names starting with `hl-virtual-keyboard`, `power-button`, `sleep-button`, `lid-switch` or `video-bus`.
- **Layout IDs** (spec §6.2): `layout` when the variant is empty, else `layout(variant)`.
- **Every task ends green:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, all on Linux. Run `cargo fmt --all` before the check: code in this plan isn't always rustfmt-formatted. Exception: until Task 5 lands, the `downlink` crate doesn't compile on Linux, so Phase 0 tasks check only what they touch.
- **Commit messages** use Conventional Commit style (`feat(input-source): …`) and end with the line `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Human-in-the-loop steps** are marked **[HUMAN]**: they need your human partner to plug in hardware, type, run `sudo`, or look at the bar. Ask, wait, and record what they report.

## Review Focus

These are the five conditions the spec implies but no feature test would naturally exercise, most likely first. Each has a pinning test in the task that owns the code.

1. **Hyprland is unreachable or `HYPRLAND_INSTANCE_SIGNATURE` is stale.** Examples: Downlink started from a long-lived terminal, or Hyprland restarted while Downlink runs. Expected: discovery falls back to the one live instance, otherwise a `Platform` error; no panic, and the next call tries again. Tests: Task 9 (`a_stale_signature_falls_back_to_the_live_instance`, `no_live_instance_is_an_error`, `several_live_instances_are_an_error`).
2. **`kb_variant` with fewer (or more) entries than `kb_layout`, or empty.** Example: `kb_layout = "zd,hu,ro,us"` with `kb_variant = ""`. Expected: layouts pair with variants by position, and missing variants count as empty. Test: Task 8 (`a_short_or_empty_variant_list_leaves_the_rest_plain`).
3. **A configured layout that's no longer in `kb_layout`.** Example: `us` removed from `input.lua` after setting it as the fallback. Expected: `NotFound`, nothing sent to Hyprland, logged once by the existing engine path. Test: Task 8 (`an_unknown_id_is_not_found`).
4. **Keyboards holding different layout lists.** Example: a per-device `kb_layout` added later. Expected: only keyboards holding the reading keyboard's list are switched; the others are left alone. Test: Task 8 (`keyboards_with_a_different_list_are_left_alone`).
5. **An executable path with spaces, `%`, `$` or quotes in the autostart `Exec=`.** Expected: quoted and escaped per the Desktop Entry spec, so the login item still starts. Test: Task 12 (`exec_quotes_and_escapes_reserved_characters`).

---

## File Structure

```
crates/input-source/
  Cargo.toml                         + Linux deps: serde, serde_json, xkbcommon (Task 8, 10)
  src/lib.rs                         cfg: hyprland on Linux, unsupported elsewhere (Task 11)
  src/hyprland/mod.rs                HyprlandBackend: InputSourceBackend (Task 11)
  src/hyprland/devices.rs            j/devices parsing + pure decisions (Task 8)
  src/hyprland/socket.rs             socket discovery + one-shot request (Task 9)
  src/hyprland/names.rs              XKB layout names via libxkbcommon (Task 10)
  tests/hyprland_devices.rs          (Task 8)
  tests/hyprland_socket.rs           (Task 9)
  tests/hyprland_names.rs            (Task 10)
  tests/hyprland_smoke.rs            #[ignore], live compositor (Task 11)
  tests/fixtures/hyprland-devices.json   captured in Phase 0 (Task 4)
crates/oryx-device/src/hid.rs        describe_open_error: permission case (Task 13)
crates/downlink/
  Cargo.toml                         tray-icon → macOS only; ksni on Linux (Task 5, 7)
  src/tray/mod.rs                    shared IDs, MenuAction, menu channel (Task 5)
  src/tray/macos.rs                  today's tray.rs tray-icon/muda code (Task 5)
  src/tray/linux.rs                  stub (Task 5), ksni tray (Task 7)
  src/tray/menu_model.rs             the menu as data (Task 6)
  src/icon.rs                        + menubar_argb (Task 6)
  src/platform/mod.rs                cfg switch macos/linux (Task 12)
  src/platform/linux.rs              replaces other.rs (Task 12)
  src/platform/autostart.rs          pure XDG autostart logic (Task 12)
  src/platform/macos.rs              + window_platform_specific (Task 12)
  src/app.rs                         tray.apply(view, dark); window platform settings (Task 5, 12)
  tests/tray_menu.rs                 (Task 6)
  tests/icon.rs                      + ARGB tests (Task 6)
  tests/autostart.rs                 (Task 12)
packaging/linux/50-zsa.rules         udev rule (Task 14)
packaging/linux/downlink.desktop     launcher entry template (Task 14)
packaging/linux/install.sh           per-user install (Task 14)
README.md                            Linux section (Task 14)
docs/testing/manual-hardware-checklist.md   Linux section (Task 15)
docs/superpowers/spikes/linux-phase0-findings.md   (Task 4)
```

---

# Phase 0 — Feasibility spike (throwaway)

Phase 0 code lives on a local branch `spike/linux-phase0` under `spikes/`. It is **never merged**. Only the findings document, spec updates and the captured `j/devices` fixture are committed to `feat/linux-support`, in Task 4.

### Task 1: Spike branch and P1, hidraw pairing

**Files:**
- Create: `spikes/50-zsa.rules` (spike branch only)
- Create: `spikes/notes.md` (spike branch only; raw observations for Task 4)

**Interfaces:**
- Consumes: the existing `cargo run -p oryx-device --example watch`.
- Produces: P1 answers in `spikes/notes.md`.

- [ ] **Step 1: Create the spike branch**

```bash
git switch feat/linux-support
git switch -c spike/linux-phase0
mkdir -p spikes
```

Expected: `git branch --show-current` prints `spike/linux-phase0`.

- [ ] **Step 2: Write the udev rule**

`spikes/50-zsa.rules`:

```
# ZSA keyboards: give the logged-in user access to their raw-HID nodes.
KERNEL=="hidraw*", ATTRS{idVendor}=="3297", TAG+="uaccess"
```

- [ ] **Step 3: [HUMAN] Install the rule and plug in the Voyager**

Ask your human partner to run:

```
! sudo install -m644 spikes/50-zsa.rules /etc/udev/rules.d/50-zsa.rules && sudo udevadm control --reload && sudo udevadm trigger
```

Then ask them to plug in the Voyager (or replug it).

- [ ] **Step 4: Check that the user can open the node**

```bash
lsusb | grep -i 3297
for n in /sys/class/hidraw/hidraw*; do
  dev=/dev/$(basename "$n")
  if udevadm info -q property -n "$dev" | grep -q 'ID_VENDOR_ID=3297'; then
    echo "$dev"; test -r "$dev" -a -w "$dev" && echo "  readable and writable by this user"
  fi
done
```

Expected: the Voyager is listed, and at least one of its hidraw nodes is readable and writable. Record the node count (not the username) in `spikes/notes.md`.

- [ ] **Step 5: Run the watch example**

Run: `cargo run -p oryx-device --example watch`

Ask your human partner to switch layers and press a few keys, then stop it with Ctrl+C.

Expected: `Connected { … }`, then `Layer(…)` on layer changes and `KeyDown`/`KeyUp` on key presses. Record in `spikes/notes.md` whether pairing worked, which events appeared, and the reported model and layout IDs. Don't paste key events: they are keystrokes.

If it prints an `OpenFailed` reason, fix the permission (Step 3) and retry. If it keeps waiting while the Voyager is plugged in, list what hidapi reports. Create `spikes/hid-list/Cargo.toml`:

```toml
[package]
name = "hid-list"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
hidapi = "2.6"
```

and `spikes/hid-list/src/main.rs`:

```rust
fn main() {
    let api = hidapi::HidApi::new().expect("hidapi");
    for d in api.device_list().filter(|d| d.vendor_id() == 0x3297) {
        println!(
            "{:04x}:{:04x} usage_page={:#06x} usage={:#04x} interface={} {}",
            d.vendor_id(),
            d.product_id(),
            d.usage_page(),
            d.usage(),
            d.interface_number(),
            d.path().to_string_lossy()
        );
    }
}
```

Run: `cargo run --manifest-path spikes/hid-list/Cargo.toml`

If no entry shows `usage_page=0xff60 usage=0x61`, P1 fails. Stop and report the output to your human partner: `Model::from_interface` would need an interface-number match on Linux, which is a design change.

- [ ] **Step 6: [HUMAN] Replug and suspend**

With the watch example running:
1. Unplug the Voyager and plug it back in. Expected: a disconnect event, then `Connected` within about 2 s.
2. Suspend (`systemctl suspend`), wake, and switch a layer. Expected: events resume without restarting the example (the 5 s heartbeat pairs again).

Record both outcomes in `spikes/notes.md`.

- [ ] **Step 7: Commit on the spike branch**

```bash
git add spikes/
git commit -m "spike(linux): P1 hidraw pairing notes

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 2: P2, layout switching reaches typing

**Files:**
- Modify: `spikes/notes.md`
- Create: `spikes/hyprland-devices.json` (spike branch; copied to the fixture in Task 4)

**Interfaces:**
- Consumes: the Voyager plugged in, Hyprland running.
- Produces: P2 answers; `spikes/hyprland-devices.json`.

- [ ] **Step 1: [HUMAN] Add `us` as the fourth layout**

Ask your human partner to edit `kb_layout` and `kb_variant` in `~/.config/hypr/input.lua`:

```lua
    kb_layout = "zd,hu,ro,us",
    kb_variant = ",102_qwerty_dot_nodead,std,",
```

Hyprland reloads its config by itself on save. Verify:

```bash
hyprctl -j devices | jq -r '.keyboards[] | "\(.name)\t\(.layout)\t\(.active_layout_index)"'
```

Expected: every keyboard shows `zd,hu,ro,us`, including the Voyager's keyboards and `hl-virtual-keyboard-fcitx5`.

- [ ] **Step 2: Capture the device list**

```bash
hyprctl -j devices > spikes/hyprland-devices.json
grep -c '"name"' spikes/hyprland-devices.json
grep -n '/home/' spikes/hyprland-devices.json || echo "no home paths"
```

Expected: `no home paths`.

- [ ] **Step 3: Switch every keyboard to `us` the way Downlink will**

```bash
hyprctl --batch "$(hyprctl -j devices | jq -r '[.keyboards[] | select(.layout == "zd,hu,ro,us") | "switchxkblayout \(.name) 3"] | join(" ; ")')"
hyprctl -j devices | jq -r '.keyboards[] | "\(.name)\t\(.active_layout_index)\t\(.active_keymap)"'
```

Expected: one `ok` per keyboard; every keyboard at index 3, "English (US)".

- [ ] **Step 4: [HUMAN] Check that typing follows**

ZD and US differ only on AltGr: on ZD, AltGr+z types `á`; on US it doesn't. Ask your human partner to press AltGr+z, and the Voyager's Hungarian-layer key for `á`, in:
1. a GTK app (for example `gnome-text-editor` or Firefox's address bar),
2. a Qt app (if one is installed; otherwise skip and note it),
3. the terminal (Alacritty/Ghostty).

Expected on `us`: no `á` in any of them. Then switch back to `zd` (the same batch with index `0`) and repeat. Expected: `á` everywhere.

Also ask them to check that the bar's layout widget shows the switched layout, and that `SUPER + Y` continues from it.

If typing doesn't follow while `hyprctl` reports the switch, fcitx5 is holding its own keymap. That is the spec §9 fallback (a fcitx5 D-Bus backend). Stop and report it to your human partner as a design change.

Record the results per app in `spikes/notes.md`.

- [ ] **Step 5: Commit on the spike branch**

```bash
git add spikes/
git commit -m "spike(linux): P2 layout switching notes and device capture

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 3: P3 and P4, iced with a ksni tray, and window behaviour

**Files:**
- Create: `spikes/linux-tray/Cargo.toml`
- Create: `spikes/linux-tray/src/main.rs`
- Modify: `spikes/notes.md`

**Interfaces:**
- Consumes: nothing from the workspace.
- Produces: P3 and P4 answers.

- [ ] **Step 1: Write the spike crate**

`spikes/linux-tray/Cargo.toml`:

```toml
[package]
name = "linux-tray-spike"
version = "0.0.0"
edition = "2024"
publish = false

# Not part of the Downlink workspace.
[workspace]

[dependencies]
iced = { version = "0.14", features = ["tokio"] }
ksni = { version = "0.3", default-features = false, features = ["blocking", "async-io"] }
futures = "0.3"
```

`spikes/linux-tray/src/main.rs`:

```rust
//! P3/P4 spike: an iced daemon with a ksni tray, spawned and updated from inside `update`.
use std::sync::{Mutex, OnceLock};

use futures::channel::mpsc;
use futures::{SinkExt, Stream, StreamExt};
use iced::theme::Mode;
use iced::{Element, Subscription, Task, Theme, system, window};
use ksni::blocking::{Handle, TrayMethods};

type Clicks = (
    mpsc::UnboundedSender<&'static str>,
    Mutex<Option<mpsc::UnboundedReceiver<&'static str>>>,
);
static CLICKS: OnceLock<Clicks> = OnceLock::new();

fn clicks() -> &'static Clicks {
    CLICKS.get_or_init(|| {
        let (tx, rx) = mpsc::unbounded();
        (tx, Mutex::new(Some(rx)))
    })
}

struct SpikeTray {
    opens: u32,
}

impl ksni::Tray for SpikeTray {
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        "downlink-spike".into()
    }

    fn title(&self) -> String {
        "Downlink spike".into()
    }

    fn icon_name(&self) -> String {
        "input-keyboard".into()
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        vec![
            StandardItem {
                label: format!("Windows opened: {}", self.opens),
                enabled: false,
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: "Open Window".into(),
                activate: Box::new(|_| {
                    let _ = clicks().0.unbounded_send("open");
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|_| {
                    let _ = clicks().0.unbounded_send("quit");
                }),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn watcher_offline(&self, reason: ksni::OfflineReason) -> bool {
        eprintln!("watcher offline: {reason:?}");
        true
    }
}

struct Spike {
    tray: Option<Handle<SpikeTray>>,
    mode: Mode,
    opens: u32,
}

#[derive(Debug, Clone)]
enum Message {
    Init,
    Click(&'static str),
    Theme(Mode),
    Opened(window::Id),
    Closed(window::Id),
}

fn main() -> iced::Result {
    iced::daemon(
        || {
            (
                Spike { tray: None, mode: Mode::None, opens: 0 },
                Task::batch([Task::done(Message::Init), system::theme().map(Message::Theme)]),
            )
        },
        update,
        view,
    )
    .theme(|spike: &Spike, _id| {
        Some(if spike.mode == Mode::Dark { Theme::Dark } else { Theme::Light })
    })
    .subscription(|_spike: &Spike| {
        Subscription::batch([
            Subscription::run(click_stream).map(Message::Click),
            system::theme_changes().map(Message::Theme),
            window::close_events().map(Message::Closed),
        ])
    })
    .run()
}

fn update(spike: &mut Spike, message: Message) -> Task<Message> {
    match message {
        Message::Init => {
            // P3: spawned from inside iced's update, where iced has entered its tokio runtime.
            match (SpikeTray { opens: 0 }).assume_sni_available(true).spawn() {
                Ok(handle) => spike.tray = Some(handle),
                Err(error) => eprintln!("tray failed: {error}"),
            }
            Task::none()
        }
        Message::Click("open") => {
            spike.opens += 1;
            let opens = spike.opens;
            if let Some(tray) = &spike.tray {
                tray.update(move |tray| tray.opens = opens);
            }
            let (_id, open) = window::open(window::Settings {
                platform_specific: window::settings::PlatformSpecific {
                    application_id: "downlink".into(),
                    ..Default::default()
                },
                ..window::Settings::default()
            });
            open.map(Message::Opened)
        }
        Message::Click(_) => iced::exit(),
        Message::Theme(mode) => {
            eprintln!("system theme: {mode:?}");
            spike.mode = mode;
            Task::none()
        }
        Message::Opened(id) => {
            eprintln!("opened {id:?}");
            Task::none()
        }
        Message::Closed(id) => {
            eprintln!("closed {id:?}; the daemon keeps running");
            Task::none()
        }
    }
}

fn view(spike: &Spike, _id: window::Id) -> Element<'_, Message> {
    iced::widget::text(format!("System theme: {:?}", spike.mode))
        .size(24)
        .into()
}

fn click_stream() -> impl Stream<Item = &'static str> {
    iced::stream::channel(16, async |mut output: mpsc::Sender<&'static str>| {
        let Some(mut rx) = clicks().1.lock().unwrap().take() else {
            return;
        };
        while let Some(click) = rx.next().await {
            if output.send(click).await.is_err() {
                break;
            }
        }
    })
}
```

- [ ] **Step 2: Build it**

Run: `cargo build --manifest-path spikes/linux-tray/Cargo.toml`

Expected: builds. If `iced::daemon`'s closure signatures differ from the above, adjust them until it builds. This is throwaway code.

- [ ] **Step 3: [HUMAN] P3, the tray**

Run: `cargo run --manifest-path spikes/linux-tray/Cargo.toml`

Ask your human partner to check:
1. A keyboard icon appears in the bar's tray area.
2. A left click opens the menu. "Windows opened: 0" is greyed out.
3. "Open Window" opens a window, and after reopening the menu it shows "Windows opened: 1". That proves `handle.update` works from inside iced's `update` without a tokio panic.
4. `omarchy-restart-shell` (or switching the Omarchy theme): the icon comes back by itself.

Record each in `spikes/notes.md`, with any stderr lines.

- [ ] **Step 4: [HUMAN] P4, windows and appearance**

With the spike still running:
1. Open a window, focus it and press Meta+W. Expected: stderr prints "closed …; the daemon keeps running", the process stays alive (`pgrep -a linux-tray-spike`), and the tray still works.
2. With a window open: `hyprctl -j clients | jq -r '.[] | select(.class == "downlink") | .class'`. Expected: prints `downlink`.
3. Close all windows, then run `gsettings set org.gnome.desktop.interface color-scheme prefer-dark`. Expected: stderr prints "system theme: Dark" with no window open. Open a window: it is dark. Run `gsettings set org.gnome.desktop.interface color-scheme prefer-light` with the window open: it turns light live.
4. Restore the original value: `gsettings set org.gnome.desktop.interface color-scheme prefer-light`.

Record each in `spikes/notes.md`. Item 3 without a window decides whether the tray icon can follow the appearance (spec §7). If no message arrives without a window, report it: the tray would then need the appearance seeded when a window opens, and that goes back into the spec.

- [ ] **Step 5: Commit on the spike branch**

```bash
git add spikes/
git commit -m "spike(linux): P3/P4 iced + ksni tray and window notes

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 4: Findings, spec updates and the fixture

**Files:**
- Create: `docs/superpowers/spikes/linux-phase0-findings.md`
- Create: `crates/input-source/tests/fixtures/hyprland-devices.json`
- Modify: `docs/superpowers/specs/2026-10-15-linux-support-design.md` (only where findings change it)

**Interfaces:**
- Consumes: `spikes/notes.md` and `spikes/hyprland-devices.json` from the spike branch.
- Produces: the fixture used by Task 8's `parses_the_captured_reply` test.

- [ ] **Step 1: Back to the feature branch, bringing the capture along**

```bash
git switch feat/linux-support
mkdir -p crates/input-source/tests/fixtures
git show spike/linux-phase0:spikes/hyprland-devices.json > crates/input-source/tests/fixtures/hyprland-devices.json
git show spike/linux-phase0:spikes/notes.md
```

- [ ] **Step 2: Write the findings**

`docs/superpowers/spikes/linux-phase0-findings.md` has one section per question (P1–P4). Each gives the question, what was run, what was observed and the answer, taken from `spikes/notes.md`. End with "Spec changes" (list them, or "None"). No usernames, no keystrokes.

- [ ] **Step 3: Update the spec where an answer changes it**

Edit the spec only for answers that differ from it, for example an extra keyboard name prefix seen in the capture. If an answer triggers a fallback named in spec §9 (P1 usage page, P2 fcitx5, P4 appearance without a window), stop. Show your human partner the finding and wait for their decision before going on to Phase 1.

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/spikes/linux-phase0-findings.md crates/input-source/tests/fixtures/hyprland-devices.json docs/superpowers/specs/2026-10-15-linux-support-design.md
git commit -m "docs: Linux Phase 0 findings and a captured Hyprland device list

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

# Phase 1 — Linux port

### Task 5: Split the tray module; `downlink` builds on Linux

The macOS-only `tray-icon` template-icon calls stop `downlink` compiling on Linux today. This task moves them into `tray/macos.rs` and gives Linux a stub `Tray`, which Task 7 replaces.

**Files:**
- Delete: `crates/downlink/src/tray.rs`
- Create: `crates/downlink/src/tray/mod.rs`
- Create: `crates/downlink/src/tray/macos.rs`
- Create: `crates/downlink/src/tray/linux.rs`
- Modify: `crates/downlink/Cargo.toml`
- Modify: `crates/downlink/src/app.rs` (`refresh_tray`, `Message::SystemTheme`)
- Test: `crates/downlink/tests/menu_action.rs` (unchanged; must still pass)

**Interfaces:**
- Produces:
  - `tray::{SHOW_LAYOUT, SETTINGS, PAUSE, QUIT, APP_SETTINGS, CLOSE_WINDOW, CLOSE_WINDOW_Q}: &str`
  - `tray::MenuAction`, `tray::menu_action(&str) -> Option<MenuAction>`
  - `tray::menu_events() -> impl Stream<Item = String>`
  - `pub(crate) fn tray::menu_sender() -> &'static futures::channel::mpsc::UnboundedSender<String>`
  - `tray::install_event_handlers()`, `tray::AppMenu::install() -> Result<AppMenu, String>`
  - `tray::Tray::new() -> Result<Tray, String>`, `tray::Tray::apply(&mut self, view: &TrayView, dark: bool)`

- [ ] **Step 1: See the failure**

Run: `cargo build -p downlink 2>&1 | grep -E '^error' | head`

Expected: errors for `with_icon_templated` and `set_icon_templated` in `src/tray.rs`.

- [ ] **Step 2: Move `tray-icon` to the macOS dependencies**

In `crates/downlink/Cargo.toml`, remove `tray-icon = "0.26"` from `[dependencies]` and add it to the macOS table:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.6"
objc2-app-kit = { version = "0.3", features = ["NSAppearance", "NSApplication", "NSResponder", "NSRunningApplication"] }
objc2-foundation = { version = "0.3", features = ["NSArray", "NSString"] }
tray-icon = "0.26"
```

- [ ] **Step 3: Write `tray/mod.rs` (the shared part)**

```bash
git mv crates/downlink/src/tray.rs crates/downlink/src/tray/macos.rs
```

`crates/downlink/src/tray/mod.rs`:

```rust
//! The tray item (spec §6.1): menu IDs and the click channel shared by both OSes, with one
//! implementation per OS.

use std::sync::{Mutex, OnceLock, PoisonError};

use futures::channel::mpsc;
use futures::{SinkExt, Stream, StreamExt};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{AppMenu, Tray, install_event_handlers};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{AppMenu, Tray, install_event_handlers};

pub const SHOW_LAYOUT: &str = "show-layout";
pub const SETTINGS: &str = "settings";
pub const PAUSE: &str = "pause";
pub const QUIT: &str = "quit";
/// App-menu items, active while a Downlink window is focused (macOS only).
pub const APP_SETTINGS: &str = "app-settings";
pub const CLOSE_WINDOW: &str = "app-close-window";
pub const CLOSE_WINDOW_Q: &str = "app-close-window-q";

/// What a clicked menu item (or its shortcut) asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    ShowLayout,
    ShowSettings,
    TogglePause,
    Quit,
    CloseWindow,
}

/// Only the menu-bar "Quit Downlink" quits; ⌘W and ⌘Q in a window close it (Phase 0, option B).
pub fn menu_action(id: &str) -> Option<MenuAction> {
    match id {
        SHOW_LAYOUT => Some(MenuAction::ShowLayout),
        SETTINGS | APP_SETTINGS => Some(MenuAction::ShowSettings),
        PAUSE => Some(MenuAction::TogglePause),
        QUIT => Some(MenuAction::Quit),
        CLOSE_WINDOW | CLOSE_WINDOW_Q => Some(MenuAction::CloseWindow),
        _ => None,
    }
}

/// The process-wide channel that carries clicked menu item IDs from the tray to the
/// `menu_events` stream: the sender, and the receiver until the stream takes it.
type MenuChannel = (
    mpsc::UnboundedSender<String>,
    Mutex<Option<mpsc::UnboundedReceiver<String>>>,
);

static MENU_CHANNEL: OnceLock<MenuChannel> = OnceLock::new();

fn menu_channel() -> &'static MenuChannel {
    MENU_CHANNEL.get_or_init(|| {
        let (tx, rx) = mpsc::unbounded();
        (tx, Mutex::new(Some(rx)))
    })
}

/// Where a tray implementation sends the ID of a clicked item.
pub(crate) fn menu_sender() -> &'static mpsc::UnboundedSender<String> {
    &menu_channel().0
}

/// Menu item IDs as they are clicked.
pub fn menu_events() -> impl Stream<Item = String> {
    iced::stream::channel(16, async |mut output: mpsc::Sender<String>| {
        let receiver = menu_channel()
            .1
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        let Some(mut rx) = receiver else {
            tracing::error!("the menu event stream was started twice; menu clicks are not handled");
            return;
        };
        while let Some(id) = rx.next().await {
            if output.send(id).await.is_err() {
                break;
            }
        }
    })
}
```

- [ ] **Step 4: Trim `tray/macos.rs` to the macOS part**

`crates/downlink/src/tray/macos.rs` becomes exactly this. It is the old `tray.rs` minus the shared items, with `apply` taking `dark` and the handler using `menu_sender()`:

```rust
//! The menu-bar item and app menu on macOS (spec §6.1): tray-icon and muda.

use companion_core::TrayView;
use tray_icon::menu::accelerator::Accelerator;
use tray_icon::menu::{
    CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

use super::{
    APP_SETTINGS, CLOSE_WINDOW, CLOSE_WINDOW_Q, PAUSE, QUIT, SETTINGS, SHOW_LAYOUT, menu_sender,
};
use crate::icon;

/// The app's main menu. On macOS it replaces winit's default menu, whose ⌘Q item would call
/// `NSApp terminate:` and end the process outside Downlink's Quit path. An accessory app never
/// shows this menu, but macOS still routes its shortcuts while a Downlink window is focused.
pub struct AppMenu {
    _menu: Menu,
}

impl AppMenu {
    /// Call from `update` (main thread, event loop running), like `Tray::new`.
    pub fn install() -> Result<AppMenu, String> {
        let menu = Menu::new();
        let app = Submenu::new("Downlink", true);
        for item in [
            &MenuItem::with_id(APP_SETTINGS, "Settings…", true, accelerator("CmdOrCtrl+,"))
                as &dyn IsMenuItem,
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id(
                CLOSE_WINDOW,
                "Close Window",
                true,
                accelerator("CmdOrCtrl+W"),
            ),
            &MenuItem::with_id(
                CLOSE_WINDOW_Q,
                "Close Window",
                true,
                accelerator("CmdOrCtrl+Q"),
            ),
        ] {
            app.append(item).map_err(|e| e.to_string())?;
        }
        menu.append(&app).map_err(|e| e.to_string())?;
        menu.init_for_nsapp();
        Ok(AppMenu { _menu: menu })
    }
}

/// 18 pt at 2× scale.
const ICON_PX: u32 = 36;

pub struct Tray {
    icon: TrayIcon,
    status: MenuItem,
    detail: MenuItem,
    pause: CheckMenuItem,
    connected: bool,
}

impl Tray {
    /// Must run on the main thread after the event loop starts: call it from `update`.
    pub fn new() -> Result<Tray, String> {
        let status = MenuItem::new("Waiting for Voyager…", false, None);
        let detail = MenuItem::new(" ", false, None);
        let show = MenuItem::with_id(SHOW_LAYOUT, "Show Layout", true, accelerator("CmdOrCtrl+L"));
        let settings = MenuItem::with_id(SETTINGS, "Settings…", true, accelerator("CmdOrCtrl+,"));
        let pause = CheckMenuItem::with_id(PAUSE, "Pause Input Switching", true, false, None);
        let quit = MenuItem::with_id(QUIT, "Quit Downlink", true, accelerator("CmdOrCtrl+Q"));
        let menu = Menu::new();
        for item in [
            &status as &dyn IsMenuItem,
            &detail,
            &PredefinedMenuItem::separator(),
            &show,
            &settings,
            &PredefinedMenuItem::separator(),
            &pause,
            &PredefinedMenuItem::separator(),
            &quit,
        ] {
            menu.append(item).map_err(|e| e.to_string())?;
        }
        let icon = TrayIconBuilder::new()
            .with_icon_templated(tray_icon_image(true)?)
            .with_menu(Box::new(menu))
            .with_tooltip("Downlink")
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Tray {
            icon,
            status,
            detail,
            pause,
            connected: true,
        })
    }

    /// `_dark` is unused: macOS recolours the template icon for the menu bar itself.
    pub fn apply(&mut self, view: &TrayView, _dark: bool) {
        self.status.set_text(&view.status);
        self.detail.set_text(if view.detail.is_empty() {
            " "
        } else {
            view.detail.as_str()
        });
        self.pause.set_checked(view.paused);
        if view.connected != self.connected {
            match tray_icon_image(view.connected) {
                Ok(image) => {
                    if let Err(error) = self.icon.set_icon_templated(Some(image)) {
                        tracing::warn!(%error, "could not update the menu-bar icon");
                    }
                }
                Err(error) => tracing::warn!(%error, "could not render the menu-bar icon"),
            }
            self.connected = view.connected;
        }
    }
}

fn accelerator(spec: &str) -> Option<Accelerator> {
    spec.parse().ok()
}

fn tray_icon_image(connected: bool) -> Result<Icon, String> {
    Icon::from_rgba(icon::menubar_rgba(ICON_PX, connected), ICON_PX, ICON_PX)
        .map_err(|e| e.to_string())
}

/// Register both event handlers. Call it once, first thing in `App::init`, before any menu or
/// tray icon exists.
///
/// tray-icon and muda keep their handler in a write-once cell. An event that arrives while no
/// handler is set fills the cell with "no handler" for good, so a handler registered later from
/// inside a subscription future could be silently ignored. With no handler, tray-icon also
/// queues every Enter/Move/Leave/Click over the status item in a global unbounded channel that
/// nothing reads, so the memory only grows.
pub fn install_event_handlers() {
    // Downlink doesn't use tray events; the handler only stops them piling up.
    TrayIconEvent::set_event_handler(Some(|_event: TrayIconEvent| {}));
    let tx = menu_sender().clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let _ = tx.unbounded_send(event.id.0);
    }));
}
```

Compared with the old file, the only changes are:
- the module doc,
- the shared items moved to `mod.rs`,
- `#[cfg(target_os = "macos")]` dropped before `menu.init_for_nsapp()`, since the whole file is macOS-only now,
- `apply` takes `_dark`,
- `install_event_handlers` uses `menu_sender()`.

- [ ] **Step 5: Write the Linux stub**

`crates/downlink/src/tray/linux.rs`:

```rust
//! The tray item on Linux. Placeholder until the StatusNotifierItem tray lands.

use companion_core::TrayView;

/// Nothing to register: the Linux tray sends clicks straight into the menu channel.
pub fn install_event_handlers() {}

/// Linux has no app menu: the compositor closes windows (for example Meta+W).
pub struct AppMenu;

impl AppMenu {
    pub fn install() -> Result<AppMenu, String> {
        Ok(AppMenu)
    }
}

pub struct Tray;

impl Tray {
    pub fn new() -> Result<Tray, String> {
        Err("the Linux tray is not available yet".into())
    }

    pub fn apply(&mut self, _view: &TrayView, _dark: bool) {}
}
```

- [ ] **Step 6: Pass the appearance to the tray in `app.rs`**

In `crates/downlink/src/app.rs`, replace `refresh_tray`:

```rust
    fn refresh_tray(&mut self) {
        let view = self.engine.tray_view(|id| self.source_name(id));
        let dark = self.is_dark();
        if let Some(tray) = self.tray.as_mut() {
            tray.apply(&view, dark);
        }
    }
```

and in `update`, make `Message::SystemTheme` refresh the tray after storing the mode:

```rust
            Message::SystemTheme(mode) => {
                // iced reports `None` before any window exists (Phase 0); keep the seeded mode then.
                // It also stops reporting changes once it pins a window's appearance, so the poll
                // re-seeds the mode too.
                if mode != Mode::None {
                    self.system_mode = mode;
                    // The Linux tray icon is drawn for the appearance.
                    self.refresh_tray();
                }
                Task::none()
            }
```

- [ ] **Step 7: Build and run the tests**

Run: `cargo build -p downlink && cargo test --workspace`

Expected: builds on Linux; all tests pass, including `menu_action.rs` unchanged.

- [ ] **Step 8: fmt and clippy**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`

Expected: clean. A dead-code warning in `linux.rs` is not acceptable; every item there is `pub`.

- [ ] **Step 9: Commit**

```bash
git add -A crates/downlink
git commit -m "refactor(tray): split the tray per OS so downlink builds on Linux

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 6: The tray menu as data, and the ARGB tray icon

**Files:**
- Create: `crates/downlink/src/tray/menu_model.rs`
- Modify: `crates/downlink/src/tray/mod.rs` (add `pub mod menu_model;`)
- Modify: `crates/downlink/src/icon.rs`
- Test: `crates/downlink/tests/tray_menu.rs`
- Test: `crates/downlink/tests/icon.rs`

**Interfaces:**
- Consumes: `companion_core::TrayView { connected: bool, status: String, detail: String, paused: bool }`; `tray::{SHOW_LAYOUT, SETTINGS, PAUSE, QUIT}`.
- Produces:
  - `tray::menu_model::MenuEntry` (`Label(String)`, `Item { id: &'static str, label: &'static str }`, `Check { id: &'static str, label: &'static str, checked: bool }`, `Separator`)
  - `tray::menu_model::menu_entries(&TrayView) -> Vec<MenuEntry>`
  - `icon::menubar_argb(size: u32, connected: bool, rgb: [u8; 3]) -> Vec<u8>`

- [ ] **Step 1: Write the failing menu tests**

`crates/downlink/tests/tray_menu.rs`:

```rust
use companion_core::TrayView;
use downlink::tray::menu_model::{MenuEntry, menu_entries};
use downlink::tray::{MenuAction, PAUSE, QUIT, SETTINGS, SHOW_LAYOUT, menu_action};

fn view(connected: bool, paused: bool) -> TrayView {
    TrayView {
        connected,
        status: if connected {
            "● Voyager connected".into()
        } else {
            "Waiting for Voyager…".into()
        },
        detail: if connected {
            "Layer: Main · Input: English (US)".into()
        } else {
            String::new()
        },
        paused,
    }
}

#[test]
fn a_connected_menu_matches_the_macos_one() {
    assert_eq!(
        menu_entries(&view(true, false)),
        vec![
            MenuEntry::Label("● Voyager connected".into()),
            MenuEntry::Label("Layer: Main · Input: English (US)".into()),
            MenuEntry::Separator,
            MenuEntry::Item { id: SHOW_LAYOUT, label: "Show Layout" },
            MenuEntry::Item { id: SETTINGS, label: "Settings…" },
            MenuEntry::Separator,
            MenuEntry::Check { id: PAUSE, label: "Pause Input Switching", checked: false },
            MenuEntry::Separator,
            MenuEntry::Item { id: QUIT, label: "Quit Downlink" },
        ]
    );
}

#[test]
fn a_disconnected_menu_has_no_detail_row() {
    let entries = menu_entries(&view(false, false));
    assert_eq!(entries[0], MenuEntry::Label("Waiting for Voyager…".into()));
    assert_eq!(entries[1], MenuEntry::Separator);
    assert_eq!(
        entries.iter().filter(|e| matches!(e, MenuEntry::Label(_))).count(),
        1
    );
}

#[test]
fn pausing_checks_the_pause_item() {
    assert!(menu_entries(&view(true, true)).contains(&MenuEntry::Check {
        id: PAUSE,
        label: "Pause Input Switching",
        checked: true,
    }));
}

#[test]
fn every_clickable_entry_maps_to_an_action_and_only_quit_quits() {
    for entry in menu_entries(&view(true, false)) {
        let id = match entry {
            MenuEntry::Item { id, .. } | MenuEntry::Check { id, .. } => id,
            MenuEntry::Label(_) | MenuEntry::Separator => continue,
        };
        let action = menu_action(id).unwrap_or_else(|| panic!("{id} has no action"));
        assert_eq!(action == MenuAction::Quit, id == QUIT, "{id}");
    }
}
```

- [ ] **Step 2: Write the failing icon tests**

Append to `crates/downlink/tests/icon.rs`, and change its first line to import the new function:

```rust
use downlink::icon::{app_icon_rgba, menubar_argb, menubar_rgba};
```

```rust
#[test]
fn tray_pixmap_is_argb_in_the_requested_colour() {
    let rgba = menubar_rgba(22, true);
    let argb = menubar_argb(22, true, [0xe8, 0xea, 0xee]);
    assert_eq!(argb.len(), rgba.len());
    for (a, r) in argb.chunks(4).zip(rgba.chunks(4)) {
        assert_eq!(a[0], r[3], "alpha comes first, unchanged");
        assert_eq!(&a[1..], &[0xe8, 0xea, 0xee], "every pixel has the glyph colour");
    }
}

#[test]
fn disconnected_tray_pixmap_is_dimmed() {
    let argb = menubar_argb(36, false, [0xff, 0xff, 0xff]);
    let alpha = argb[((13 * 36 + 18) * 4) as usize];
    assert!(alpha > 60 && alpha < 120, "dimmed alpha was {alpha}");
}
```

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test -p downlink --test tray_menu --test icon`

Expected: compile errors: no `menu_model` module, no `menubar_argb`.

- [ ] **Step 4: Implement `menu_model.rs`**

Add `pub mod menu_model;` to `crates/downlink/src/tray/mod.rs`, under the `use` lines.

`crates/downlink/src/tray/menu_model.rs`:

```rust
//! The tray menu as data, built from `TrayView`. The Linux tray turns it into DBusMenu items.

use companion_core::TrayView;

use super::{PAUSE, QUIT, SETTINGS, SHOW_LAYOUT};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntry {
    /// A greyed-out line of text: the status and the detail.
    Label(String),
    Item {
        id: &'static str,
        label: &'static str,
    },
    Check {
        id: &'static str,
        label: &'static str,
        checked: bool,
    },
    Separator,
}

/// The same items as the macOS menu (spec §7). The detail row is left out while it's empty.
pub fn menu_entries(view: &TrayView) -> Vec<MenuEntry> {
    let mut entries = vec![MenuEntry::Label(view.status.clone())];
    if !view.detail.is_empty() {
        entries.push(MenuEntry::Label(view.detail.clone()));
    }
    entries.extend([
        MenuEntry::Separator,
        MenuEntry::Item { id: SHOW_LAYOUT, label: "Show Layout" },
        MenuEntry::Item { id: SETTINGS, label: "Settings…" },
        MenuEntry::Separator,
        MenuEntry::Check { id: PAUSE, label: "Pause Input Switching", checked: view.paused },
        MenuEntry::Separator,
        MenuEntry::Item { id: QUIT, label: "Quit Downlink" },
    ]);
    entries
}
```

- [ ] **Step 5: Implement `menubar_argb`**

Add to `crates/downlink/src/icon.rs`, after `menubar_rgba`:

```rust
/// The menu-bar glyph in one solid colour, as ARGB32 in network byte order: a StatusNotifierItem
/// pixmap. SNI has no template images, so the caller picks the colour for the appearance.
pub fn menubar_argb(size: u32, connected: bool, rgb: [u8; 3]) -> Vec<u8> {
    menubar_rgba(size, connected)
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|pixel| [pixel[3], rgb[0], rgb[1], rgb[2]])
        .collect()
}
```

- [ ] **Step 6: Run the tests**

Run: `cargo test -p downlink --test tray_menu --test icon`

Expected: PASS.

- [ ] **Step 7: Full check and commit**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

```bash
git add crates/downlink
git commit -m "feat(tray): the tray menu as data and an ARGB tray icon

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 7: The `ksni` tray on Linux

**Files:**
- Modify: `crates/downlink/Cargo.toml`
- Modify: `crates/downlink/src/tray/linux.rs` (replace the stub)
- Test: `crates/downlink/tests/tray_menu.rs` (one label-escaping test)

**Interfaces:**
- Consumes: `menu_model::{MenuEntry, menu_entries}`, `icon::menubar_argb`, `tray::menu_sender()`.
- Produces: `tray::Tray::new()`, `tray::Tray::apply(&TrayView, bool)` on Linux; `tray::menu_model::dbusmenu_label(&str) -> String`.

- [ ] **Step 1: Write the failing label test**

Append to `crates/downlink/tests/tray_menu.rs`:

```rust
use downlink::tray::menu_model::dbusmenu_label;

#[test]
fn underscores_in_labels_are_not_mnemonics() {
    assert_eq!(
        dbusmenu_label("Layer: Sym_L · Input: us"),
        "Layer: Sym__L · Input: us"
    );
    assert_eq!(dbusmenu_label("Settings…"), "Settings…");
}
```

Run: `cargo test -p downlink --test tray_menu`

Expected: FAIL (no `dbusmenu_label`).

- [ ] **Step 2: Implement `dbusmenu_label`**

Append to `crates/downlink/src/tray/menu_model.rs`:

```rust
/// DBusMenu reads `_` as a mnemonic marker; a literal underscore is written twice.
pub fn dbusmenu_label(text: &str) -> String {
    text.replace('_', "__")
}
```

Run: `cargo test -p downlink --test tray_menu`

Expected: PASS.

- [ ] **Step 3: Add `ksni`**

In `crates/downlink/Cargo.toml`, add:

```toml
[target.'cfg(target_os = "linux")'.dependencies]
# async-io, not the default tokio: ksni's blocking calls would start a tokio runtime inside the
# one iced runs `update` in, and tokio panics on that (spec §5).
ksni = { version = "0.3", default-features = false, features = ["blocking", "async-io"] }
```

- [ ] **Step 4: Replace the stub with the tray**

`crates/downlink/src/tray/linux.rs`:

```rust
//! The tray item on Linux (spec §7): a StatusNotifierItem over D-Bus, shown by Omarchy's bar.

use companion_core::TrayView;
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{CheckmarkItem, StandardItem};
use ksni::{Icon, MenuItem, OfflineReason, ToolTip};

use super::menu_model::{MenuEntry, dbusmenu_label, menu_entries};
use super::menu_sender;
use crate::icon;

/// Glyph colours: light on a dark appearance, dark on a light one (the palettes' key text).
const ON_DARK: [u8; 3] = [0xe8, 0xea, 0xee];
const ON_LIGHT: [u8; 3] = [0x1d, 0x20, 0x25];
/// One logical size at 1× and 2×; the host picks the closest.
const ICON_SIZES: [u32; 2] = [22, 44];

/// Nothing to register: menu items send clicks straight into the menu channel.
pub fn install_event_handlers() {}

/// Linux has no app menu: the compositor closes windows (for example Meta+W).
pub struct AppMenu;

impl AppMenu {
    pub fn install() -> Result<AppMenu, String> {
        Ok(AppMenu)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct State {
    view: TrayView,
    dark: bool,
}

struct DownlinkTray {
    state: State,
}

impl ksni::Tray for DownlinkTray {
    /// A left click opens the menu, as on macOS.
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        "downlink".into()
    }

    fn title(&self) -> String {
        "Downlink".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        let rgb = if self.state.dark { ON_DARK } else { ON_LIGHT };
        ICON_SIZES
            .iter()
            .map(|&px| Icon {
                width: px as i32,
                height: px as i32,
                data: icon::menubar_argb(px, self.state.view.connected, rgb),
            })
            .collect()
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: "Downlink".into(),
            description: self.state.view.status.clone(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        menu_entries(&self.state.view)
            .into_iter()
            .map(to_ksni)
            .collect()
    }

    fn watcher_offline(&self, reason: OfflineReason) -> bool {
        tracing::warn!(?reason, "the tray host went away; the icon returns when it's back");
        true
    }
}

fn to_ksni(entry: MenuEntry) -> MenuItem<DownlinkTray> {
    match entry {
        MenuEntry::Label(text) => StandardItem {
            label: dbusmenu_label(&text),
            enabled: false,
            ..Default::default()
        }
        .into(),
        MenuEntry::Item { id, label } => StandardItem {
            label: label.into(),
            activate: Box::new(move |_: &mut DownlinkTray| send(id)),
            ..Default::default()
        }
        .into(),
        MenuEntry::Check { id, label, checked } => CheckmarkItem {
            label: label.into(),
            checked,
            activate: Box::new(move |_: &mut DownlinkTray| send(id)),
            ..Default::default()
        }
        .into(),
        MenuEntry::Separator => MenuItem::Separator,
    }
}

fn send(id: &str) {
    let _ = menu_sender().unbounded_send(id.to_owned());
}

pub struct Tray {
    handle: Handle<DownlinkTray>,
    state: State,
}

impl Tray {
    /// Spawns the D-Bus service on its own thread. With `assume_sni_available`, a bar that
    /// isn't up yet (autostart racing it at login) is waited for instead of failing.
    pub fn new() -> Result<Tray, String> {
        let state = State {
            view: TrayView {
                connected: false,
                status: "Waiting for Voyager…".into(),
                detail: String::new(),
                paused: false,
            },
            dark: false,
        };
        let handle = DownlinkTray {
            state: state.clone(),
        }
        .assume_sni_available(true)
        .spawn()
        .map_err(|e| e.to_string())?;
        Ok(Tray { handle, state })
    }

    pub fn apply(&mut self, view: &TrayView, dark: bool) {
        let state = State {
            view: view.clone(),
            dark,
        };
        if state == self.state {
            return;
        }
        self.state = state.clone();
        if self.handle.update(move |tray| tray.state = state).is_none() {
            tracing::warn!("the tray service stopped; the menu is no longer updated");
        }
    }
}
```

- [ ] **Step 5: Build, test, fmt, clippy**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

Expected: clean and passing.

- [ ] **Step 6: [HUMAN] Try it in the bar**

Run: `DOWNLINK_LOG=debug cargo run -p downlink`

Ask your human partner to check:
1. The Downlink key icon appears in the bar.
2. A left click opens the menu, with the status row and, when the Voyager is connected, the detail row.
3. Show Layout and Settings… open their windows.
4. Pause Input Switching toggles its checkmark.
5. Quit Downlink quits.
6. Switching the Omarchy theme between a light and a dark one changes the glyph colour.

Input switching does nothing yet; that comes in Task 11. Record any failure and fix it before committing.

- [ ] **Step 7: Commit**

```bash
git add crates/downlink Cargo.lock
git commit -m "feat(tray): StatusNotifierItem tray on Linux with ksni

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 8: Hyprland device list and the switching decisions

**Files:**
- Modify: `crates/input-source/Cargo.toml`
- Modify: `crates/input-source/src/lib.rs` (declare `pub mod hyprland` on Linux)
- Create: `crates/input-source/src/hyprland/mod.rs` (module declarations only, for now)
- Create: `crates/input-source/src/hyprland/devices.rs`
- Test: `crates/input-source/tests/hyprland_devices.rs`

**Interfaces:**
- Consumes: `InputSource { id: String, name: String }`, `InputSourceError::{NotFound, Platform}`.
- Produces (module `input_source::hyprland::devices`):
  - `pub struct Keyboard { pub name: String, pub rules: String, pub model: String, pub layout: String, pub variant: String, pub options: String, pub active_layout_index: usize, pub main: bool }`
  - `Keyboard::is_typed(&self) -> bool`, `Keyboard::layout_ids(&self) -> Vec<String>`
  - `parse_keyboards(json: &str) -> Result<Vec<Keyboard>, InputSourceError>`
  - `layout_ids(layout: &str, variant: &str) -> Vec<String>`
  - `reading_keyboard(&[Keyboard]) -> Option<&Keyboard>`
  - `current_id(&[Keyboard]) -> Result<String, InputSourceError>`
  - `switch_commands(&[Keyboard], id: &str) -> Result<Vec<String>, InputSourceError>`
  - `batch(commands: &[String]) -> String`
  - `check_batch_reply(reply: &str, commands: usize) -> Result<(), InputSourceError>`
  - `sources(reader: &Keyboard, names: &[String]) -> Vec<InputSource>`
  - `no_keyboards() -> InputSourceError`

- [ ] **Step 1: Add the dependencies and module skeleton**

`crates/input-source/Cargo.toml`, add:

```toml
[target.'cfg(target_os = "linux")'.dependencies]
serde.workspace = true
serde_json.workspace = true
```

In `crates/input-source/src/lib.rs`, below `mod fake;`:

```rust
#[cfg(target_os = "linux")]
pub mod hyprland;
```

`crates/input-source/src/hyprland/mod.rs`:

```rust
//! Hyprland: switches the seat's XKB layout through Hyprland's IPC socket (spec §6).

pub mod devices;
```

- [ ] **Step 2: Write the failing tests**

`crates/input-source/tests/hyprland_devices.rs`:

```rust
#![cfg(target_os = "linux")]

use input_source::hyprland::devices::{
    Keyboard, batch, check_batch_reply, current_id, layout_ids, parse_keyboards,
    reading_keyboard, sources, switch_commands,
};
use input_source::{InputSource, InputSourceError};

const LAYOUT: &str = "zd,hu,ro,us";
const VARIANT: &str = ",102_qwerty_dot_nodead,std,";

fn kb(name: &str, index: usize) -> Keyboard {
    Keyboard {
        name: name.into(),
        rules: String::new(),
        model: String::new(),
        layout: LAYOUT.into(),
        variant: VARIANT.into(),
        options: "compose:ralt".into(),
        active_layout_index: index,
        main: false,
    }
}

fn seat(voyager_index: usize) -> Vec<Keyboard> {
    let mut virtual_kb = kb("hl-virtual-keyboard-fcitx5", 0);
    virtual_kb.main = true;
    vec![
        kb("power-button", 0),
        kb("video-bus", 0),
        kb("at-translated-set-2-keyboard", 0),
        kb("zsa-technology-labs-voyager", voyager_index),
        virtual_kb,
    ]
}

#[test]
fn parses_hyprlands_reply_ignoring_unknown_fields() {
    let json = r#"{"mice": [], "keyboards": [{
        "address": "0x55a8fc65b1a0", "name": "hl-virtual-keyboard-fcitx5",
        "rules": "", "model": "", "layout": "zd,hu,ro,us",
        "variant": ",102_qwerty_dot_nodead,std,", "options": "compose:ralt",
        "active_layout_index": 3, "active_keymap": "English (US)",
        "capsLock": false, "numLock": true, "main": true
    }], "tablets": [], "touch": [], "switches": []}"#;
    let keyboards = parse_keyboards(json).unwrap();
    assert_eq!(keyboards.len(), 1);
    assert_eq!(keyboards[0].name, "hl-virtual-keyboard-fcitx5");
    assert_eq!(keyboards[0].active_layout_index, 3);
    assert!(keyboards[0].main);
}

#[test]
fn parses_the_captured_reply() {
    let json = include_str!("fixtures/hyprland-devices.json");
    let keyboards = parse_keyboards(json).unwrap();
    assert!(keyboards.iter().any(|k| k.is_typed()));
    assert!(current_id(&keyboards).is_ok());
}

#[test]
fn a_garbled_reply_is_a_platform_error() {
    assert!(matches!(
        parse_keyboards("unknown request"),
        Err(InputSourceError::Platform(_))
    ));
}

#[test]
fn layout_ids_pair_layouts_with_variants_by_position() {
    assert_eq!(
        layout_ids(LAYOUT, VARIANT),
        ["zd", "hu(102_qwerty_dot_nodead)", "ro(std)", "us"]
    );
}

#[test]
fn a_short_or_empty_variant_list_leaves_the_rest_plain() {
    assert_eq!(layout_ids(LAYOUT, ""), ["zd", "hu", "ro", "us"]);
    assert_eq!(layout_ids("us,de", ",nodeadkeys,extra"), ["us", "de(nodeadkeys)"]);
    assert_eq!(layout_ids(" us , de ", ""), ["us", "de"]);
    assert!(layout_ids("", "").is_empty());
}

#[test]
fn untyped_keyboards_are_recognised_by_name() {
    for name in [
        "hl-virtual-keyboard-fcitx5",
        "power-button",
        "sleep-button",
        "lid-switch",
        "video-bus",
    ] {
        assert!(!kb(name, 0).is_typed(), "{name}");
    }
    assert!(kb("zsa-technology-labs-voyager", 0).is_typed());
}

#[test]
fn the_furthest_typed_keyboard_is_read() {
    assert_eq!(current_id(&seat(3)).unwrap(), "us");
    assert_eq!(
        reading_keyboard(&seat(3)).unwrap().name,
        "zsa-technology-labs-voyager"
    );
}

#[test]
fn untyped_keyboards_are_not_read_while_a_typed_one_exists() {
    let mut keyboards = seat(0);
    keyboards[0].active_layout_index = 3; // power-button drifted
    assert_eq!(current_id(&keyboards).unwrap(), "zd");
}

#[test]
fn without_typed_keyboards_the_main_one_is_read() {
    let mut virtual_kb = kb("hl-virtual-keyboard-fcitx5", 2);
    virtual_kb.main = true;
    let keyboards = vec![kb("power-button", 0), virtual_kb];
    assert_eq!(current_id(&keyboards).unwrap(), "ro(std)");
}

#[test]
fn no_keyboards_is_a_platform_error() {
    assert_eq!(
        current_id(&[]),
        Err(InputSourceError::Platform("no keyboards".into()))
    );
}

#[test]
fn an_out_of_range_index_is_a_platform_error() {
    let keyboards = vec![kb("at-translated-set-2-keyboard", 9)];
    assert!(matches!(current_id(&keyboards), Err(InputSourceError::Platform(_))));
}

#[test]
fn switching_covers_every_keyboard_with_the_same_list_untyped_included() {
    assert_eq!(
        switch_commands(&seat(0), "us").unwrap(),
        [
            "switchxkblayout power-button 3",
            "switchxkblayout video-bus 3",
            "switchxkblayout at-translated-set-2-keyboard 3",
            "switchxkblayout zsa-technology-labs-voyager 3",
            "switchxkblayout hl-virtual-keyboard-fcitx5 3",
        ]
    );
}

#[test]
fn keyboards_with_a_different_list_are_left_alone() {
    let mut keyboards = seat(0);
    keyboards[2].layout = "zd,hu,ro".into();
    let commands = switch_commands(&keyboards, "zd").unwrap();
    assert!(!commands.iter().any(|c| c.contains("at-translated-set-2-keyboard")));
    assert_eq!(commands.len(), 4);
}

#[test]
fn an_unknown_id_is_not_found() {
    assert_eq!(
        switch_commands(&seat(0), "de"),
        Err(InputSourceError::NotFound("de".into()))
    );
}

#[test]
fn commands_are_sent_as_one_batch() {
    let commands = ["switchxkblayout a 3".to_owned(), "switchxkblayout b 3".to_owned()];
    assert_eq!(
        batch(&commands),
        "[[BATCH]]switchxkblayout a 3 ; switchxkblayout b 3"
    );
}

#[test]
fn a_batch_succeeds_only_when_every_command_says_ok() {
    assert_eq!(check_batch_reply("ok\n\n\nok", 2), Ok(()));
    assert_eq!(check_batch_reply("ok", 1), Ok(()));
    let failed = check_batch_reply("ok\n\n\ndevice not found", 2).unwrap_err();
    assert!(
        matches!(&failed, InputSourceError::Platform(m) if m.contains("device not found")),
        "{failed:?}"
    );
    assert!(check_batch_reply("ok", 2).is_err(), "a missing reply");
    assert!(check_batch_reply("", 1).is_err(), "no reply at all");
}

#[test]
fn sources_are_named_by_index_deduplicated_and_sorted() {
    let mut reader = kb("at-translated-set-2-keyboard", 0);
    reader.layout = "zd,us,us,hu".into();
    reader.variant = String::new();
    let names = ["ZD (US with HU/RO alt keys)".to_owned(), "English (US)".to_owned()];
    assert_eq!(
        sources(&reader, &names),
        [
            InputSource { id: "us".into(), name: "English (US)".into() },
            InputSource { id: "hu".into(), name: "hu".into() },
            InputSource { id: "zd".into(), name: "ZD (US with HU/RO alt keys)".into() },
        ]
    );
}
```

Note: `parses_the_captured_reply` uses the fixture from Task 4, which was captured with the four-layout list, so `current_id` succeeds.

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test -p input-source --test hyprland_devices`

Expected: compile error (`devices` module missing).

- [ ] **Step 4: Implement `devices.rs`**

`crates/input-source/src/hyprland/devices.rs`:

```rust
//! Hyprland's keyboard list (`j/devices`) and the decisions Downlink makes from it (spec §6.2–6.4).

use serde::Deserialize;

use crate::{InputSource, InputSourceError};

/// Devices Hyprland lists as keyboards that nobody types on. The same list as Omarchy's
/// `keyboard-layout-cycle`, so Downlink and `SUPER + Y` read the same keyboard.
const UNTYPED_PREFIXES: [&str; 5] = [
    "hl-virtual-keyboard",
    "power-button",
    "sleep-button",
    "lid-switch",
    "video-bus",
];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Keyboard {
    pub name: String,
    #[serde(default)]
    pub rules: String,
    #[serde(default)]
    pub model: String,
    pub layout: String,
    #[serde(default)]
    pub variant: String,
    #[serde(default)]
    pub options: String,
    #[serde(default)]
    pub active_layout_index: usize,
    #[serde(default)]
    pub main: bool,
}

#[derive(Deserialize)]
struct Devices {
    keyboards: Vec<Keyboard>,
}

impl Keyboard {
    pub fn is_typed(&self) -> bool {
        !UNTYPED_PREFIXES.iter().any(|p| self.name.starts_with(p))
    }

    /// This keyboard's layout IDs, in `kb_layout` order.
    pub fn layout_ids(&self) -> Vec<String> {
        layout_ids(&self.layout, &self.variant)
    }

    fn same_layouts(&self, other: &Keyboard) -> bool {
        self.layout == other.layout && self.variant == other.variant
    }
}

pub fn no_keyboards() -> InputSourceError {
    InputSourceError::Platform("no keyboards".into())
}

pub fn parse_keyboards(json: &str) -> Result<Vec<Keyboard>, InputSourceError> {
    serde_json::from_str::<Devices>(json)
        .map(|devices| devices.keyboards)
        .map_err(|e| InputSourceError::Platform(format!("unexpected reply from Hyprland: {e}")))
}

/// `layout`, or `layout(variant)` when it has one. Variants pair with layouts by position; a
/// missing variant counts as empty.
pub fn layout_ids(layout: &str, variant: &str) -> Vec<String> {
    if layout.trim().is_empty() {
        return Vec::new();
    }
    let mut variants = variant.split(',');
    layout
        .split(',')
        .map(|layout| {
            let layout = layout.trim();
            match variants.next().map(str::trim).unwrap_or("") {
                "" => layout.to_owned(),
                variant => format!("{layout}({variant})"),
            }
        })
        .collect()
}

/// The keyboard being typed on: the typed one furthest through the list, since only the active
/// keyboard moves through it. Without typed keyboards, the main one, then any.
pub fn reading_keyboard(keyboards: &[Keyboard]) -> Option<&Keyboard> {
    keyboards
        .iter()
        .filter(|k| k.is_typed())
        .max_by_key(|k| k.active_layout_index)
        .or_else(|| keyboards.iter().find(|k| k.main))
        .or_else(|| keyboards.first())
}

pub fn current_id(keyboards: &[Keyboard]) -> Result<String, InputSourceError> {
    let reader = reading_keyboard(keyboards).ok_or_else(no_keyboards)?;
    let ids = reader.layout_ids();
    ids.get(reader.active_layout_index).cloned().ok_or_else(|| {
        InputSourceError::Platform(format!(
            "{} reports layout {} of {}",
            reader.name,
            reader.active_layout_index,
            ids.len()
        ))
    })
}

/// One `switchxkblayout` per keyboard holding the reading keyboard's list, untyped ones included,
/// so none of them drifts (spec §6.3).
pub fn switch_commands(keyboards: &[Keyboard], id: &str) -> Result<Vec<String>, InputSourceError> {
    let reader = reading_keyboard(keyboards).ok_or_else(no_keyboards)?;
    let index = reader
        .layout_ids()
        .iter()
        .position(|layout| layout == id)
        .ok_or_else(|| InputSourceError::NotFound(id.to_owned()))?;
    Ok(keyboards
        .iter()
        .filter(|k| k.same_layouts(reader))
        .map(|k| format!("switchxkblayout {} {index}", k.name))
        .collect())
}

pub fn batch(commands: &[String]) -> String {
    format!("[[BATCH]]{}", commands.join(" ; "))
}

/// Hyprland answers a batch with one reply per command, separated by `\n\n\n`.
pub fn check_batch_reply(reply: &str, commands: usize) -> Result<(), InputSourceError> {
    let replies: Vec<&str> = reply.split("\n\n\n").map(str::trim).collect();
    if replies.len() == commands && replies.iter().all(|r| *r == "ok") {
        return Ok(());
    }
    let reply = reply.trim();
    Err(InputSourceError::Platform(format!(
        "Hyprland did not switch the layout: {}",
        if reply.is_empty() {
            "no reply".to_owned()
        } else {
            reply.replace("\n\n\n", "; ")
        }
    )))
}

/// The selectable layouts: the reading keyboard's list, named by index (an empty or missing name
/// falls back to the ID), deduplicated and sorted by name.
pub fn sources(reader: &Keyboard, names: &[String]) -> Vec<InputSource> {
    let mut sources: Vec<InputSource> = Vec::new();
    for (index, id) in reader.layout_ids().into_iter().enumerate() {
        if sources.iter().any(|s| s.id == id) {
            continue;
        }
        let name = names
            .get(index)
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| id.clone());
        sources.push(InputSource { id, name });
    }
    sources.sort_by(|a, b| a.name.cmp(&b.name));
    sources
}
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p input-source --test hyprland_devices`

Expected: PASS. `the_furthest_typed_keyboard_is_read` relies on `max_by_key` returning the Voyager (index 3), the only maximum.

- [ ] **Step 6: Full check and commit**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

```bash
git add crates/input-source Cargo.lock
git commit -m "feat(input-source): parse Hyprland's keyboards and decide layout switches

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 9: Hyprland socket discovery and requests

**Files:**
- Create: `crates/input-source/src/hyprland/socket.rs`
- Modify: `crates/input-source/src/hyprland/mod.rs` (add `pub mod socket;`)
- Modify: `crates/input-source/Cargo.toml` (`[dev-dependencies] tempfile.workspace = true`)
- Test: `crates/input-source/tests/hyprland_socket.rs`

**Interfaces:**
- Consumes: `InputSourceError::Platform`.
- Produces (module `input_source::hyprland::socket`):
  - `resolve_socket(runtime_dir: &Path, signature: Option<&str>, alive: impl Fn(&Path) -> bool) -> Result<PathBuf, InputSourceError>`
  - `accepts_connections(path: &Path) -> bool`
  - `request(socket: &Path, command: &str) -> Result<String, InputSourceError>`
  - `system_socket() -> Result<PathBuf, InputSourceError>`

- [ ] **Step 1: Write the failing tests**

Add to `crates/input-source/Cargo.toml`:

```toml
[dev-dependencies]
tempfile.workspace = true
```

`crates/input-source/tests/hyprland_socket.rs`:

```rust
#![cfg(target_os = "linux")]

use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

use input_source::InputSourceError;
use input_source::hyprland::socket::{request, resolve_socket};

fn instance(runtime: &Path, name: &str) -> PathBuf {
    let dir = runtime.join("hypr").join(name);
    fs::create_dir_all(&dir).unwrap();
    let socket = dir.join(".socket.sock");
    fs::write(&socket, "").unwrap();
    socket
}

fn never_alive(_: &Path) -> bool {
    false
}

#[test]
fn the_signature_from_the_environment_wins() {
    let runtime = tempfile::tempdir().unwrap();
    let wanted = instance(runtime.path(), "abc_1");
    instance(runtime.path(), "def_2");
    assert_eq!(
        resolve_socket(runtime.path(), Some("abc_1"), never_alive).unwrap(),
        wanted
    );
}

#[test]
fn a_stale_signature_falls_back_to_the_live_instance() {
    let runtime = tempfile::tempdir().unwrap();
    let live = instance(runtime.path(), "live_2");
    let alive = |p: &Path| p.starts_with(runtime.path().join("hypr/live_2"));
    assert_eq!(
        resolve_socket(runtime.path(), Some("gone_1"), alive).unwrap(),
        live
    );
    assert_eq!(resolve_socket(runtime.path(), None, alive).unwrap(), live);
}

#[test]
fn no_live_instance_is_an_error() {
    let runtime = tempfile::tempdir().unwrap();
    assert_eq!(
        resolve_socket(runtime.path(), None, never_alive),
        Err(InputSourceError::Platform("Hyprland is not running".into()))
    );
    instance(runtime.path(), "dead_1");
    assert_eq!(
        resolve_socket(runtime.path(), Some("gone"), never_alive),
        Err(InputSourceError::Platform("Hyprland is not running".into()))
    );
}

#[test]
fn several_live_instances_are_an_error() {
    let runtime = tempfile::tempdir().unwrap();
    instance(runtime.path(), "a_1");
    instance(runtime.path(), "b_2");
    assert_eq!(
        resolve_socket(runtime.path(), None, |_| true),
        Err(InputSourceError::Platform(
            "several Hyprland instances are running".into()
        ))
    );
}

#[test]
fn a_request_writes_the_command_and_reads_the_reply_to_the_end() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".socket.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 256];
        let n = stream.read(&mut buf).unwrap();
        stream.write_all(b"ok\n\n\nok").unwrap();
        String::from_utf8_lossy(&buf[..n]).into_owned()
    });
    let reply = request(&path, "[[BATCH]]switchxkblayout a 0 ; switchxkblayout b 0").unwrap();
    assert_eq!(reply, "ok\n\n\nok");
    assert_eq!(
        server.join().unwrap(),
        "[[BATCH]]switchxkblayout a 0 ; switchxkblayout b 0"
    );
}

#[test]
fn a_request_to_a_missing_socket_is_a_platform_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        request(&dir.path().join(".socket.sock"), "j/devices"),
        Err(InputSourceError::Platform(_))
    ));
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p input-source --test hyprland_socket`

Expected: compile error (`socket` module missing).

- [ ] **Step 3: Implement `socket.rs`**

Add `pub mod socket;` to `crates/input-source/src/hyprland/mod.rs`.

`crates/input-source/src/hyprland/socket.rs`:

```rust
//! Finding Hyprland's IPC socket and sending it one request (spec §6.1).

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, fs};

use crate::InputSourceError;

const SOCKET_NAME: &str = ".socket.sock";
/// Hyprland answers in milliseconds; this only stops a hung compositor from freezing Downlink.
const TIMEOUT: Duration = Duration::from_secs(1);

fn platform(error: impl std::fmt::Display) -> InputSourceError {
    InputSourceError::Platform(format!("Hyprland IPC: {error}"))
}

/// The instance named by `signature` if its socket exists; otherwise the one instance under
/// `<runtime_dir>/hypr/` whose socket is `alive`. A shell started before a Hyprland restart
/// still names the old instance.
pub fn resolve_socket(
    runtime_dir: &Path,
    signature: Option<&str>,
    alive: impl Fn(&Path) -> bool,
) -> Result<PathBuf, InputSourceError> {
    let hypr = runtime_dir.join("hypr");
    if let Some(signature) = signature.filter(|s| !s.is_empty()) {
        let socket = hypr.join(signature).join(SOCKET_NAME);
        if socket.exists() {
            return Ok(socket);
        }
    }
    let live: Vec<PathBuf> = fs::read_dir(&hypr)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path().join(SOCKET_NAME))
        .filter(|socket| alive(socket))
        .collect();
    match live.as_slice() {
        [socket] => Ok(socket.clone()),
        [] => Err(InputSourceError::Platform("Hyprland is not running".into())),
        _ => Err(InputSourceError::Platform(
            "several Hyprland instances are running".into(),
        )),
    }
}

pub fn accepts_connections(path: &Path) -> bool {
    UnixStream::connect(path).is_ok()
}

/// One request per connection: write the command, read until Hyprland closes the socket.
pub fn request(socket: &Path, command: &str) -> Result<String, InputSourceError> {
    let mut stream = UnixStream::connect(socket).map_err(platform)?;
    stream.set_read_timeout(Some(TIMEOUT)).map_err(platform)?;
    stream.set_write_timeout(Some(TIMEOUT)).map_err(platform)?;
    stream.write_all(command.as_bytes()).map_err(platform)?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply).map_err(platform)?;
    Ok(reply)
}

/// The running compositor's socket, from `XDG_RUNTIME_DIR` and `HYPRLAND_INSTANCE_SIGNATURE`.
pub fn system_socket() -> Result<PathBuf, InputSourceError> {
    let runtime = env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| InputSourceError::Platform("XDG_RUNTIME_DIR is not set".into()))?;
    let signature = env::var("HYPRLAND_INSTANCE_SIGNATURE").ok();
    resolve_socket(Path::new(&runtime), signature.as_deref(), accepts_connections)
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p input-source --test hyprland_socket`

Expected: PASS.

- [ ] **Step 5: Full check and commit**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

```bash
git add crates/input-source Cargo.lock
git commit -m "feat(input-source): find Hyprland's IPC socket and send requests

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 10: Layout names from libxkbcommon

**Files:**
- Create: `crates/input-source/src/hyprland/names.rs`
- Modify: `crates/input-source/src/hyprland/mod.rs` (add `pub mod names;`)
- Modify: `crates/input-source/Cargo.toml`
- Test: `crates/input-source/tests/hyprland_names.rs`

**Interfaces:**
- Produces: `input_source::hyprland::names::layout_names(rules: &str, model: &str, layout: &str, variant: &str) -> Vec<String>`, one name per layout in order, or empty if the keymap doesn't compile.

- [ ] **Step 1: Write the failing tests**

`crates/input-source/tests/hyprland_names.rs`:

```rust
#![cfg(target_os = "linux")]

use input_source::hyprland::names::layout_names;

#[test]
fn stock_layouts_get_their_xkb_names_in_order() {
    assert_eq!(
        layout_names("", "", "us,de", ",nodeadkeys"),
        ["English (US)", "German (no dead keys)"]
    );
}

#[test]
fn a_keymap_that_does_not_compile_gives_no_names() {
    assert!(layout_names("", "", "no-such-layout-xyz", "").is_empty());
}
```

Run: `cargo test -p input-source --test hyprland_names`

Expected: compile error (`names` module missing).

- [ ] **Step 2: Add `xkbcommon`**

In `crates/input-source/Cargo.toml`, add to the Linux table:

```toml
xkbcommon = { version = "0.9", default-features = false }
```

- [ ] **Step 3: Implement `names.rs`**

Add `pub mod names;` to `crates/input-source/src/hyprland/mod.rs`.

`crates/input-source/src/hyprland/names.rs`:

```rust
//! Human-readable layout names, read from the compiled keymap (spec §6.2).

use xkbcommon::xkb;

/// Each layout's XKB name, e.g. "English (US)", in `layout` order. Options are left out: they
/// don't change names, and a custom option the system rules don't know would fail the compile.
/// libxkbcommon also searches `~/.config/xkb`, so custom layouts get their names too.
pub fn layout_names(rules: &str, model: &str, layout: &str, variant: &str) -> Vec<String> {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let Some(keymap) = xkb::Keymap::new_from_names(
        &context,
        rules,
        model,
        layout,
        variant,
        None,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    ) else {
        return Vec::new();
    };
    (0..keymap.num_layouts())
        .map(|index| keymap.layout_get_name(index).to_owned())
        .collect()
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p input-source --test hyprland_names`

Expected: PASS. libxkbcommon may print a warning for `no-such-layout-xyz` on stderr; that's expected.

- [ ] **Step 5: Full check and commit**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

```bash
git add crates/input-source Cargo.lock
git commit -m "feat(input-source): name Hyprland layouts from libxkbcommon

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 11: The Hyprland backend

**Files:**
- Modify: `crates/input-source/src/hyprland/mod.rs`
- Modify: `crates/input-source/src/lib.rs`
- Test: `crates/input-source/tests/hyprland_smoke.rs`

**Interfaces:**
- Consumes: `devices::*`, `socket::{system_socket, request}`, `names::layout_names`.
- Produces: `input_source::hyprland::HyprlandBackend` (unit struct, `InputSourceBackend`); `input_source::system_backend()` returns it on Linux.

- [ ] **Step 1: Write the smoke test**

`crates/input-source/tests/hyprland_smoke.rs`:

```rust
//! Touches the live compositor. Run by hand: cargo test -p input-source -- --ignored
#![cfg(target_os = "linux")]

use input_source::system_backend;

#[test]
#[ignore = "talks to the running Hyprland"]
fn lists_layouts_including_the_current_one() {
    let backend = system_backend();
    let sources = backend.list().expect("list");
    assert!(!sources.is_empty());
    let current = backend.current().expect("current");
    assert!(
        sources.iter().any(|s| s.id == current),
        "{current} not in {sources:?}"
    );
}

#[test]
#[ignore = "talks to the running Hyprland"]
fn reselecting_the_current_layout_succeeds() {
    let backend = system_backend();
    let current = backend.current().expect("current");
    backend.select(&current).expect("select current");
    assert_eq!(backend.current().unwrap(), current);
}

#[test]
#[ignore = "talks to the running Hyprland"]
fn selecting_an_unknown_layout_is_not_found() {
    let backend = system_backend();
    assert_eq!(
        backend.select("no-such-layout"),
        Err(input_source::InputSourceError::NotFound("no-such-layout".into()))
    );
}
```

Run: `cargo test -p input-source --test hyprland_smoke -- --ignored`

Expected: FAIL. `system_backend()` is still `UnsupportedBackend`, so `list` returns `Unsupported`.

- [ ] **Step 2: Implement the backend**

`crates/input-source/src/hyprland/mod.rs` becomes:

```rust
//! Hyprland: switches the seat's XKB layout through Hyprland's IPC socket (spec §6).

pub mod devices;
pub mod names;
pub mod socket;

use crate::{InputSource, InputSourceBackend, InputSourceError};
use devices::Keyboard;

/// Resolves the socket on every call, so it keeps working across a Hyprland restart.
pub struct HyprlandBackend;

fn keyboards(socket: &std::path::Path) -> Result<Vec<Keyboard>, InputSourceError> {
    devices::parse_keyboards(&socket::request(socket, "j/devices")?)
}

impl InputSourceBackend for HyprlandBackend {
    fn list(&self) -> Result<Vec<InputSource>, InputSourceError> {
        let keyboards = keyboards(&socket::system_socket()?)?;
        let reader = devices::reading_keyboard(&keyboards).ok_or_else(devices::no_keyboards)?;
        let names = names::layout_names(&reader.rules, &reader.model, &reader.layout, &reader.variant);
        Ok(devices::sources(reader, &names))
    }

    fn current(&self) -> Result<String, InputSourceError> {
        devices::current_id(&keyboards(&socket::system_socket()?)?)
    }

    fn select(&self, id: &str) -> Result<(), InputSourceError> {
        let socket = socket::system_socket()?;
        let commands = devices::switch_commands(&keyboards(&socket)?, id)?;
        let reply = socket::request(&socket, &devices::batch(&commands))?;
        devices::check_batch_reply(&reply, commands.len())
    }
}
```

- [ ] **Step 3: Select it on Linux**

`crates/input-source/src/lib.rs`: change the module doc and the cfg lines:

```rust
//! The OS keyboard input source (macOS: Text Input Sources; Linux: Hyprland's XKB layouts).

mod fake;
#[cfg(target_os = "linux")]
pub mod hyprland;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod unsupported;
```

and `system_backend`:

```rust
/// The backend for the OS this binary was built for.
pub fn system_backend() -> Box<dyn InputSourceBackend> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::TisBackend)
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(hyprland::HyprlandBackend)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Box::new(unsupported::UnsupportedBackend)
    }
}
```

- [ ] **Step 4: Run the smoke test and the example**

Run: `cargo test -p input-source --test hyprland_smoke -- --ignored`

Expected: PASS (re-selecting the current layout changes nothing for the user).

Run: `cargo run -p input-source --example list`

Expected: one `id<TAB>name` line per layout, e.g. `us	English (US)` and `zd	ZD (US with HU/RO alt keys)`.

- [ ] **Step 5: [HUMAN] Switching end to end**

Ask your human partner to write the test configuration into `~/.config/downlink/config.toml`, keeping their `appearance` and `launch_at_login` lines:

```toml
[switching]
fallback = "us"

[switching.layers]
"0" = "zd"
```

Then run `cargo run -p downlink` with the Voyager connected and check:
1. On layer 0, the bar shows ZD. AltGr+z types `á`.
2. Unplugging the Voyager switches every keyboard to `us` (the bar shows English (US)).
3. Replugging switches back to `zd`.
4. Quit Downlink from the tray: the layout is `us`.
5. Settings lists the layouts by name, with the ZD and US choices selected.

Record any failure and fix it before committing.

- [ ] **Step 6: Full check and commit**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

```bash
git add crates/input-source
git commit -m "feat(input-source): Hyprland backend selected on Linux

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 12: Launch at login and window identity on Linux

**Files:**
- Create: `crates/downlink/src/platform/autostart.rs`
- Delete: `crates/downlink/src/platform/other.rs`
- Create: `crates/downlink/src/platform/linux.rs`
- Modify: `crates/downlink/src/platform/mod.rs`
- Modify: `crates/downlink/src/platform/macos.rs` (add `window_platform_specific`)
- Modify: `crates/downlink/src/app.rs` (`open_window`)
- Test: `crates/downlink/tests/autostart.rs`

**Interfaces:**
- Produces (module `downlink::platform::autostart`, platform-neutral):
  - `pub const FILE_NAME: &str = "downlink.desktop"`
  - `exec_arg(executable: &Path) -> String`
  - `desktop_entry(executable: &Path) -> String`
  - `entry_path(config_dir: &Path) -> PathBuf`
  - `install(config_dir: &Path, executable: &Path) -> io::Result<PathBuf>`, `remove(config_dir: &Path) -> io::Result<()>`, `is_installed(config_dir: &Path) -> bool`
  - `is_dev_build(executable: &Path) -> bool`
  - `needs_refresh(existing: Option<&str>, executable: &Path) -> bool`
- Produces: `platform::window_platform_specific() -> iced::window::settings::PlatformSpecific` on both OSes.

- [ ] **Step 1: Write the failing tests**

`crates/downlink/tests/autostart.rs`:

```rust
use std::path::Path;

use downlink::platform::autostart;

const INSTALLED: &str = "/opt/downlink/bin/downlink";

#[test]
fn the_entry_starts_the_executable_hidden_from_launchers() {
    let entry = autostart::desktop_entry(Path::new(INSTALLED));
    assert!(entry.starts_with("[Desktop Entry]\n"));
    for line in [
        "Type=Application",
        "Name=Downlink",
        "Exec=/opt/downlink/bin/downlink",
        "Icon=downlink",
        "NoDisplay=true",
    ] {
        assert!(entry.lines().any(|l| l == line), "missing {line:?} in\n{entry}");
    }
}

#[test]
fn exec_quotes_and_escapes_reserved_characters() {
    assert_eq!(autostart::exec_arg(Path::new(INSTALLED)), INSTALLED);
    assert_eq!(
        autostart::exec_arg(Path::new("/opt/My Apps/downlink")),
        r#""/opt/My Apps/downlink""#
    );
    assert_eq!(
        autostart::exec_arg(Path::new("/opt/50%/downlink")),
        "/opt/50%%/downlink"
    );
    // Quoting escapes `$` with one backslash; the string escape then doubles that backslash.
    assert_eq!(
        autostart::exec_arg(Path::new("/opt/a$b/downlink")),
        r#""/opt/a\\$b/downlink""#
    );
    assert_eq!(
        autostart::exec_arg(Path::new(r#"/opt/say "hi"/downlink"#)),
        r#""/opt/say \\"hi\\"/downlink""#
    );
}

#[test]
fn install_and_remove_under_a_config_directory() {
    let config = tempfile::tempdir().unwrap();
    let path = autostart::install(config.path(), Path::new(INSTALLED)).unwrap();
    assert_eq!(path, config.path().join("autostart/downlink.desktop"));
    assert_eq!(path, autostart::entry_path(config.path()));
    assert!(autostart::is_installed(config.path()));
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        autostart::desktop_entry(Path::new(INSTALLED))
    );
    autostart::remove(config.path()).unwrap();
    assert!(!autostart::is_installed(config.path()));
    autostart::remove(config.path()).expect("removing twice is fine");
}

#[test]
fn cargo_build_directories_are_dev_builds() {
    for dev in [
        "/opt/src/downlink/target/debug/downlink",
        "/opt/src/downlink/target/release/downlink",
        "/opt/src/downlink/target/x86_64-unknown-linux-gnu/release/downlink",
    ] {
        assert!(autostart::is_dev_build(Path::new(dev)), "{dev}");
    }
    for installed in [INSTALLED, "/usr/local/bin/downlink", "/opt/target/downlink"] {
        assert!(!autostart::is_dev_build(Path::new(installed)), "{installed}");
    }
}

#[test]
fn a_dev_build_never_refreshes_the_login_item() {
    let dev = Path::new("/opt/src/downlink/target/debug/downlink");
    assert!(!autostart::needs_refresh(None, dev));
    let other = autostart::desktop_entry(Path::new(INSTALLED));
    assert!(!autostart::needs_refresh(Some(&other), dev));
}

#[test]
fn an_installed_binary_writes_a_missing_or_different_entry_only() {
    let exe = Path::new(INSTALLED);
    assert!(autostart::needs_refresh(None, exe));
    let same = autostart::desktop_entry(exe);
    assert!(!autostart::needs_refresh(Some(&same), exe));
    let moved = autostart::desktop_entry(Path::new("/usr/local/bin/downlink"));
    assert!(autostart::needs_refresh(Some(&moved), exe));
}
```

Run: `cargo test -p downlink --test autostart`

Expected: compile error (`autostart` module missing).

- [ ] **Step 2: Implement `autostart.rs`**

`crates/downlink/src/platform/autostart.rs`:

```rust
//! An XDG autostart entry that starts Downlink at login (spec §8.1). Pure, so it's tested on
//! every OS.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::{fs, io};

pub const FILE_NAME: &str = "downlink.desktop";

/// Characters that make the Desktop Entry spec require a quoted `Exec` argument.
const RESERVED: &[char] = &[
    ' ', '\t', '\n', '"', '\'', '\\', '>', '<', '~', '|', '&', ';', '$', '*', '?', '#', '(', ')',
    '`',
];

/// The executable as one `Exec=` argument. `%` is doubled (field codes); an argument with a
/// reserved character is quoted with `"`, `` ` ``, `$` and `\` backslash-escaped; then the string
/// escape doubles every backslash.
pub fn exec_arg(executable: &Path) -> String {
    let raw = executable.to_string_lossy().replace('%', "%%");
    let quoted = if raw.contains(RESERVED) {
        let mut out = String::from("\"");
        for c in raw.chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                out.push('\\');
            }
            out.push(c);
        }
        out.push('"');
        out
    } else {
        raw
    };
    quoted.replace('\\', "\\\\")
}

pub fn desktop_entry(executable: &Path) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Downlink\n\
         Comment=Live layout and per-layer input switching for the ZSA Voyager\n\
         Exec={}\n\
         Icon=downlink\n\
         NoDisplay=true\n",
        exec_arg(executable)
    )
}

pub fn entry_path(config_dir: &Path) -> PathBuf {
    config_dir.join("autostart").join(FILE_NAME)
}

pub fn install(config_dir: &Path, executable: &Path) -> io::Result<PathBuf> {
    let path = entry_path(config_dir);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&path, desktop_entry(executable))?;
    Ok(path)
}

pub fn remove(config_dir: &Path) -> io::Result<()> {
    match fs::remove_file(entry_path(config_dir)) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

pub fn is_installed(config_dir: &Path) -> bool {
    entry_path(config_dir).exists()
}

/// True inside a cargo build directory: `…/target/debug/…`, `…/target/release/…`, or the same
/// under a target triple.
pub fn is_dev_build(executable: &Path) -> bool {
    let names: Vec<&OsStr> = executable
        .components()
        .filter_map(|c| match c {
            Component::Normal(name) => Some(name),
            _ => None,
        })
        .collect();
    names.iter().enumerate().any(|(i, name)| {
        *name == "target"
            && names[i + 1..]
                .iter()
                .take(2)
                .any(|next| *next == "debug" || *next == "release")
    })
}

/// Whether startup should rewrite the entry: never from a dev build, and only when the entry on
/// disk (`existing`, `None` if missing) differs from the one for `executable`.
pub fn needs_refresh(existing: Option<&str>, executable: &Path) -> bool {
    !is_dev_build(executable) && existing != Some(desktop_entry(executable).as_str())
}
```

Note: `/opt/target/downlink` is not a dev build, because no `debug`/`release` follows `target`.

- [ ] **Step 3: Replace `other.rs` with `linux.rs`**

```bash
git rm crates/downlink/src/platform/other.rs
```

`crates/downlink/src/platform/linux.rs`:

```rust
//! Linux glue (spec §8). Appearance comes from iced's portal watcher, and the compositor
//! handles windows, so most of this does nothing.

use std::path::PathBuf;
use std::{fs, io};

use iced::window::settings::PlatformSpecific;

use super::autostart;

pub fn hide_dock_icon() {}

pub fn activate_app() {}

fn config_dir() -> Result<PathBuf, String> {
    dirs::config_dir().ok_or_else(|| "could not find the config directory".to_owned())
}

pub fn set_launch_at_login(enabled: bool) -> Result<(), String> {
    let config = config_dir()?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        autostart::install(&config, &exe)
            .map(|_| ())
            .map_err(|e| e.to_string())
    } else {
        autostart::remove(&config).map_err(|e| e.to_string())
    }
}

/// Startup check: rewrite the autostart entry if it's missing or points elsewhere, except from a
/// dev build. Returns whether it was rewritten.
pub fn refresh_launch_at_login() -> Result<bool, String> {
    let config = config_dir()?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let existing = match fs::read_to_string(autostart::entry_path(&config)) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.to_string()),
    };
    if !autostart::needs_refresh(existing.as_deref(), &exe) {
        return Ok(false);
    }
    autostart::install(&config, &exe).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Unknown here; iced's own detection (the XDG portal, through mundy) supplies it.
pub fn system_mode() -> iced::theme::Mode {
    iced::theme::Mode::None
}

/// The Wayland `app_id`, matching `downlink.desktop`, so Hyprland and the launcher know the
/// windows.
pub fn window_platform_specific() -> PlatformSpecific {
    PlatformSpecific {
        application_id: "downlink".into(),
        ..PlatformSpecific::default()
    }
}
```

- [ ] **Step 4: Wire the module switch and the macOS counterpart**

`crates/downlink/src/platform/mod.rs`:

```rust
//! OS-specific glue. Everything else in Downlink is platform-neutral.

pub mod autostart;
pub mod launch_agent;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{
    activate_app, hide_dock_icon, refresh_launch_at_login, set_launch_at_login, system_mode,
    window_platform_specific,
};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{
    activate_app, hide_dock_icon, refresh_launch_at_login, set_launch_at_login, system_mode,
    window_platform_specific,
};
```

Append to `crates/downlink/src/platform/macos.rs`:

```rust
/// Nothing macOS-specific: the bundle identifies the windows.
pub fn window_platform_specific() -> iced::window::settings::PlatformSpecific {
    iced::window::settings::PlatformSpecific::default()
}
```

- [ ] **Step 5: Use it when opening windows**

In `crates/downlink/src/app.rs`, `open_window`, add `platform_specific` to both settings:

```rust
        let settings = match kind {
            WindowKind::Layout => window::Settings {
                size: Size::new(980.0, 440.0),
                min_size: Some(Size::new(640.0, 300.0)),
                platform_specific: platform::window_platform_specific(),
                ..window::Settings::default()
            },
            WindowKind::Settings => window::Settings {
                size: Size::new(560.0, 720.0),
                min_size: Some(Size::new(480.0, 420.0)),
                platform_specific: platform::window_platform_specific(),
                ..window::Settings::default()
            },
        };
```

- [ ] **Step 6: Run the tests**

Run: `cargo test -p downlink --test autostart`

Expected: PASS.

- [ ] **Step 7: [HUMAN] Toggle launch at login**

Run `cargo run -p downlink`, open Settings, and ask your human partner to turn **Launch at login** on.

Expected: no error in Settings, and `cat ~/.config/autostart/downlink.desktop` shows an `Exec=` pointing at `target/debug/downlink`. Turning the toggle off removes the file. (A dev build writes the entry when toggled, but never rewrites it on startup; the reboot test happens in Task 15 with the installed binary.)

- [ ] **Step 8: Full check and commit**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

```bash
git add -A crates/downlink
git commit -m "feat(platform): XDG autostart and the Wayland app_id on Linux

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 13: A permission error points at the udev rule

**Files:**
- Modify: `crates/oryx-device/src/hid.rs:82-88`
- Test: `crates/oryx-device/tests/hid_errors.rs`

**Interfaces:**
- Produces: `describe_open_error` maps hidapi's hidraw permission error to the README hint.

- [ ] **Step 1: Write the failing test**

Append to `crates/oryx-device/tests/hid_errors.rs`:

```rust
#[test]
fn a_hidraw_permission_error_points_at_the_udev_rule() {
    let raw = "hidapi error: Failed to open a device with path '/dev/hidraw3': Permission denied";
    assert_eq!(
        describe_open_error(raw),
        "No permission to open the keyboard (install the udev rule, see README)"
    );
}
```

The existing `other_errors_pass_through_unchanged` test uses lowercase `"permission denied"` and must keep passing, so the match is on hidapi's `strerror` text, `Permission denied`.

Run: `cargo test -p oryx-device --test hid_errors`

Expected: FAIL on the new test.

- [ ] **Step 2: Implement**

`crates/oryx-device/src/hid.rs`, `describe_open_error`:

```rust
pub fn describe_open_error(raw: &str) -> String {
    if raw.contains("exclusive access") {
        "The keyboard is in use by another app (quit Keymapp)".to_owned()
    } else if raw.contains("Permission denied") {
        // hidraw on Linux, from strerror(EACCES): the udev rule isn't installed (spec §8.3).
        "No permission to open the keyboard (install the udev rule, see README)".to_owned()
    } else {
        raw.to_owned()
    }
}
```

- [ ] **Step 3: Run the tests**

Run: `cargo test -p oryx-device --test hid_errors`

Expected: PASS (all three).

- [ ] **Step 4: Full check and commit**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

```bash
git add crates/oryx-device
git commit -m "feat(oryx-device): point a hidraw permission error at the udev rule

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 14: Linux packaging and README

**Files:**
- Create: `packaging/linux/50-zsa.rules`
- Create: `packaging/linux/downlink.desktop`
- Create: `packaging/linux/install.sh`
- Modify: `README.md`

**Interfaces:**
- Consumes: the `make-icons` bin, which writes `icon_<n>x<n>.png` (and `@2x` variants) into a directory.

- [ ] **Step 1: The udev rule**

`packaging/linux/50-zsa.rules`:

```
# ZSA keyboards: give the logged-in user access to their raw-HID nodes, so Downlink runs without
# root. The 50- prefix sorts it before 73-seat-late.rules, which applies the uaccess tag.
KERNEL=="hidraw*", ATTRS{idVendor}=="3297", TAG+="uaccess"
```

- [ ] **Step 2: The launcher entry template**

`packaging/linux/downlink.desktop`:

```
[Desktop Entry]
Type=Application
Name=Downlink
Comment=Live layout and per-layer input switching for the ZSA Voyager
Exec=__EXEC__
Icon=downlink
Categories=Utility;
StartupNotify=false
```

- [ ] **Step 3: The install script**

`packaging/linux/install.sh`:

```bash
#!/usr/bin/env bash
# Installs Downlink for the current user: the binary, its launcher entry and icons. No sudo; the
# udev rule is a separate step (see README). Usage: packaging/linux/install.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
EXE="$BIN_DIR/downlink"

cargo build --release -p downlink --manifest-path "$ROOT/Cargo.toml"
install -Dm755 "$ROOT/target/release/downlink" "$EXE"

ICONS="$(mktemp -d)"
trap 'rm -rf "$ICONS"' EXIT
cargo run --release -p downlink --bin make-icons --manifest-path "$ROOT/Cargo.toml" -- "$ICONS"
for size in 16 32 128 256 512; do
  install -Dm644 "$ICONS/icon_${size}x${size}.png" \
    "$DATA_DIR/icons/hicolor/${size}x${size}/apps/downlink.png"
done

mkdir -p "$DATA_DIR/applications"
sed "s|__EXEC__|$EXE|" "$ROOT/packaging/linux/downlink.desktop" \
  > "$DATA_DIR/applications/downlink.desktop"
chmod 644 "$DATA_DIR/applications/downlink.desktop"

echo "Installed $EXE"
```

Run: `chmod +x packaging/linux/install.sh && bash -n packaging/linux/install.sh`

Expected: no output (syntax OK).

- [ ] **Step 4: [HUMAN] Install and launch**

Run: `packaging/linux/install.sh`

Expected: `Installed …/.local/bin/downlink`. Ask your human partner to quit any running Downlink, then start "Downlink" from the Omarchy app launcher. Expected: the tray icon appears. Opening Show Layout shows a window that Hyprland lists with class `downlink` (`hyprctl -j clients | jq -r '.[].class' | grep downlink`).

- [ ] **Step 5: Update the README**

In `README.md`:

1. In the intro list, change "**Follows macOS Light/Dark** automatically." to "**Follows the system Light/Dark** automatically."
2. Replace the Requirements section with:

```markdown
## Requirements

- A ZSA Voyager running Oryx firmware (any layout built in Oryx)
- macOS 13 or newer, or Linux with Hyprland (tested on Omarchy)
- To build: Rust stable ≥ 1.90 (`rustup`). On macOS, also Xcode Command Line Tools.
```

3. Rename "## Build and install" to "## Build and install on macOS". After that section (before "## Moving over from Keymapp…"), add:

````markdown
## Build and install on Linux (Hyprland)

1. Let your user open the keyboard (once, needs sudo):

   ```bash
   sudo install -m644 packaging/linux/50-zsa.rules /etc/udev/rules.d/
   sudo udevadm control --reload && sudo udevadm trigger
   ```

   Replug the Voyager afterwards. Without the rule, the menu says "No permission to open the
   keyboard".
2. Install the binary, launcher entry and icons into `~/.local`:

   ```bash
   packaging/linux/install.sh
   ```

3. Start **Downlink** from your app launcher. Its icon appears in the bar's tray (Omarchy shows
   it with the `omarchy.tray` widget).

Downlink switches Hyprland's keyboard layouts: list every layout you want to switch to in
`kb_layout` (at most four), for example `kb_layout = "us,de"`. To see their IDs:
`cargo run -p input-source --example list`. Closing a window (for example with Meta+W) keeps
Downlink running; to quit, use **Quit Downlink** in the tray menu.
````

4. In "## Configuration", after the macOS paragraph and example, add:

````markdown
On Linux, settings are in `~/.config/downlink/config.toml`, the layout cache in
`~/.cache/downlink/`, and logs in `~/.local/state/downlink/`. Layout IDs are Hyprland's
`kb_layout` entries, with the variant in brackets when there is one:

```toml
[switching]
fallback = "us"

[switching.layers]
"0" = "us"
"1" = "de(nodeadkeys)"
```

Launch at login writes `~/.config/autostart/downlink.desktop`.
````

5. Fix the macOS-only wording above the config block: change "Settings are saved in" to "On macOS, settings are saved in", and "To list the input-source IDs on your Mac:" to "To list the input-source IDs:".

Keep the rest of the README unchanged.

- [ ] **Step 6: Commit**

```bash
git add packaging/linux README.md
git commit -m "feat(packaging): Linux udev rule, launcher entry and install script

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 15: [HUMAN] Linux hardware checklist

**Files:**
- Modify: `docs/testing/manual-hardware-checklist.md`

- [ ] **Step 1: Add the Linux section**

Rename the existing intro so it reads as the macOS section, and add below it:

```markdown
## Linux (Hyprland on Omarchy)

Run against the installed binary (`packaging/linux/install.sh`, then start Downlink from the
launcher), with the udev rule installed. Record pass/fail and the date for each item.

- [ ] Cold start with the keyboard plugged in: connected within 2 s, tray icon in the bar.
- [ ] Plug the keyboard in after start: connects within 2 s.
- [ ] Unplug and replug: fallback layout on unplug, icon dims, reconnects.
- [ ] Suspend and resume, three cycles: events resume each time without restarting Downlink.
- [ ] All layers show the right legends in the layout window.
- [ ] Layer 0 selects ZD; transient layers keep it; the bar's layout widget agrees.
- [ ] Fallback layout on unplug and on Quit.
- [ ] Pause Input Switching stops switching; resuming switches on the next layer change.
- [ ] Appearance "System" follows the Omarchy light/dark theme live (windows and tray icon);
      Light and Dark force a palette.
- [ ] Launch at login: enable, reboot, Downlink is running and its icon is in the bar.
- [ ] Meta+W in the Layout and Settings windows closes the window only; the icon stays and
      switching continues.
- [ ] Restart the bar (`omarchy-restart-shell`): the tray icon comes back.
- [ ] A second launch exits immediately ("already running" in the log).
- [ ] Offline (Wi-Fi off) with a cached layout: the layout still shows, switching works.
- [ ] Logs in `~/.local/state/downlink/` have no ERROR lines during the above.
```

- [ ] **Step 2: [HUMAN] Run it**

Walk your human partner through each item and tick the ones that pass. Add a "Last run:" line with the date and commit hash, as the macOS section has. For each failure: stop, debug it with superpowers:systematic-debugging, fix it in its own commit with a test where one is possible, then re-run the item.

- [ ] **Step 3: Commit**

```bash
git add docs/testing/manual-hardware-checklist.md
git commit -m "docs: Linux manual hardware checklist, first run

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 16: [HUMAN] macOS still works

**Files:**
- Modify: `docs/testing/manual-hardware-checklist.md` (the macOS "Last run" line)

- [ ] **Step 1: Push the branch for the Mac**

Ask your human partner how they want to get the branch onto the Mac (push to the remote, or another way). Push only with their go-ahead.

- [ ] **Step 2: [HUMAN] Build and test on the Mac**

On the Mac, in the checkout of `feat/linux-support`:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p input-source -- --ignored
packaging/macos/bundle.sh
```

Expected: everything passes and the bundle builds. Any compile error is most likely in `tray/macos.rs` or `platform/macos.rs`; fix it on the Mac or send the error back to fix here.

- [ ] **Step 3: [HUMAN] Run the macOS checklist**

Run the macOS section of `docs/testing/manual-hardware-checklist.md` against the new bundle. Focus on the tray, ⌘W/⌘Q, Light/Dark and launch at login, the areas this branch touched. Update its "Last run:" line.

- [ ] **Step 4: Commit**

```bash
git add docs/testing/manual-hardware-checklist.md
git commit -m "docs: macOS checklist re-run on the Linux branch

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

Then use superpowers:finishing-a-development-branch.
