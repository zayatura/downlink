use std::fmt;

use input_source::InputSource;

/// One entry in an input-source pick list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceChoice {
    NoChange,
    Source {
        id: String,
        name: String,
    },
    /// Configured, but not installed or not enabled any more.
    Missing {
        id: String,
    },
}

impl SourceChoice {
    pub fn id(&self) -> Option<&str> {
        match self {
            SourceChoice::NoChange => None,
            SourceChoice::Source { id, .. } | SourceChoice::Missing { id } => Some(id),
        }
    }

    pub fn is_missing(&self) -> bool {
        matches!(self, SourceChoice::Missing { .. })
    }

    pub fn options(sources: &[InputSource]) -> Vec<SourceChoice> {
        std::iter::once(SourceChoice::NoChange)
            .chain(sources.iter().map(|s| SourceChoice::Source {
                id: s.id.clone(),
                name: s.name.clone(),
            }))
            .collect()
    }

    pub fn current(id: Option<&str>, sources: &[InputSource]) -> SourceChoice {
        let Some(id) = id else {
            return SourceChoice::NoChange;
        };
        sources
            .iter()
            .find(|s| s.id == id)
            .map(|s| SourceChoice::Source {
                id: s.id.clone(),
                name: s.name.clone(),
            })
            .unwrap_or_else(|| SourceChoice::Missing { id: id.to_owned() })
    }
}

impl fmt::Display for SourceChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceChoice::NoChange => f.write_str("No change"),
            SourceChoice::Source { name, .. } => f.write_str(name),
            SourceChoice::Missing { id } => write!(f, "⚠ {id} (not installed)"),
        }
    }
}
