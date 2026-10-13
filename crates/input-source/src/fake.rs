use std::sync::Mutex;

use crate::{InputSource, InputSourceBackend, InputSourceError};

/// An in-memory backend for tests.
#[derive(Debug)]
pub struct FakeBackend {
    sources: Vec<InputSource>,
    current: Mutex<String>,
    selections: Mutex<Vec<String>>,
}

impl FakeBackend {
    pub fn new(sources: Vec<InputSource>, current: &str) -> FakeBackend {
        FakeBackend {
            sources,
            current: Mutex::new(current.to_owned()),
            selections: Mutex::new(Vec::new()),
        }
    }

    /// Every successful `select`, in order.
    pub fn selections(&self) -> Vec<String> {
        self.selections.lock().expect("not poisoned").clone()
    }
}

impl InputSourceBackend for FakeBackend {
    fn list(&self) -> Result<Vec<InputSource>, InputSourceError> {
        Ok(self.sources.clone())
    }

    fn current(&self) -> Result<String, InputSourceError> {
        Ok(self.current.lock().expect("not poisoned").clone())
    }

    fn select(&self, id: &str) -> Result<(), InputSourceError> {
        if !self.sources.iter().any(|s| s.id == id) {
            return Err(InputSourceError::NotFound(id.to_owned()));
        }
        *self.current.lock().expect("not poisoned") = id.to_owned();
        self.selections
            .lock()
            .expect("not poisoned")
            .push(id.to_owned());
        Ok(())
    }
}
