//! Hyprland: switches the seat's XKB layout through Hyprland's IPC socket (spec §6).

pub mod devices;
pub mod names;
pub mod socket;

use crate::{InputSource, InputSourceBackend, InputSourceError};
use devices::Keyboard;

/// Resolves the socket on every call, so it keeps working across a Hyprland restart.
pub struct HyprlandBackend;

fn keyboards(socket: &std::path::Path) -> Result<Vec<Keyboard>, InputSourceError> {
    devices::parse_keyboards(&socket::request(socket, "j/devices")?)
}

impl InputSourceBackend for HyprlandBackend {
    fn list(&self) -> Result<Vec<InputSource>, InputSourceError> {
        let keyboards = keyboards(&socket::system_socket()?)?;
        let reader = devices::reading_keyboard(&keyboards).ok_or_else(devices::no_keyboards)?;
        let names = names::layout_names(
            &reader.rules,
            &reader.model,
            &reader.layout,
            &reader.variant,
        );
        Ok(devices::sources(reader, &names))
    }

    fn current(&self) -> Result<String, InputSourceError> {
        devices::current_id(&keyboards(&socket::system_socket()?)?)
    }

    fn select(&self, id: &str) -> Result<(), InputSourceError> {
        let socket = socket::system_socket()?;
        let commands = devices::switch_commands(&keyboards(&socket)?, id)?;
        let reply = socket::request(&socket, &devices::batch(&commands))?;
        devices::check_batch_reply(&reply, commands.len())
    }
}
