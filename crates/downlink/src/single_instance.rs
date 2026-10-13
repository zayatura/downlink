use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;

/// Held for the life of the process; the OS releases it if the process dies.
pub struct InstanceLock {
    _file: File,
}

/// `Ok(None)` means another Downlink already holds the lock.
pub fn acquire(path: &Path) -> io::Result<Option<InstanceLock>> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(InstanceLock { _file: file })),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(error),
    }
}
