# Manual hardware checklist (spec §11)

Run against the bundled app (`packaging/macos/bundle.sh`, then open `target/bundle/Downlink.app`).
Record pass/fail and the date for each item.

Last run: 2026-10-13, bundle built from `feat/v1` at a14fa7a. All ticked items passed; launch at
login and offline are still to do.

- [x] Cold start with the keyboard plugged in: connected within 2 s, no Dock icon.
- [x] Plug the keyboard in after start: connects within 2 s.
- [x] Unplug and replug: fallback source on unplug, icon dims, reconnects.
- [x] Sleep and wake, three cycles: events resume each time without restarting Downlink.
- [x] All 12 layers show the right legends in the layout window.
- [x] Layers mapped to EN/RO/HU switch the input source; transient layers keep it.
- [x] Fallback source on unplug and on Quit.
- [x] Pause Input Switching stops switching; resuming switches on the next layer change.
- [x] Appearance "System" follows macOS Light/Dark live; Light and Dark force a palette.
- [ ] Launch at login: enable, log out and back in (or reboot), Downlink is running.
- [x] ⌘W and ⌘Q in the Layout and Settings windows close the window only; the icon stays and switching continues.
- [x] Minimize the layout window (⌘M), then choose Show Layout from the menu bar: the window comes back and takes focus.
- [x] A second launch exits immediately ("already running" in the log).
- [ ] Offline (Wi-Fi off) with a cached layout: the layout still shows, switching works.
- [x] Logs in `~/Library/Logs/Downlink/` have no ERROR lines during the above.
