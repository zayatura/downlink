# Downlink — Design Spec

- **Date:** 2026-10-13
- **Status:** Draft, awaiting review
- **Bundle ID:** `com.zayatura.downlink`
- **Mockups:** [layout window](../mockups/layout-window.html), [menu bar and settings](../mockups/menubar-settings.html)

## 1. Summary

Downlink is a native Rust menu-bar app for the ZSA Voyager keyboard. It talks to the keyboard
directly over USB HID. It shows the flashed Oryx layout live, with the active layer and pressed
keys, and it switches the OS input source to match the active keyboard layer.

It replaces two things:

1. **ZSA Keymapp**, for its "My Layout" live view and its keyboard connection.
2. **A Python layout-switcher service**, which polled Keymapp's gRPC API and switched the macOS
   input source with the `macism` CLI.

The name comes from the Voyager space probes. The keyboard sends telemetry (layer and key events)
down to the host, and Downlink receives it and acts on it.

## 2. Goals and success criteria

| # | Goal | Success looks like |
|---|------|--------------------|
| G1 | Connect automatically | Within about 2 s of the keyboard appearing (login, replug, wake from sleep), with no user action. Survives sleep/wake and USB replug indefinitely. |
| G2 | Switch input source by layer | Same behavior as the Python service (section 6.4). No Keymapp and no `macism` needed. |
| G3 | Live layout view | A window draws the flashed layout. It follows the active layer and highlights pressed keys with no visible lag. |
| G4 | Nice GUI | Matches the approved mockups: a menu-bar icon and menu, a layout window, and a Settings window. |
| G5 | Automatic dark/light | With "System" selected, every window follows the OS appearance live. Light and Dark can be forced. |
| G6 | Full replacement | Keymapp and the Python service can be uninstalled. Downlink starts at login and runs with no windows open. |
| G7 | Linux-ready | macOS only for v1, but all OS-specific code sits behind small interfaces, so a Linux port adds implementations without restructuring. |

## 3. Non-goals for v1

- Flashing firmware, the heatmap, Keymapp's "smart layers" (switching keyboard layers per app),
  RGB and status-LED control, and signing in to an Oryx account.
- Keyboards other than the Voyager. The protocol is shared across ZSA boards, but only Voyager
  geometry ships. Section 10 notes how more boards would be added.
- A floating overlay or HUD. The layout window must not prevent adding one later.
- Changing the keyboard's layer from the app (`ORYX_SET_LAYER`). The device command channel
  exists, so this can be added later.
- Signing with an Apple Developer ID, notarization, auto-update, Windows support.

## 4. Background: how Keymapp works (research findings)

Research was done on Keymapp 1.3.7 (universal Mach-O). The sources were: symbol and string
extraction from the binary, the live USB HID registry (`ioreg`), Keymapp's local SQLite database
opened read-only, Keymapp's gRPC API, ZSA's open-source firmware, and one live Oryx GraphQL
request. Decompilation was not needed; section 4.6 lists what is still unverified.

### 4.1 Keymapp's architecture

- Go 1.23 backend (`github.com/zsa/keymapp/backend`). The UI is a Wails webview with a Svelte
  frontend embedded in the binary.
- USB access has two parts. A C++ library wrapped with SWIG (`github.com/zsa/zsausb`, bundled
  `libusb-1.0.0.dylib`) handles hotplug and flashing. `github.com/sstallion/go-hid` (hidapi)
  carries the raw-HID packets.
- Backend methods include `InitUSB`, `OpenDevice`, `hidCallback`, `RequestPairing`,
  `RequestFirmwareVersion`, `RequestFirmwareProtocolVersion`, `GetLayout`, plus the gRPC server
  methods `GetStatus`, `SetLayer` and so on.
- Its gRPC API is a Unix socket at `~/Library/Application Support/.keymapp/keymapp.sock`. Its
  `GetStatus` reports `firmware_version` and `current_layer`.

### 4.2 USB transport (verified live)

