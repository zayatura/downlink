//! macOS backend: Carbon Text Input Sources (TIS).

use std::ffi::c_void;
use std::sync::{Mutex, MutexGuard, PoisonError};

use core_foundation::base::TCFType;
use core_foundation::boolean::{CFBoolean, CFBooleanRef};
use core_foundation::string::{CFString, CFStringRef};
use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation_sys::base::{CFRelease, CFTypeRef};

use crate::{InputSource, InputSourceBackend, InputSourceError};

type TISInputSourceRef = *const c_void;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn TISCreateInputSourceList(properties: *const c_void, include_all_installed: u8)
    -> CFArrayRef;
    fn TISCopyCurrentKeyboardInputSource() -> TISInputSourceRef;
    fn TISGetInputSourceProperty(source: TISInputSourceRef, key: CFStringRef) -> *const c_void;
    fn TISSelectInputSource(source: TISInputSourceRef) -> i32;
    static kTISPropertyInputSourceID: CFStringRef;
    static kTISPropertyLocalizedName: CFStringRef;
    static kTISPropertyInputSourceCategory: CFStringRef;
    static kTISPropertyInputSourceIsSelectCapable: CFStringRef;
    static kTISCategoryKeyboardInputSource: CFStringRef;
}

pub struct TisBackend;

/// Text Input Sources is not thread-safe: concurrent calls from different threads abort the
/// process or return inconsistent lists. Every backend method holds this for its whole body.
static TIS_LOCK: Mutex<()> = Mutex::new(());

fn tis_lock() -> MutexGuard<'static, ()> {
    // The guard protects no data, so a poisoned lock is still a usable lock.
    TIS_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The enabled input sources; the CFArray is released on drop.
struct SourceList(CFArrayRef);

impl SourceList {
    fn enabled() -> Result<SourceList, InputSourceError> {
        // A null properties dictionary means "no filter"; 0 = enabled sources only.
        let array = unsafe { TISCreateInputSourceList(std::ptr::null(), 0) };
        if array.is_null() {
            Err(InputSourceError::Platform(
                "TISCreateInputSourceList returned no list".into(),
            ))
        } else {
            Ok(SourceList(array))
        }
    }

    fn iter(&self) -> impl Iterator<Item = TISInputSourceRef> + '_ {
        let count = unsafe { CFArrayGetCount(self.0) };
        (0..count).map(move |i| unsafe { CFArrayGetValueAtIndex(self.0, i) })
    }
}

impl Drop for SourceList {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0 as CFTypeRef) }
    }
}

fn string_property(source: TISInputSourceRef, key: CFStringRef) -> Option<String> {
    let value = unsafe { TISGetInputSourceProperty(source, key) };
    (!value.is_null())
        .then(|| unsafe { CFString::wrap_under_get_rule(value as CFStringRef) }.to_string())
}

fn bool_property(source: TISInputSourceRef, key: CFStringRef) -> bool {
    let value = unsafe { TISGetInputSourceProperty(source, key) };
    !value.is_null() && bool::from(unsafe { CFBoolean::wrap_under_get_rule(value as CFBooleanRef) })
}

fn source_id(source: TISInputSourceRef) -> Option<String> {
    string_property(source, unsafe { kTISPropertyInputSourceID })
}

fn is_selectable_keyboard(source: TISInputSourceRef) -> bool {
    let keyboard =
        unsafe { CFString::wrap_under_get_rule(kTISCategoryKeyboardInputSource) }.to_string();
    string_property(source, unsafe { kTISPropertyInputSourceCategory }).as_deref()
        == Some(keyboard.as_str())
        && bool_property(source, unsafe { kTISPropertyInputSourceIsSelectCapable })
}

impl InputSourceBackend for TisBackend {
    fn list(&self) -> Result<Vec<InputSource>, InputSourceError> {
        let _guard = tis_lock();
        let list = SourceList::enabled()?;
        let mut sources: Vec<InputSource> = list
            .iter()
            .filter(|source| is_selectable_keyboard(*source))
            .filter_map(|source| {
                let id = source_id(source)?;
                let name = string_property(source, unsafe { kTISPropertyLocalizedName })
                    .unwrap_or_else(|| id.clone());
                Some(InputSource { id, name })
            })
            .collect();
        sources.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
        sources.dedup_by(|a, b| a.id == b.id);
        Ok(sources)
    }

    fn current(&self) -> Result<String, InputSourceError> {
        let _guard = tis_lock();
        let source = unsafe { TISCopyCurrentKeyboardInputSource() };
        if source.is_null() {
            return Err(InputSourceError::Platform(
                "no current keyboard input source".into(),
            ));
        }
        let id = source_id(source);
        unsafe { CFRelease(source as CFTypeRef) };
        id.ok_or_else(|| InputSourceError::Platform("the current input source has no ID".into()))
    }

    fn select(&self, id: &str) -> Result<(), InputSourceError> {
        let _guard = tis_lock();
        let list = SourceList::enabled()?;
        let source = list
            .iter()
            .find(|source| {
                is_selectable_keyboard(*source) && source_id(*source).as_deref() == Some(id)
            })
            .ok_or_else(|| InputSourceError::NotFound(id.to_owned()))?;
        let status = unsafe { TISSelectInputSource(source) };
        if status == 0 {
            Ok(())
        } else {
            Err(InputSourceError::Platform(format!(
                "TISSelectInputSource failed with status {status}"
            )))
        }
    }
}
