use downlink::layout_fetch::{FetchOutcome, resolve_fetch};

const VALID: &str = r#"{"data":{"layout":{"title":"Fresh","geometry":"voyager","hashId":"abc12","privacy":false,
    "revision":{"hashId":"rev99","qmkVersion":"25.0","layers":[
        {"position":0,"title":"Main","color":"","keys":[],"builtIn":null,"hashId":"x","prevHashId":null}]}}}}"#;
const CACHED: &str = r#"{"data":{"layout":{"title":"Cached","geometry":"voyager","hashId":"abc12","privacy":false,
    "revision":{"hashId":"rev98","qmkVersion":"25.0","layers":[
        {"position":0,"title":"Main","color":"","keys":[],"builtIn":null,"hashId":"x","prevHashId":null}]}}}}"#;

fn use_title(outcome: &FetchOutcome) -> &str {
    match outcome {
        FetchOutcome::Use { layout, .. } => &layout.title,
        other => panic!("expected Use, got {other:?}"),
    }
}

#[test]
fn a_fresh_fetch_that_parses_is_used_and_stored_without_touching_the_cache() {
    let outcome = resolve_fetch(Ok(VALID.into()), false, || {
        panic!("the cache must not be read")
    });
    assert_eq!(use_title(&outcome), "Fresh");
    let FetchOutcome::Use {
        store_json,
        fallback,
        ..
    } = outcome
    else {
        unreachable!()
    };
    assert_eq!(store_json.as_deref(), Some(VALID));
    assert_eq!(fallback, None);
}

#[test]
fn a_network_error_falls_back_to_a_readable_cache_and_stores_nothing() {
    let outcome = resolve_fetch(Err("network down".into()), false, || Some(CACHED.into()));
    assert_eq!(use_title(&outcome), "Cached");
    let FetchOutcome::Use {
        store_json,
        fallback,
        ..
    } = outcome
    else {
        unreachable!()
    };
    assert_eq!(store_json, None, "the cache already holds this copy");
    assert_eq!(fallback.as_deref(), Some("network down"));
}

#[test]
fn api_json_and_not_found_failures_also_fall_back_to_a_readable_cache() {
    let bad_bodies = [
        (r#"{"errors":[{"message":"boom"}],"data":null}"#, "boom"),
        ("<html>502 Bad Gateway</html>", "not valid JSON"),
        (r#"{"data":{"layout":null}}"#, "not found"),
    ];
    for (body, expected_reason) in bad_bodies {
        let outcome = resolve_fetch(Ok(body.into()), false, || Some(CACHED.into()));
        assert_eq!(use_title(&outcome), "Cached", "body: {body}");
        let FetchOutcome::Use {
            store_json,
            fallback,
            ..
        } = outcome
        else {
            unreachable!()
        };
        assert_eq!(store_json, None, "a bad body is never cached: {body}");
        assert!(fallback.unwrap().contains(expected_reason), "body: {body}");
    }
}

#[test]
fn a_failed_fetch_without_a_usable_cache_fails_with_the_fetch_error() {
    assert_eq!(
        resolve_fetch(Err("network down".into()), false, || None),
        FetchOutcome::Fail("network down".into())
    );
    assert_eq!(
        resolve_fetch(Err("network down".into()), false, || Some(
            "not json".into()
        )),
        FetchOutcome::Fail("network down".into()),
        "an unreadable cache is no better than none"
    );
    let FetchOutcome::Fail(reason) = resolve_fetch(
        Ok(r#"{"errors":[{"message":"boom"}]}"#.into()),
        false,
        || None,
    ) else {
        panic!("expected Fail");
    };
    assert!(reason.contains("boom"), "{reason}");
}

#[test]
fn a_cached_copy_that_parses_is_used_and_not_stored_again() {
    let outcome = resolve_fetch(Ok(CACHED.into()), true, || panic!("already loaded"));
    assert_eq!(use_title(&outcome), "Cached");
    let FetchOutcome::Use {
        store_json,
        fallback,
        ..
    } = outcome
    else {
        unreachable!()
    };
    assert_eq!((store_json, fallback), (None, None));
}

#[test]
fn an_unparsable_cached_copy_refetches_once_and_cannot_loop() {
    assert_eq!(
        resolve_fetch(Ok("not json".into()), true, || None),
        FetchOutcome::Refetch
    );
    // The refetch is a fresh fetch (`from_cache = false`); if it fails too, the same bad cache
    // gives `Fail` rather than another `Refetch`.
    assert_eq!(
        resolve_fetch(Err("network down".into()), false, || Some(
            "not json".into()
        )),
        FetchOutcome::Fail("network down".into())
    );
}
