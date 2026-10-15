use std::ffi::CString;
use std::time::Duration;

use hidapi::{HidApi, HidDevice};
use oryx_protocol::{Model, PACKET_LEN};

use crate::{Discovery, Found, Transport};

/// Finds supported keyboards through hidapi (IOKit on macOS, hidraw on Linux).
#[derive(Default)]
pub struct HidDiscovery {
    api: Option<HidApi>,
}

impl HidDiscovery {
    pub fn new() -> HidDiscovery {
        HidDiscovery::default()
    }
}

impl Discovery for HidDiscovery {
    type Transport = HidTransport;

    fn find(&mut self) -> Result<Option<Found>, String> {
        if self.api.is_none() {
            self.api = Some(HidApi::new().map_err(|e| e.to_string())?);
        }
        let api = self.api.as_mut().expect("initialised above");
        api.refresh_devices().map_err(|e| e.to_string())?;
        Ok(api.device_list().find_map(|info| {
            let model = Model::from_interface(
                info.vendor_id(),
                info.product_id(),
                info.usage_page(),
                info.usage(),
            )?;
            Some(Found {
                model,
                serial: info.serial_number().map(str::to_owned),
                path: info.path().to_string_lossy().into_owned(),
            })
        }))
    }

    fn open(&mut self, found: &Found) -> Result<HidTransport, String> {
        let api = self
            .api
            .as_ref()
            .ok_or_else(|| "HID API not initialised".to_owned())?;
        let path = CString::new(found.path.clone()).map_err(|e| e.to_string())?;
        let device = api
            .open_path(&path)
            .map_err(|e| describe_open_error(&e.to_string()))?;
        Ok(HidTransport { device })
    }
}

pub struct HidTransport {
    device: HidDevice,
}

impl Transport for HidTransport {
    fn read(&mut self, buf: &mut [u8; PACKET_LEN], timeout: Duration) -> Result<usize, String> {
        let millis = i32::try_from(timeout.as_millis()).unwrap_or(i32::MAX);
        self.device
            .read_timeout(buf, millis)
            .map_err(|e| e.to_string())
    }

    fn write(&mut self, packet: &[u8; PACKET_LEN]) -> Result<(), String> {
        // hidapi expects a leading report-ID byte (0x00: no numbered reports).
        let mut report = [0u8; PACKET_LEN + 1];
        report[1..].copy_from_slice(packet);
        self.device
            .write(&report)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

/// Turns hidapi's open error into something a person can act on.
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