| Property | Value |
|----------|-------|
| Vendor ID / product ID | `0x3297` / `0x1977` |
| Raw-HID interface | usage page `0xFF60`, usage `0x61` |
| Report size | 32 bytes in, 32 bytes out |
| USB serial number | `<layoutId>/<revisionId>`, e.g. `eVAVm/m57mrj` |

The Voyager also exposes standard keyboard, mouse and consumer interfaces (usage page `0x01`).
Downlink opens only the `0xFF60/0x61` interface.

### 4.3 The Oryx raw-HID protocol (source: `zsa/qmk_modules`, `oryx/oryx.{h,c}`, protocol v5)

Packets are 32 bytes: `[code, params…, 0xFE (stop byte, ORYX_STOP_BIT = -2), padding]`.

Host → keyboard commands:

| Code | Name | Params |
|------|------|--------|
| 0x00 | `GET_FW_VERSION` | — |
| 0x01 | `PAIRING_INIT` | — |
| 0x02 | `PAIRING_VALIDATE` | — (no-op, kept for compatibility) |
| 0x03 | `DISCONNECT` | — |
| 0x04 | `SET_LAYER` | `[on/off, layer]` |
| 0x05–0x0A | RGB and status-LED control | (out of scope) |
| 0x0B / 0x0C | `SET_AUTOMOUSE` / `GET_AUTOMOUSE` | (v5 only, out of scope) |
| 0xFE | `GET_PROTOCOL_VERSION` | — |

Keyboard → host events:

| Code | Name | Payload |
|------|------|---------|
| 0x00 | `GET_FW_VERSION` | ASCII serial `layoutId/revisionId`, terminated by 0xFE |
| 0x04 | `PAIRING_SUCCESS` | — (immediately followed by a `LAYER` event) |
| 0x05 | `LAYER` | `[layer]` = highest active layer |
| 0x06 | `KEYDOWN` | `[col, row]` (matrix position) |
| 0x07 | `KEYUP` | `[col, row]` |
| 0xFE | `GET_PROTOCOL_VERSION` | `[version]` |
| 0xFF | `ERROR` | `[error code]` |

Key facts from the firmware source:

- **Pairing is vestigial.** `PAIRING_INIT` sets `paired = true` at once and sends
  `PAIRING_SUCCESS` followed by the current `LAYER`. The pairing UI that older Oryx clients had
  is not needed.
- `LAYER`, `KEYDOWN` and `KEYUP` events are sent **only while paired**. A host must send
  `PAIRING_INIT` after every (re)connect.
- The firmware drops repeated `LAYER` events (one-shot layers fire the change three times), and
  it reports `get_highest_layer(layer_state | default_layer_state)`.
- If a send fails, the firmware clears `paired`. So a host that stopped reading has to pair
  again.

### 4.4 Layout data (verified live)

- **Endpoint:** `POST https://oryx.zsa.io/graphql`. The query is
  `layout(hashId, geometry, revisionId)`, taking `hashId` and `revisionId` from the USB serial and
  `geometry = "voyager"`. **No authentication is needed** for a layout whose privacy flag is off;
  Keymapp's `auth` table was empty and the request returned HTTP 200.
- The response has `layout { title, geometry, hashId, privacy, revision { hashId, qmkVersion,
  config, layers[] { position, title, color, keys[52] } , combos[] } }`.
- Each key has `tap`, `hold`, `doubleTap` and `tapHold` actions, each
  `{ code, layer, modifiers, macro, … }` or null, plus `customLabel`, `emoji`, `icon` and
  `glowColor`.
- **Keycode label catalogue:** `GET https://configure.zsa.io/metadata.json?release=<ver>`, which
  Keymapp reaches by a redirect from `keymapp-startup-data.json`. It holds about 2,500 keycodes:
  `{ code, label, glyph, searchLabel, … }` and 61 categories. Some common codes (for example
  `KC_BSPC`) have no label, so Downlink needs its own built-in label table as a fallback.

### 4.5 Key order and geometry

