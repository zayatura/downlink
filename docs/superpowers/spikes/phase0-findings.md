# Phase 0 findings

Throwaway probes on the local branch `spike/phase0` (never merged): `spikes/hid-probe`,
`spikes/tis-probe` and `spikes/tray-daemon`. Rust 1.99.0, macOS 27.0.1 (arm64), Voyager on
protocol v5. Raw packets: [phase0-captures.txt](phase0-captures.txt).

| # | Question (spec §4.6) | Answer | Evidence |
|---|----------------------|--------|----------|
| 1 | Do LAYER/KEYDOWN/KEYUP/FW_VERSION arrive via hidapi as described? | **Yes.** The replies arrive in send order (FW_VERSION, PROTOCOL_VERSION `05`, PAIRING_SUCCESS), followed by LAYER 0. Key events report `(col, row)`. Layers 0–8 seen. Bytes after the `0xFE` stop byte are uninitialised garbage. | Experiment C: 68 packets, no unknown codes |
| 2 | Does macOS require Input Monitoring for the 0xFF60 interface? | **No.** No prompt appeared, and neither Terminal nor the probe was added to Privacy & Security → Input Monitoring. | Experiment C, System Settings check |
| 3 | Can Downlink and Keymapp hold the interface at the same time? | **No.** Keymapp opens it exclusively, so both shared and exclusive opens fail with IOKit `0xE00002C5` ("exclusive access and device already open") while Keymapp is connected. With Keymapp quit, the shared open works. | Experiments A, B, C |
| 4 | Does TISSelectInputSource switch the custom layouts reliably? | **Yes, 30/30.** The EN → RO → HU cycle ran ×10, and `TISCopyCurrentKeyboardInputSource` confirmed each switch. No `macism` workaround is needed. | TIS probe |
| 5 | Does tray-icon work inside an iced 0.14 daemon on macOS? | **Yes.** The tray is created from the first `update()` on the main thread. The Accessory policy hides the Dock icon. Windows open in front. The daemon survives closing its last window. The menu's Quit returns from `run()`. Exception: **⌘Q in a window quits the whole process**; see below. | tray-daemon items 1–6 |
| 6 | Transparent vs no-op key encoding | Resolved during planning: a null action is `KC_TRANSPARENT` | spec §4.4 |

## Sleep/wake behaviour

The device stayed open across sleep, with no read error at wake. Events resumed after wake
**without** re-pairing. The 5 s heartbeat is therefore defensive rather than essential. It is
kept because it is cheap and covers firmware that unpaired after a failed send.

The real data also confirms the matrix → Oryx mapping. Holding matrix (5,0) raised layer 3 and
(5,1) raised layer 7: the left thumb keys Space/MO(3) and Tab/MO(7), at Oryx indices 24 and 25.

## ⌘Q and window activation

- `platform::activate_app()` (`activateIgnoringOtherApps`) plus `window::gain_focus` bring new
  windows to the front.
- **⌘Q:** iced_winit doesn't handle ⌘Q. winit installs a default macOS app menu whose Quit item
  sends `NSApp terminate:`. That terminates the process immediately. Very likely `run()` never
  returns, so the spec §8 "safety net after the event loop returns" can't be relied on, and the
  fallback input source would not be selected.
- **Fix (both options below):** on macOS, Downlink installs its own app menu with muda
  (`Menu::init_for_nsapp`), replacing winit's. Its ⌘Q item sends a normal menu event that the
  app handles. **Open decision for the human partner:** should ⌘Q in a Downlink window
  (A) quit Downlink through the normal Quit path, or (B) only close the window, with Quit
  available solely from the menu-bar menu?

## Theme

At boot, `iced::system::theme()` returned `Mode::None`: winit can't read the appearance before
a window exists. iced then reads each window's real appearance when it opens, broadcasts
`SystemThemeChanged` when it differs, and follows `ThemeChanged` live. Switching appearance
re-coloured the window correctly. Without a seed, though, the first frame of the first window
renders with the fallback (light) palette.

## Consequences for the plan

1. **Task 20, tray-icon 0.26 deprecations:** `.with_icon(..).with_icon_as_template(true)` →
   `.with_icon_templated(..)`, and `set_icon_with_as_template(Some(i), true)` →
   `set_icon_templated(Some(i))`. Phase 1 builds with `-D warnings`.
2. **Tasks 9 and 23, plus spec §8, Keymapp exclusivity:** `HidDiscovery::open` maps the IOKit
   "exclusive access" error to "The keyboard is in use by another app (quit Keymapp)". The pure
   function `describe_open_error` is tested. The README already tells users to quit Keymapp.
3. **Tasks 18 and 20, startup theme:** a new `platform::system_mode()` (macOS: `defaults read -g
   AppleInterfaceStyle`; elsewhere `Mode::None`) seeds `App::system_mode` at Init and whenever a
   window opens, so the first frame uses the right palette. The output parser
   `appearance_from_interface_style` is tested.
4. **Task 6, protocol tests:** a new test asserts the firmware-version text stops at the stop
   byte even when garbage follows it.
5. **Task 20 and spec §8, ⌘Q:** pending the open decision above. Both options replace winit's
   default menu with a muda app menu, and the exit safety net stays as a backstop only.
6. **Spec §4.6 and §14:** questions 1–5 are marked resolved; the Input Monitoring risk row is
   closed.
