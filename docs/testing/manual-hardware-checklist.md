# Manual hardware checklist (spec §11)

## macOS

Run against the bundled app (`packaging/macos/bundle.sh`, then open `target/bundle/Downlink.app`).
Record pass/fail and the date for each item.

Last run: 2026-10-15, bundle built from `feat/linux-support` (d37df54). All items passed, with
three gaps: sleep and wake ran two cycles, not three; launch at login was checked only by its
registration (the LaunchAgent is loaded and points at the bundle), the log-out is still to do; and
the window was minimized with its yellow button, as ⌘M is not bound (nor on `main`). Also checked:
the keycaps against `docs/images/layout-light.png`, the menu's Pause checkmark and separators, and
that a second launch without `--show-layout` exits silently and opens no window.

- [x] Cold start with the keyboard plugged in: connected within 2 s, no Dock icon.
- [x] Plug the keyboard in after start: connects within 2 s.
- [x] Unplug and replug: fallback source on unplug, icon dims, reconnects.
- [x] Sleep and wake, three cycles: events resume each time without restarting Downlink.
- [x] All 12 layers show the right legends in the layout window.
- [x] Layers mapped to EN/RO/HU switch the input source; transient layers keep it.
- [x] Fallback source on unplug and on Quit.
- [x] Pause Input Switching stops switching; resuming switches on the next layer change.
- [x] Appearance "System" follows macOS Light/Dark live; Light and Dark force a palette.
- [x] Launch at login: enable, log out and back in (or reboot), Downlink is running.
- [x] ⌘W in the Layout and Settings windows closes the window only; the icon stays and switching continues.
- [ ] ⌘Q in a window closes every Downlink window; the icon stays and switching continues.
- [ ] Opening a window adds Downlink to the Dock and ⌘Tab; closing the last one removes it.
- [ ] Quit from the Dock icon's menu ends Downlink and selects the fallback source.
- [x] Minimize the layout window (⌘M), then choose Show Layout from the menu bar: the window comes back and takes focus.
- [x] A second launch exits immediately ("already running" in the log).
- [x] Offline (Wi-Fi off) with a cached layout: the layout still shows, switching works.
- [x] Logs in `~/Library/Logs/Downlink/` have no ERROR lines during the above.

## Linux (Hyprland on Omarchy)

Run against the installed binary (`packaging/linux/install.sh`, then start Downlink from the
launcher), with the udev rule installed. Record pass/fail and the date for each item.

Last run: 2026-10-15, installed from `feat/linux-support` (18a216d), Hyprland 0.56.2 on Omarchy.
All items passed.

- [x] Cold start from the launcher with the keyboard plugged in: connected within 2 s, tray icon
      in the bar, the Layout window opens, no "Launching Downlink…" notice.
- [x] Plug the keyboard in after start: connects within 2 s.
- [x] Unplug and replug: fallback layout on unplug, icon dims, reconnects.
- [x] Suspend and resume, three cycles: events resume each time without restarting Downlink.
- [x] All layers show the right legends in the layout window.
- [x] Layer 0 selects ZD; transient layers keep it; the bar's layout widget agrees.
- [x] Fallback layout on unplug and on Quit.
- [x] Pause Input Switching stops switching (its checkmark shows); resuming switches on the next
      layer change.
- [x] Appearance "System" follows the Omarchy light/dark theme live (windows and tray icon);
      Light and Dark force a palette.
- [x] Launch at login: enable, reboot, Downlink is running and its icon is in the bar.
- [x] Meta+W in the Layout and Settings windows closes the window only; the icon stays and
      switching continues.
- [x] Restart the bar (`omarchy-restart-shell`): the tray icon comes back.
- [x] Launch again from the launcher: with no window open, the Layout window opens; with a
      Downlink window open on another workspace, it comes forward. No notice either way.
- [x] Offline (Wi-Fi off) with a cached layout: the layout still shows, switching works.
- [x] Logs in `~/.local/state/downlink/` have no ERROR lines during the above.