- QMK's `keyboards/zsa/voyager/keyboard.json` (on the `zsa/qmk_firmware` `firmware25` branch)
  lists 52 keys in `LAYOUT` order, each with a matrix `[row, col]` and a physical `x, y` (in key
  units, 16 × 5.75 overall).
- **Oryx numbers keys differently from QMK.** Oryx goes left half first (four rows of six keys,
  then two thumb keys), then the right half the same way:

  | Oryx index | QMK `LAYOUT` index |
  |------------|--------------------|
  | `r*6 + c` (r 0–3, c 0–5) | `r*12 + c` |
  | 24, 25 | 48, 49 |
  | `26 + r*6 + c` | `r*12 + 6 + c` |
  | 50, 51 | 50, 51 |

  This was checked against real data: a Colemak-DH layer 0 decodes correctly with this mapping
  and incorrectly without it.
- `KEYDOWN`/`KEYUP` events report `(col, row)`. Downlink maps them to a QMK matrix position, then
  to the `LAYOUT` index, then to the Oryx index.

### 4.6 Not yet verified (resolved by the Phase 0 spike, section 9)

1. Whether layer, key and firmware-version events arrive exactly as described when read through
   `hidapi`.
2. Whether macOS asks for Input Monitoring permission to open the `0xFF60` interface.
3. Whether Downlink and Keymapp can both hold that interface at the same time.
4. Whether `TISSelectInputSource` switches custom Ukelele layouts reliably. `macism` exists
   because input methods (as opposed to plain keyboard layouts) sometimes don't switch.
5. Whether `tray-icon` can be created and driven inside an iced 0.14 `daemon` on macOS. It must
   be created on the main thread after the event loop starts.
6. How Oryx encodes a transparent key (one that falls through to the layer below) compared with
   a no-op key.

## 5. Architecture

### 5.1 Workspace layout

```
Cargo workspace (repo root)
├─ crates/
│  ├─ oryx-protocol/   pure: Command → bytes, bytes → Event. No I/O.
│  ├─ oryx-device/     hidapi transport, discovery, handshake, read loop, reconnect
│  ├─ oryx-layout/     Oryx GraphQL + metadata client, layout model, disk cache,
│  │                   Voyager geometry and matrix → Oryx index mapping, key labels
│  ├─ input-source/    trait InputSourceBackend + macOS TIS implementation
│  ├─ companion-core/  pure engine (state machine) + config model
│  └─ downlink/        app binary: iced daemon, tray-icon, windows, theme, platform glue
├─ packaging/macos/    Info.plist template, bundle script, LaunchAgent template
└─ docs/
```

Dependency direction: `downlink` depends on everything. `companion-core` depends on
`oryx-protocol`, `oryx-layout` (model types) and `input-source` (trait only). The other crates
don't depend on each other except `oryx-device → oryx-protocol`.

### 5.2 Crates

**`oryx-protocol`** — `enum Command { GetFwVersion, PairingInit, GetProtocolVersion,
Disconnect, SetLayer { on, layer } }` and `fn encode(&Command) -> [u8; 32]`. Also
`enum Event { FwVersion { layout_id, revision_id }, PairingSuccess, Layer(u8),
KeyDown { col, row }, KeyUp { col, row }, ProtocolVersion(u8), Error(u8), Unknown(u8) }` and
`fn decode(&[u8]) -> Result<Event, DecodeError>`. It's tested with captured packets.

**`oryx-device`** — runs a dedicated thread built on the `hidapi` crate (IOKit on macOS, hidraw
on Linux). It finds the device by vendor ID, product ID, usage page and usage.
- The loop:
  - Disconnected: enumerate every 1 s.
  - On a match: open it, send `PAIRING_INIT`, `GET_FW_VERSION` and `GET_PROTOCOL_VERSION`, then
    `read_timeout(100 ms)` in a loop.
  - On a read or write error: report `Disconnected` and go back to enumerating.
