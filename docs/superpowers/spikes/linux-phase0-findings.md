# Linux Phase 0 — Findings

- **Date:** 2026-10-15
- **Spec:** [Downlink on Linux](../specs/2026-10-15-linux-support-design.md), §9
- **Setup:** Hyprland 0.56.2 on Omarchy, Wayland, uwsm, quickshell bar, fcitx5 running; ZSA
  Voyager (protocol 5)
- **Spike code:** throwaway, kept on the local branch `spike/linux-phase0` and never merged

All four questions pass. No fallback from spec §9 was needed.

## P1 — Does hidraw pairing work?

**Ran:** `cargo run -p oryx-device --example watch` with the udev rule installed. Switched
layers, pressed keys, replugged, then suspended (s2idle) and resumed.

**Observed:**
- The Voyager exposes 4 hidraw nodes, all readable and writable by the logged-in user.
- hidapi's hidraw backend reports the `0xFF60/0x61` interface, so `Model::from_interface` works
  unchanged. The example printed `Connected { model: Voyager, … protocol: Some(5) }`, then
  `Layer(n)` on layer changes and `KeyDown`/`KeyUp` on key presses.
- Replug: `Disconnected`, then `Connected` again (also on another USB port).
- After suspend and resume, layer and key events arrived again without restarting.
- While the keyboard is idle, `Layer(<current>)` repeats every ~5 s: the firmware answers each
  heartbeat re-pair with the current layer. That's the same code path as on macOS, and the
  engine ignores a repeated layer.
- Arch's `extra/zsa-udev` package was already installed (as a dependency). Its `50-oryx.rules`
  is the same `uaccess` rule as ours, and its `50-wally.rules` also sets mode `0666` on every
  ZSA USB device. The nodes looked `0600` during the design only because the Voyager wasn't
  plugged in.

**Answer:** yes. On Arch, `zsa-udev` is enough; our rule is the fallback.

## P2 — Does a layout switch reach typing?

**Ran:** set `kb_layout = "zd,hu,ro,us"`, `kb_variant = ",102_qwerty_dot_nodead,std,"`. Sent one
`hyprctl --batch` of `switchxkblayout <name> <index>` to every keyboard holding that list, to `us`
(index 3) and back to `zd` (index 0). Typed AltGr+z and the Voyager's Hungarian-layer `á` key.

**Observed:**
- 12 keyboards. After the config reload, fcitx5's virtual keyboard (`hl-virtual-keyboard-fcitx5`,
  the `main` one) kept the stale `zd,hu,ro` list until a later keymap change. The other 11
  picked up the new list. The captured fixture
  (`crates/input-source/tests/fixtures/hyprland-devices.json`) is from that moment.
- The batch replied `ok` once per command. All 11 switched, and the virtual keyboard followed
  too, although it wasn't in the batch: fcitx5 mirrors the physical keyboard's state.
- On `us`: no `á` in Nautilus (GTK), KeePassXC (Qt) or Ghostty. On `zd`: `á` in all three.
- The bar's layout widget showed ZD, and `SUPER + Y` continued from it to HU on all 12 keyboards.

**Answer:** yes. Typing follows with fcitx5 running; no fcitx5 backend is needed. The rule
"switch keyboards whose layout and variant equal the reading keyboard's" holds up.

## P3 — Do iced and a `ksni` tray coexist?

**Ran:** a minimal iced 0.14 `daemon` with a ksni 0.3 tray (`blocking` + `async-io`), spawned
from inside `update` with `assume_sni_available(true)`.

**Observed:**
- No tokio panic. The item registered as `org.kde.StatusNotifierItem-<pid>-1`.
- The icon showed in the bar. A left click opened the menu, and the disabled row was greyed.
- A menu click reached iced and opened a window. `handle.update` from `update` changed the menu.
- `omarchy-restart-shell`: `watcher_offline` fired, and the icon came back by itself.

**Answer:** yes.

## P4 — Do windows behave?

**Ran:** opened windows from the tray, closed one with Meta+W, checked `hyprctl -j clients`,
switched between a dark and a light Omarchy theme, and flipped `color-scheme` with a window open.

**Observed:**
- Meta+W closed the window. The daemon and the tray kept running.
- The window's class is `downlink`, set through `platform_specific.application_id`.
- With no window open, switching to a dark Omarchy theme delivered `Dark`, so the tray icon can
  follow the appearance.
- With a window open, `prefer-light` then `prefer-dark` delivered `Light` then `Dark`, and the
  window re-themed live.
- iced reports `Mode::None` as a theme change each time a window opens on Wayland (winit can't
  read a window's theme there). Downlink's `App` already ignores `Mode::None`.

**Answer:** yes.

## Spec changes

- §4, fcitx5 row: the virtual keyboard's layout string can be stale after a reload; it follows
  through fcitx5 anyway.
- §4, hidraw row, and §8.3: Arch's `zsa-udev` provides the same rule; the README recommends it
  on Arch and keeps ours as the fallback.
- §8.4, Appearance: iced reports `Mode::None` when a window opens on Wayland; `App` must keep
  ignoring it.
