use crate::{InputSource, InputSourceBackend, InputSourceError};

pub struct UnsupportedBackend;

impl InputSourceBackend for UnsupportedBackend {
    fn list(&self) -> Result<Vec<InputSource>, InputSourceError> {
        Err(InputSourceError::Unsupported)
    }

    fn current(&self) -> Result<String, InputSourceError> {
        Err(InputSourceError::Unsupported)
    }

    fn select(&self, _id: &str) -> Result<(), InputSourceError> {
        Err(InputSourceError::Unsupported)
    }
}
