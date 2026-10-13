//! Decides what to do with the result of loading a layout (spec §8: if the fetch fails, use the
//! cache when there is one).

use std::sync::Arc;

use oryx_layout::{Layout, parse_graphql};

/// What the app should do next.
#[derive(Debug, Clone, PartialEq)]
pub enum FetchOutcome {
    /// Show this layout. `store_json` is a fresh response worth caching; `fallback` is the error
    /// that made us use the cached copy instead.
    Use {
        layout: Arc<Layout>,
        store_json: Option<String>,
        fallback: Option<String>,
    },
    /// The cached copy is unreadable: fetch again, skipping the cache (a fresh fetch, so a
    /// second failure ends in `Fail` and cannot loop).
    Refetch,
    /// Nothing usable: report this error.
    Fail(String),
}

/// `fetched` is the cached text when `from_cache`, otherwise the Oryx response (or the network
/// error). `cached` loads the cached text on demand and is only called after a failed fresh
/// fetch. Any failed fresh fetch (network, HTTP status, GraphQL error, bad JSON, not found) uses
/// a readable cached copy when there is one.
pub fn resolve_fetch(
    fetched: Result<String, String>,
    from_cache: bool,
    cached: impl FnOnce() -> Option<String>,
) -> FetchOutcome {
    if from_cache {
        return match fetched.ok().and_then(|json| parse_graphql(&json).ok()) {
            Some(layout) => FetchOutcome::Use {
                layout: Arc::new(layout),
                store_json: None,
                fallback: None,
            },
            None => FetchOutcome::Refetch,
        };
    }
    let failure = match fetched {
        Ok(json) => match parse_graphql(&json) {
            Ok(layout) => {
                return FetchOutcome::Use {
                    layout: Arc::new(layout),
                    store_json: Some(json),
                    fallback: None,
                };
            }
            Err(error) => error.to_string(),
        },
        Err(network) => network,
    };
    match cached().and_then(|json| parse_graphql(&json).ok()) {
        Some(layout) => FetchOutcome::Use {
            layout: Arc::new(layout),
            store_json: None,
            fallback: Some(failure),
        },
        None => FetchOutcome::Fail(failure),
    }
}