- It sends out `DeviceEvent { Connected { layout_id, revision_id, protocol }, Layer(u8),
  KeyDown(MatrixPos), KeyUp(MatrixPos), Disconnected { reason }, Unsupported { reason } }`.
- It takes in `DeviceCommand { Shutdown, Send(Command) }`.
- The USB serial number is used as a fallback for the layout and revision IDs if
  `GET_FW_VERSION` doesn't answer.
- It has a `Transport` trait, so tests can replace hidapi with a fake.

**`oryx-layout`** — `async fn fetch_layout(layout_id, revision_id, geometry)` uses `reqwest`
with rustls and sends the GraphQL query from section 4.4. It also has
`async fn fetch_metadata(release)` and a `LayoutCache`, which stores one JSON file per revision
ID plus the metadata. The model:
- `Layout { title, layout_id, revision_id, layers: Vec<Layer> }`
- `Layer { index, title, color, keys: [KeyAction; 52] }`
- `KeyAction { tap, hold, double_tap, tap_hold, custom_label }`

The **Voyager geometry** is a static table taken from `keyboard.json`, with Oryx index ↔ matrix
↔ `(x, y)`. **Labels** turn an `Action` into display text in this order: custom label, then the
metadata catalogue, then a built-in fallback table, then the raw code. Layer actions render as
"→ {layer title}" (TO/TG/DF) or "hold: {layer title}" (MO/LT/OSL).

**`input-source`** — the platform interface:
```rust
pub struct InputSource { pub id: String, pub name: String }
pub trait InputSourceBackend: Send {
    fn list(&self) -> Result<Vec<InputSource>>;      // enabled + select-capable only
    fn current(&self) -> Result<String>;             // id
    fn select(&self, id: &str) -> Result<()>;
}
```
The macOS implementation calls Carbon TIS through FFI: `TISCreateInputSourceList` filtered by
`kTISPropertyInputSourceIsEnabled` and `kTISPropertyInputSourceIsSelectCapable`, plus
`TISCopyCurrentKeyboardInputSource` and `TISSelectInputSource`. A `FakeBackend` is used in
tests. Linux implementations come later, behind `#[cfg(target_os = "linux")]`.

**`companion-core`** — `Engine` is a pure state machine:
`fn handle(&mut self, input: Input) -> Vec<Effect>`.
- `Input` = `Device(DeviceEvent)`, `LayoutLoaded(Result<Layout>)`, `CurrentSourceChanged(id)`,
  `ConfigChanged(Config)`, `SetPaused(bool)`, `PreviewLayer(Option<u8>)`, `Quit`.
- `Effect` = `SelectInputSource(id)`, `FetchLayout { layout_id, revision_id }`,
  `PersistConfig`, `RefreshTray`, `Exit`.
- The state is the connection state, the layout and revision IDs, the active layer, the pressed
  set (Oryx indices, cleared on disconnect only), the last source Downlink selected, `paused`,
  the preview layer and config.
- It holds no I/O, so all the switching rules are unit-tested.

**`downlink`** — the iced 0.14 `daemon`:
- Subscriptions: the device thread, tray and menu events, `iced::system::theme_changes()`.
- Effects are executed in `update()`. Input-source calls run on the main thread, and fetches run
  as async `Task`s.
- Two window types: Layout and Settings, each opened on demand. At most one of each exists.
- `tray-icon` provides the menu-bar icon and menu.
- Platform glue goes in small modules behind `cfg`: launching at login, single-instance lock,
  app paths.

### 5.3 Data flow

```
Voyager ──HID──▶ device thread ──DeviceEvent──▶ iced Subscription ─┐
tray/menu ──MenuEvent────────────────────────▶ iced Subscription ─┤
OS appearance ──theme_changes()──────────────▶ iced Subscription ─┤
                                                                  ▼
                                  update(): Engine::handle(input) → effects
                                     ├─ SelectInputSource → TIS (main thread)
                                     ├─ FetchLayout → async Task → LayoutLoaded
                                     ├─ PersistConfig → write config.toml
                                     └─ RefreshTray → icon state + menu labels
                                  view(window): render Engine state
```

