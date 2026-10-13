use oryx_layout::{Modifiers, ParseError, parse_graphql};
use serde_json::{Value, json};

fn action(code: &str) -> Value {
    json!({"code": code, "layer": null, "modifiers": null, "macro": null, "color": null, "description": null, "modifier": null})
}

fn layer_action(code: &str, layer: u8) -> Value {
    json!({"code": code, "layer": layer, "modifiers": null, "macro": null})
}

fn key(tap: Value, hold: Value) -> Value {
    json!({"tap": tap, "hold": hold, "doubleTap": null, "tapHold": null, "customLabel": null, "emoji": null, "glowColor": null})
}

fn transparent() -> Value {
    key(Value::Null, Value::Null)
}

fn layer(position: u8, title: &str, keys: Vec<Value>) -> Value {
    json!({"position": position, "title": title, "color": "", "keys": keys, "builtIn": null, "hashId": "x", "prevHashId": null})
}

fn response(layers: Vec<Value>) -> String {
    json!({"data": {"layout": {
        "title": "Test layout", "geometry": "voyager", "hashId": "abc12", "privacy": false,
        "revision": {"hashId": "rev99", "qmkVersion": "25.0", "layers": layers}
    }}})
    .to_string()
}

#[test]
fn parses_ids_title_and_layers_sorted_by_position() {
    let json = response(vec![
        layer(1, "RO", vec![transparent()]),
        layer(0, "Main", vec![key(action("KC_A"), Value::Null)]),
    ]);
    let layout = parse_graphql(&json).expect("parses");
    assert_eq!(layout.title, "Test layout");
    assert_eq!(layout.layout_id, "abc12");
    assert_eq!(layout.revision_id, "rev99");
    assert_eq!(layout.geometry, "voyager");
    let titles: Vec<&str> = layout.layers.iter().map(|l| l.title.as_str()).collect();
    assert_eq!(titles, vec!["Main", "RO"]);
    assert_eq!(layout.layers[0].index, 0);
    assert_eq!(layout.layers[0].keys[0].tap.as_ref().unwrap().code, "KC_A");
}

#[test]
fn all_null_slots_mean_transparent() {
    let layout = parse_graphql(&response(vec![layer(
        0,
        "Main",
        vec![transparent(), key(action("KC_NO"), Value::Null)],
    )]))
    .unwrap();
    assert!(layout.layers[0].keys[0].is_transparent());
    assert!(
        !layout.layers[0].keys[1].is_transparent(),
        "KC_NO is an explicit no-op, not transparent"
    );
}

#[test]
fn layer_actions_keep_their_target() {
    let layout = parse_graphql(&response(vec![layer(
        0,
        "Main",
        vec![key(action("KC_SPACE"), layer_action("MO", 3))],
    )]))
    .unwrap();
    let k = &layout.layers[0].keys[0];
    assert_eq!(k.hold.as_ref().unwrap().code, "MO");
    assert_eq!(k.hold.as_ref().unwrap().layer, Some(3));
}

#[test]
fn left_and_right_modifiers_are_merged() {
    let tap = json!({"code": "KC_F", "layer": null, "modifiers": {
        "leftAlt": false, "leftCtrl": false, "leftGui": false, "leftShift": false,
        "rightAlt": true, "rightCtrl": false, "rightGui": false, "rightShift": true
    }});
    let layout =
        parse_graphql(&response(vec![layer(1, "RO", vec![key(tap, Value::Null)])])).unwrap();
    let m = layout.layers[0].keys[0].tap.as_ref().unwrap().modifiers;
    assert_eq!(
        m,
        Modifiers {
            ctrl: false,
            alt: true,
            gui: false,
            shift: true
        }
    );
    assert!(m.any());
}

#[test]
fn custom_labels_are_kept_and_blank_ones_dropped() {
    let mut labelled = key(action("KC_S"), Value::Null);
    labelled["customLabel"] = json!("Ș");
    let mut blank = key(action("KC_T"), Value::Null);
    blank["customLabel"] = json!("  ");
    let layout = parse_graphql(&response(vec![layer(1, "RO", vec![labelled, blank])])).unwrap();
    assert_eq!(layout.layers[0].keys[0].custom_label.as_deref(), Some("Ș"));
    assert_eq!(layout.layers[0].keys[1].custom_label, None);
}

#[test]
fn null_titles_and_missing_keys_are_tolerated() {
    let json = json!({"data": {"layout": {"title": null, "geometry": "voyager", "hashId": "abc12",
        "revision": {"hashId": "rev99", "layers": [{"position": 0, "title": null}]}}}})
    .to_string();
    let layout = parse_graphql(&json).unwrap();
    assert_eq!(layout.title, "");
    assert_eq!(layout.layers[0].title, "");
    assert!(layout.layers[0].keys.is_empty());
}

#[test]
fn an_action_without_a_code_is_treated_as_absent() {
    let tap = json!({"layer": null});
    let layout = parse_graphql(&response(vec![layer(
        0,
        "Main",
        vec![key(tap, Value::Null)],
    )]))
    .unwrap();
    assert!(layout.layers[0].keys[0].tap.is_none());
}

#[test]
fn missing_layout_is_not_found() {
    let json = json!({"data": {"layout": null}}).to_string();
    assert_eq!(parse_graphql(&json), Err(ParseError::NotFound));
}

#[test]
fn graphql_errors_are_reported_when_there_is_no_layout() {
    let json = json!({"data": null, "errors": [{"message": "Layout not found"}]}).to_string();
    assert_eq!(
        parse_graphql(&json),
        Err(ParseError::Api("Layout not found".into()))
    );
}

#[test]
fn invalid_json_is_a_json_error() {
    assert!(matches!(
        parse_graphql("{not json"),
        Err(ParseError::Json(_))
    ));
}
