use serde::Deserialize;

/// A layout revision as flashed on the keyboard.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub title: String,
    pub layout_id: String,
    pub revision_id: String,
    pub geometry: String,
    /// Sorted by `index`.
    pub layers: Vec<Layer>,
}

impl Layout {
    pub fn layer(&self, index: u8) -> Option<&Layer> {
        self.layers.iter().find(|layer| layer.index == index)
    }

    /// The key at Oryx `index` on the first of `layers` that doesn't leave it transparent, and
    /// that layer's index: what the firmware runs when the key is pressed with `layers` on.
    pub fn resolve_key(
        &self,
        layers: impl IntoIterator<Item = u8>,
        index: usize,
    ) -> Option<(u8, &Key)> {
        layers.into_iter().find_map(|layer| {
            self.layer(layer)?
                .keys
                .get(index)
                .filter(|key| !key.is_transparent())
                .map(|key| (layer, key))
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub index: u8,
    pub title: String,
    /// In Oryx key order (see `geometry`).
    pub keys: Vec<Key>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Key {
    pub tap: Option<Action>,
    pub hold: Option<Action>,
    pub double_tap: Option<Action>,
    pub tap_hold: Option<Action>,
    pub custom_label: Option<String>,
}

impl Key {
    /// Oryx compiles a key whose four action slots are all empty to `KC_TRANSPARENT`; a tap
    /// that names it explicitly does the same.
    pub fn is_transparent(&self) -> bool {
        let tap_transparent = self
            .tap
            .as_ref()
            .is_none_or(|action| matches!(action.code.as_str(), "KC_TRANSPARENT" | "KC_TRNS"));
        tap_transparent
            && self.hold.is_none()
            && self.double_tap.is_none()
            && self.tap_hold.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    pub code: String,
    /// Target layer for layer actions (TO, MO, TG, …).
    pub layer: Option<u8>,
    pub modifiers: Modifiers,
}

/// Modifiers wrapped around an action. Left and right are merged for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub gui: bool,
    pub shift: bool,
}

impl Modifiers {
    pub fn any(self) -> bool {
        self.ctrl || self.alt || self.gui || self.shift
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("Oryx response is not valid JSON: {0}")]
    Json(String),
    #[error("Oryx returned an error: {0}")]
    Api(String),
    #[error("layout not found on Oryx (it may be private or deleted)")]
    NotFound,
}

/// Parses the raw JSON body returned by Oryx's `getLayout` GraphQL query.
pub fn parse_graphql(json: &str) -> Result<Layout, ParseError> {
    let response: RawResponse =
        serde_json::from_str(json).map_err(|e| ParseError::Json(e.to_string()))?;
    let Some(raw) = response.data.and_then(|data| data.layout) else {
        return Err(match response.errors.into_iter().next() {
            Some(error) => ParseError::Api(error.message),
            None => ParseError::NotFound,
        });
    };
    let mut layers: Vec<Layer> = raw
        .revision
        .layers
        .into_iter()
        .map(RawLayer::into_layer)
        .collect();
    layers.sort_by_key(|layer| layer.index);
    Ok(Layout {
        title: raw.title.unwrap_or_default(),
        layout_id: raw.hash_id,
        revision_id: raw.revision.hash_id,
        geometry: raw.geometry.unwrap_or_default(),
        layers,
    })
}

#[derive(Deserialize)]
struct RawResponse {
    data: Option<RawData>,
    #[serde(default)]
    errors: Vec<RawError>,
}

#[derive(Deserialize)]
struct RawError {
    message: String,
}

#[derive(Deserialize)]
struct RawData {
    layout: Option<RawLayout>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLayout {
    title: Option<String>,
    geometry: Option<String>,
    hash_id: String,
    revision: RawRevision,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRevision {
    hash_id: String,
    #[serde(default)]
    layers: Vec<RawLayer>,
}

#[derive(Deserialize)]
struct RawLayer {
    #[serde(default)]
    position: u8,
    title: Option<String>,
    #[serde(default)]
    keys: Vec<RawKey>,
}

impl RawLayer {
    fn into_layer(self) -> Layer {
        Layer {
            index: self.position,
            title: self.title.unwrap_or_default(),
            keys: self.keys.into_iter().map(RawKey::into_key).collect(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawKey {
    tap: Option<RawAction>,
    hold: Option<RawAction>,
    double_tap: Option<RawAction>,
    tap_hold: Option<RawAction>,
    custom_label: Option<String>,
}

impl RawKey {
    fn into_key(self) -> Key {
        Key {
            tap: self.tap.and_then(RawAction::into_action),
            hold: self.hold.and_then(RawAction::into_action),
            double_tap: self.double_tap.and_then(RawAction::into_action),
            tap_hold: self.tap_hold.and_then(RawAction::into_action),
            custom_label: self
                .custom_label
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty()),
        }
    }
}

#[derive(Deserialize)]
struct RawAction {
    code: Option<String>,
    layer: Option<u8>,
    modifiers: Option<RawModifiers>,
}

impl RawAction {
    fn into_action(self) -> Option<Action> {
        let code = self.code?;
        let m = self.modifiers.unwrap_or_default();
        let on = |left: Option<bool>, right: Option<bool>| {
            left.unwrap_or(false) || right.unwrap_or(false)
        };
        Some(Action {
            code,
            layer: self.layer,
            modifiers: Modifiers {
                ctrl: on(m.left_ctrl, m.right_ctrl),
                alt: on(m.left_alt, m.right_alt),
                gui: on(m.left_gui, m.right_gui),
                shift: on(m.left_shift, m.right_shift),
            },
        })
    }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawModifiers {
    left_ctrl: Option<bool>,
    left_alt: Option<bool>,
    left_gui: Option<bool>,
    left_shift: Option<bool>,
    right_ctrl: Option<bool>,
    right_alt: Option<bool>,
    right_gui: Option<bool>,
    right_shift: Option<bool>,
}