Switching never needs a window: the subscriptions run whenever the daemon runs.

## 6. User-facing behavior

### 6.1 Menu bar (approved: icon only)

- A monochrome template icon (an antenna glyph), drawn normally when connected and dimmed or
  outlined when disconnected or not working. No text next to it, because macOS already shows the
  input source.
- The menu, from top to bottom:
  1. Status (disabled item): "● Voyager connected" / "Waiting for Voyager…" /
     "Firmware doesn't report live data" / "Can't open keyboard: {reason}".
  2. Detail (disabled item): "Layer: {layer title} · Input: {source name}".
  3. —
  4. **Show Layout** (⌘L)
  5. **Settings…** (⌘,)
  6. —
  7. **Pause Input Switching** (checkmark when on; not saved, resets on launch)
  8. —
  9. **Quit Downlink** (⌘Q)

### 6.2 Layout window (approved: option A, "layer tabs on top")

- **Layer chips** along the top, one per layer, using the Oryx layer titles. The live layer is
  highlighted.
- **Following vs preview:** by default the chips and keyboard follow the live layer. Clicking a
  chip pins a preview of that layer and shows a **Follow live** chip; clicking that, or the live
  layer's chip, goes back to following.
- **Keyboard** drawn on an iced `canvas` at the physical Voyager key positions, scaled to the
  window width (minimum width ≈ 640 px).
  - The tap legend is large. The hold legend is smaller underneath.
  - Modifiers use ⌃ ⌥ ⌘ ⇧ on macOS and words on Linux.
  - Layer keys get the accent border and the target layer's name.
  - Empty or no-op keys are dimmed. Transparent keys are dimmed and marked once their encoding
    is confirmed (section 4.6, item 6).
  - **Pressed keys** fill with the press colour from `KEYDOWN` until `KEYUP`. Highlights are
    tied to physical key positions, so a held layer key stays lit after the layer changes. All
    pressed highlights clear on disconnect, so a missed `KEYUP` can't leave a key stuck.
- **Footer:** connection dot and state, "{layout title} · rev {revision}", and the current input
  source.
- **Disconnected:** the last-known layout stays visible, greyed out, with "Waiting for Voyager…".
- **No layout available** (offline and nothing cached): the keyboard shows key outlines only,
  with "Layout unavailable — will retry when online."

### 6.3 Settings window (approved as mocked)

1. **Input source per layer** — one row per layer (index and title), each with a dropdown of
   "No change" plus every enabled, select-capable source the OS reports. A missing or uninstalled
   source shows a warning icon next to that row.
2. **When the keyboard disconnects or Downlink quits** — a single "Switch to" dropdown, or
   "No change".
3. **General** — Appearance (System / Light / Dark) and Launch at login (on/off).
4. **Keyboard** — model, layout title, revision, protocol version. "Layout data: cached {age}"
   with a **Refresh from Oryx** button.

Changes take effect immediately and are saved straight away. There is no Save button.

### 6.4 Input-source switching rules (ported from the Python service)

1. When the active layer `L` changes and switching is not paused:
   - if `L` is mapped to source `S` and `S` ≠ the last source Downlink selected, select `S`;
   - if `L` is "No change" (transient), do nothing.
2. When the keyboard disconnects (or is unsupported) and a fallback is set, select the fallback.
   Then forget the last-selected source, so the next connect switches again.
3. On quit, select the fallback (if set) before exiting.
4. On connect, the first `LAYER` event (sent right after pairing) applies rule 1.
5. If the user changes the input source by hand, Downlink doesn't fight it. Rule 1 only acts on
   layer changes. "Last selected" means "last selected by Downlink", so a later change to the
   same layer will still switch.
6. Pausing stops rules 1–3. Resuming does nothing until the next layer change.

Rules 1–3 and 5 match the Python service's behavior. Rule 6 (pause) is new.

### 6.5 Theme

- `Appearance = System` follows `iced::system::theme()` at startup and
  `system::theme_changes()` afterwards.
