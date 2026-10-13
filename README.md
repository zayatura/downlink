# Downlink

A small menu-bar app for the **ZSA Voyager** keyboard:

- **Live layout view.** Shows the layout flashed on your keyboard, follows the active layer and highlights the keys you press.
- **Per-layer input-source switching.** For example: Main → English, RO layer → Romanian, HU layer → Hungarian. Transient layers (Navigation, Symbols, …) leave the input source alone.
- **Reconnects by itself** after sleep, wake and replugging.
- **Follows macOS Light/Dark** automatically.

Downlink talks to the keyboard directly over USB. You don't need Keymapp running.

> Downlink is an independent project. It is **not affiliated with or endorsed by ZSA Technology Labs**.
> "ZSA", "Voyager", "Oryx" and "Keymapp" are used only to describe compatibility.

## Requirements

- macOS 13 or newer (Linux support is planned)
- A ZSA Voyager running Oryx firmware (any layout built in Oryx)
- To build: Rust stable ≥ 1.90 (`rustup`) and Xcode Command Line Tools

## Build and install

```bash
packaging/macos/bundle.sh
cp -R target/bundle/Downlink.app /Applications/
open /Applications/Downlink.app
```

The Downlink key icon appears in the menu bar. Open **Settings…** to choose an input source for each
layer and the fallback used when the keyboard disconnects or Downlink quits. Turn on **Launch at
login** there, too.

⌘W and ⌘Q close Downlink's windows but keep it running in the menu bar. To quit, use **Quit
Downlink** in the menu-bar menu.

## Moving over from Keymapp and a layout-switcher script

1. Quit Keymapp and turn off its "start at login" option, or uninstall it. Keymapp opens the
   keyboard's raw-HID interface exclusively, so Downlink cannot connect while Keymapp is running:
   Keymapp must be quit. Downlink needs no Input Monitoring permission.
2. If you used a script or service that switched input sources from Keymapp's API, unload it,
   for example `launchctl unload ~/Library/LaunchAgents/<its label>.plist`.
3. Open Downlink's Settings and map your layers.

## Configuration

Settings are saved in `~/Library/Application Support/Downlink/config.toml`:

```toml
appearance = "system"          # "system" | "light" | "dark"
launch_at_login = true

[switching]
fallback = "com.apple.keylayout.US"

[switching.layers]               # keyed by Oryx layer index; unlisted layers don't switch
"0" = "com.apple.keylayout.US"
"1" = "com.apple.keylayout.Romanian-Standard"
```

To list the input-source IDs on your Mac: `cargo run -p input-source --example list`.

- Layout data is fetched once per revision from Oryx and cached in `~/Library/Caches/Downlink/`.
- Logs are in `~/Library/Logs/Downlink/`. Set `DOWNLINK_LOG=debug` for more detail.

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p downlink                      # run without bundling
cargo run -p oryx-device --example watch   # print raw keyboard events
```

The design is in `docs/superpowers/specs/2026-10-13-downlink-design.md`.

## License

MIT. See [LICENSE](LICENSE).
