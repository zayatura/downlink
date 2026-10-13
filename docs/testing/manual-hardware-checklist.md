# Manual hardware checklist (spec §11)

Run against the bundled app (`packaging/macos/bundle.sh`, then open `target/bundle/Downlink.app`).
Record pass/fail and the date for each item.

- [ ] Cold start with the keyboard plugged in: connected within 2 s, no Dock icon.
- [ ] Plug the keyboard in after start: connects within 2 s.
- [ ] Unplug and replug: fallback source on unplug, icon dims, reconnects.
- [ ] Sleep and wake, three cycles: events resume each time without restarting Downlink.
- [ ] All 12 layers show the right legends in the layout window.
- [ ] Layers mapped to EN/RO/HU switch the input source; transient layers keep it.
- [ ] Fallback source on unplug and on Quit.
- [ ] Pause Input Switching stops switching; resuming switches on the next layer change.
- [ ] Appearance "System" follows macOS Light/Dark live; Light and Dark force a palette.
- [ ] Launch at login: enable, log out and back in (or reboot), Downlink is running.
- [ ] ⌘W and ⌘Q in the Layout and Settings windows close the window only; the icon stays and switching continues.
- [ ] A second launch exits immediately ("already running" in the log).
- [ ] Offline (Wi-Fi off) with a cached layout: the layout still shows, switching works.
- [ ] Logs in `~/Library/Logs/Downlink/` have no ERROR lines during the above.