- `Light` and `Dark` force a palette.
- Two custom iced `Theme`s (light and dark), with the colours from the mockups
  (dark: bg `#1d1f24`, key `#30343c`, accent `#4cc2ff`, press `#ffb547`; light: bg `#f4f5f7`,
  key `#e9ebef`, accent `#0a84ff`, press `#ff9f0a`), applied to every window.

## 7. Configuration, cache and paths

The config is TOML, created with defaults on first run. It isn't committed anywhere, so it may
contain the user's own input-source IDs:

```toml
appearance = "system"          # "system" | "light" | "dark"
launch_at_login = true

[switching]
fallback = "com.apple.keylayout.US"   # or omit for "No change"

[switching.layers]               # keyed by Oryx layer index; unmapped = "No change"
"0" = "com.example.keylayout.english"
"1" = "com.example.keylayout.romanian"
"2" = "com.example.keylayout.hungarian"
```

Mappings are keyed by layer **index**, because that is what the firmware reports. If a layer is
reordered in Oryx, the mapping follows the index, not the title. The Settings window shows
titles next to indices so this is visible.

| What | macOS | Linux (later) |
|------|-------|---------------|
| Config | `~/Library/Application Support/Downlink/config.toml` | `$XDG_CONFIG_HOME/downlink/config.toml` |
| Cache (layouts, metadata) | `~/Library/Caches/Downlink/` | `$XDG_CACHE_HOME/downlink/` |
| Logs (rotating, `tracing`) | `~/Library/Logs/Downlink/` | `$XDG_STATE_HOME/downlink/` |
| Single-instance lock | in the cache dir | in `$XDG_RUNTIME_DIR` |

Paths come from the `directories` crate where possible.

## 8. Errors and edge cases

| Situation | Behavior |
|-----------|----------|
| Keyboard unplugged, or the host sleeps | A read error leads to `Disconnected`, then rule 6.4.2 (fallback), then enumerating every 1 s again. The menu says "Waiting for Voyager…". |
| The HID interface won't open (permission denied, busy) | The menu shows "Can't open keyboard: {reason}". Retry with backoff (1 s, doubling up to 10 s). |
| No `PAIRING_SUCCESS` or `LAYER` within 2 s | Treated as non-Oryx firmware: `Unsupported`, the menu says so, and switching is inactive. Check again on the next reconnect. |
| Protocol version below 4 or above 5 | Log a warning and carry on; only the core events are used. |
| Layout fetch fails | Use the cache if present. Otherwise "Layout unavailable", and retry on the next connect or with **Refresh**. Switching doesn't depend on the layout. |
| Private layout (fetch returns no layout) | Shown as "Layout unavailable (private layout)". Signing in is out of scope. |
| A configured source is missing | Warning icon in Settings. The switch is skipped and a warning is logged once per source. |
| `TISSelectInputSource` fails | Log it, don't retry in a loop. The next layer change tries again. |
| Downlink launched again | The second copy finds the lock, logs "already running", and exits. |
| Config file is corrupt | Rename it to `config.toml.bad-{timestamp}`, start with defaults, log an error. |
| Keymapp running at the same time | Allowed if the spike shows the HID interface can be shared. Otherwise the README says to quit Keymapp, and the "busy" error mentions it. |

## 9. Delivery phases

### Phase 0 — Feasibility spike (throwaway code, not merged)

Answer the six unknowns in section 4.6 with the smallest possible probes:

- a `hidapi` CLI that pairs and prints decoded events, including while Keymapp is running;
- a TIS CLI that lists sources and selects one of the custom layouts;
- a minimal iced `daemon` with a `tray-icon` menu that opens and closes a window, with no Dock
  icon.

Captured packets from the first probe become the test fixtures for `oryx-protocol`.

**Exit criteria:** each unknown is answered and the spec is updated where an answer changes it.
If `tray-icon` cannot be made to work with iced, stop and revisit the GUI choice. The fallback is
Tauri 2, which reuses every crate except `downlink`.

