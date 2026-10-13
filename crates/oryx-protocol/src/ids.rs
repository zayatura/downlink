use std::fmt;

const MAX_ID_LEN: usize = 32;

/// The Oryx layout and revision a keyboard runs, from its firmware version
/// string or USB serial number (`"layoutId/revisionId"`).
///
/// Both parts are 1–32 ASCII alphanumerics, so they are safe to use in file names.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayoutIds {
    layout_id: String,
    revision_id: String,
}

impl LayoutIds {
    pub fn new(layout_id: &str, revision_id: &str) -> Option<LayoutIds> {
        (is_valid_id(layout_id) && is_valid_id(revision_id)).then(|| LayoutIds {
            layout_id: layout_id.to_owned(),
            revision_id: revision_id.to_owned(),
        })
    }

    pub fn parse(text: &str) -> Option<LayoutIds> {
        let (layout_id, revision_id) = text.trim().split_once('/')?;
        LayoutIds::new(layout_id, revision_id)
    }

    pub fn layout_id(&self) -> &str {
        &self.layout_id
    }

    pub fn revision_id(&self) -> &str {
        &self.revision_id
    }
}

fn is_valid_id(part: &str) -> bool {
    !part.is_empty() && part.len() <= MAX_ID_LEN && part.bytes().all(|b| b.is_ascii_alphanumeric())
}

impl fmt::Display for LayoutIds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.layout_id, self.revision_id)
    }
}
