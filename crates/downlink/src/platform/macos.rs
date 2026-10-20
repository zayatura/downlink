use std::ptr::NonNull;
use std::{fs, io};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSApplication, NSApplicationActivationPolicy,
    NSApplicationWillTerminateNotification,
};
use objc2_foundation::{NSAppleEventManager, NSArray, NSNotification, NSNotificationCenter};

use super::launch_agent;

/// Shows Downlink in the Dock and ⌘Tab while a window is open, and makes it a menu-bar-only app
/// otherwise. Call on the main thread.
pub fn show_in_dock(visible: bool) {
    let Some(mtm) = MainThreadMarker::new() else {
        tracing::warn!("show_in_dock must run on the main thread");
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    let policy = if visible {
        NSApplicationActivationPolicy::Regular
    } else {
        NSApplicationActivationPolicy::Accessory
    };
    // The bundle's LSUIElement already starts Downlink as an accessory app, and macOS answers
    // a request for the policy it already has with "refused".
    if app.activationPolicy() == policy {
        return;
    }
    #[allow(unused_unsafe)]
    let changed = unsafe { app.setActivationPolicy(policy) };
    if !changed {
        tracing::warn!(visible, "macOS refused the activation policy change");
    }
}

/// Runs `on_exit` when AppKit terminates the app, which ends the process without returning from
/// the event loop: the Dock's Quit, ⌘Tab's Q, and logout.
pub fn on_terminate(on_exit: impl Fn() + 'static) {
    let block = RcBlock::new(move |_: NonNull<NSNotification>| on_exit());
    // SAFETY: no object filter, and a `None` queue runs the block synchronously on the posting
    // thread, which for this notification is the main thread.
    let observer = unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(NSApplicationWillTerminateNotification),
            None,
            None,
            &block,
        )
    };
    // The observer lives as long as the process.
    std::mem::forget(observer);
}

/// Apple event class and ID of "reopen" (`kCoreEventClass`, `kAEReopenApplication`).
const CORE_EVENT_CLASS: u32 = u32::from_be_bytes(*b"aevt");
const REOPEN_APPLICATION: u32 = u32::from_be_bytes(*b"rapp");

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `ReopenHandler` doesn't implement
    // `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Box<dyn Fn()>]
    struct ReopenHandler;

    impl ReopenHandler {
        // SAFETY: the signature an Apple event handler selector must have.
        #[unsafe(method(handleReopen:withReplyEvent:))]
        fn handle_reopen(&self, _event: *mut AnyObject, _reply: *mut AnyObject) {
            (self.ivars())();
        }
    }
);

/// Runs `on_reopen` when Finder, Spotlight or the Dock opens Downlink while it runs. macOS sends
/// the running app a reopen event then, not a second launch. Call on the main thread, after the
/// event loop has started, as AppKit installs its own handler while launching.
pub fn on_reopen(on_reopen: impl Fn() + 'static) {
    let Some(mtm) = MainThreadMarker::new() else {
        tracing::warn!("on_reopen must run on the main thread");
        return;
    };
    let handler = ReopenHandler::alloc(mtm).set_ivars(Box::new(on_reopen) as Box<dyn Fn()>);
    // SAFETY: NSObject's `init` on an allocated instance whose ivars are set.
    let handler: Retained<ReopenHandler> = unsafe { msg_send![super(handler), init] };
    let manager = NSAppleEventManager::sharedAppleEventManager();
    // SAFETY: the selector is `handleReopen:withReplyEvent:` above, with the handler signature.
    // The manager doesn't retain the handler, so it is kept for the life of the process below.
    unsafe {
        let _: () = msg_send![
            &manager,
            setEventHandler: &*handler,
            andSelector: sel!(handleReopen:withReplyEvent:),
            forEventClass: CORE_EVENT_CLASS,
            andEventID: REOPEN_APPLICATION,
        ];
    }
    std::mem::forget(handler);
}

/// Brings Downlink's windows to the front (needed for accessory apps).
pub fn activate_app() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    #[allow(unused_unsafe, deprecated)]
    unsafe {
        app.activateIgnoringOtherApps(true)
    };
}

pub fn set_launch_at_login(enabled: bool) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("could not find the home directory")?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        launch_agent::install(&home, &exe)
            .map(|_| ())
            .map_err(|e| e.to_string())
    } else {
        launch_agent::remove(&home).map_err(|e| e.to_string())
    }
}

/// Startup check (spec §12 says the Settings toggle writes the login item, so a launch only
/// repairs it): from an app bundle, rewrite the LaunchAgent if it is missing or points elsewhere.
/// Returns whether it was rewritten.
pub fn refresh_launch_at_login() -> Result<bool, String> {
    let home = dirs::home_dir().ok_or("could not find the home directory")?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let existing = match fs::read_to_string(launch_agent::agent_path(&home)) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.to_string()),
    };
    if !launch_agent::needs_refresh(existing.as_deref(), &exe) {
        return Ok(false);
    }
    launch_agent::install(&home, &exe).map_err(|e| e.to_string())?;
    Ok(true)
}

/// The current macOS appearance, `Mode::None` off the main thread. iced only learns of a change
/// through a window whose appearance it hasn't pinned yet, so Downlink asks AppKit directly: at
/// startup, when it opens a window, and on every poll while one is open.
pub fn system_mode() -> iced::theme::Mode {
    let Some(mtm) = MainThreadMarker::new() else {
        return iced::theme::Mode::None;
    };
    let appearance = NSApplication::sharedApplication(mtm).effectiveAppearance();
    // SAFETY: AppKit's appearance-name constants are immutable statics.
    let (aqua, dark) = unsafe { (NSAppearanceNameAqua, NSAppearanceNameDarkAqua) };
    match appearance.bestMatchFromAppearancesWithNames(&NSArray::from_slice(&[aqua, dark])) {
        Some(name) if &*name == dark => iced::theme::Mode::Dark,
        _ => iced::theme::Mode::Light,
    }
}

/// Nothing macOS-specific: the bundle identifies the windows.
pub fn window_platform_specific() -> iced::window::settings::PlatformSpecific {
    iced::window::settings::PlatformSpecific::default()
}
