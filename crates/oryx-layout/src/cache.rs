use std::path::PathBuf;
use std::time::SystemTime;
use std::{fs, io};

use oryx_protocol::LayoutIds;

/// Raw Oryx JSON, one file per layout revision. It is parsed on load, so a
/// model change never invalidates the cache.
#[derive(Debug, Clone)]
pub struct LayoutCache {
    dir: PathBuf,
}

impl LayoutCache {
    pub fn new(dir: impl Into<PathBuf>) -> LayoutCache {
        LayoutCache { dir: dir.into() }
    }

    /// `LayoutIds` only holds ASCII alphanumerics, so this never leaves `dir`.
    pub fn path_for(&self, ids: &LayoutIds) -> PathBuf {
        self.dir
            .join(format!("{}-{}.json", ids.layout_id(), ids.revision_id()))
    }

    pub fn load(&self, ids: &LayoutIds) -> Option<String> {
        fs::read_to_string(self.path_for(ids)).ok()
    }

    pub fn store(&self, ids: &LayoutIds, json: &str) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let path = self.path_for(ids);
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, json)?;
        fs::rename(&tmp, &path)
    }

    pub fn modified(&self, ids: &LayoutIds) -> Option<SystemTime> {
        fs::metadata(self.path_for(ids)).ok()?.modified().ok()
    }
}