### Phase 1 — v1 (macOS)

All of sections 5–8, built test-first. The plan will break this into tasks.

### Later

A Linux port: an input-source backend (exact desktop targets decided then), XDG autostart, and
tray via StatusNotifierItem. Also an optional floating overlay and other ZSA keyboards.

## 10. Linux-readiness rules

- The only `cfg(target_os)` code lives in `input-source` backends and the `downlink` platform
  modules (launching at login, paths, single-instance lock, bundle-only behavior such as
  `LSUIElement`).
- Every other crate builds and tests on Linux unchanged. CI (when added) runs `cargo test` for
  the workspace on Linux as well as macOS.
- Keyboard geometry is data (a table per model), so adding a Moonlander or Ergodox later means
  adding a table and a geometry string, not code.

## 11. Testing strategy

Code is written test-first: a failing test, then the implementation, then a refactor.

| Crate | Tests |
|-------|-------|
| `oryx-protocol` | Encode and decode every in-scope code. Captured real packets from Phase 0. Malformed input: short packets, missing stop byte, unknown codes. |
| `oryx-device` | The loop driven through a fake `Transport`: connect, handshake order, event forwarding, read error → `Disconnected`, re-enumeration, `Unsupported` timeout, backoff. |
| `oryx-layout` | Parsing a trimmed real GraphQL response (fixture). Oryx index ↔ matrix ↔ geometry, checked both ways for all 52 keys. Label resolution order and fallbacks. Cache read and write, keyed by revision. |
| `companion-core` | Every rule in 6.4 as a separate test (transient layers, no redundant switch, fallback on disconnect and quit, pause, first `LAYER` after connect), plus the preview/follow state and pressed-set clearing. |
| `input-source` | The fake backend in core tests. On macOS, `#[ignore]`d smoke tests (list, current, select and back) run by hand. |
| `downlink` | Theme mapping and tray-state derivation as plain functions. Windows are checked by hand. |

**Manual hardware checklist** (before calling v1 done): cold start with the keyboard plugged in;
plug in after start; unplug and replug; sleep and wake (at least 3 cycles); each of the 12 layers
shown correctly; switching for layers 0/1/2; transient layers keep the current source; fallback
on unplug and on quit; theme follows the OS appearance live; launch at login after a reboot.

## 12. Packaging and migration

- `packaging/macos/bundle.sh` builds a release binary and puts together `Downlink.app`:
  - `Info.plist` with `CFBundleIdentifier = com.zayatura.downlink` and `LSUIElement = true`;
  - an icon;
  - ad-hoc code signing (`codesign -s -`).
- **Launch at login:** a user LaunchAgent at
  `~/Library/LaunchAgents/com.zayatura.downlink.plist`, with `RunAtLoad = true` and
  `KeepAlive = { SuccessfulExit = false }`, so it restarts after a crash but stays quit after
  Quit. Written and removed by the Settings toggle. `SMAppService` isn't used because it needs a
  Developer ID-signed app.
- **README migration steps:** turn off Keymapp's auto-start (or uninstall it), then unload and
  remove the Python service's LaunchAgent. Downlink takes over the same input-source behavior.
- **Toolchain:** stable Rust via `rustup`. The minimum supported Rust version is whatever iced
  0.14 requires.

## 13. Risks

| Risk | Mitigation |
|------|------------|
| tray-icon and iced don't integrate on macOS | Phase 0 probe. The fallback GUI is Tauri 2, keeping all the non-GUI crates. |
| Input Monitoring permission needed for HID | Phase 0 probe. If needed, show a first-run explanation and a status-menu hint. |
| Oryx GraphQL schema changes, or the API stops working without a login | Cache every fetched revision. The UI degrades to key outlines, and switching keeps working. |
| `TISSelectInputSource` unreliable for some sources | Phase 0 probe with the real custom layouts. If needed, use the workaround `macism` uses. |
| The firmware protocol changes (v6+) | Only core events are used. Unknown codes are logged and ignored. |
