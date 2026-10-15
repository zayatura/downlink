//! Rebuilds the firmware's layer stack, which the keyboard never reports in full.
//!
//! Oryx only sends the highest active layer. A key a layer leaves transparent falls through to
//! the next lower layer that is *on*, so the layout window needs the whole stack. It is pieced
//! together from the layer reports and the keys pressed just before them:
//!
//! - A report below the current top switches off everything above the reported layer.
//! - A report above it stacks the layer on top (MO, LT, TG, OSL, TT), unless the key pressed
//!   last moves to that layer (`TO`): then the layer is the only one left on.
//! - `DF` replaces the default layer, the bottom of the stack.

use std::collections::BTreeSet;

use oryx_layout::Key;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayerStack {
    /// Layers switched on above the default layer.
    on: BTreeSet<u8>,
    default: u8,
    /// Layers the last pressed key moves to with `TO`, waiting for the matching report.
    pending_moves: BTreeSet<u8>,
}

impl LayerStack {
    pub fn reset(&mut self) {
        *self = LayerStack::default();
    }

    /// The highest active layer.
    pub fn top(&self) -> u8 {
        self.on.last().copied().unwrap_or(0).max(self.default)
    }

    /// The active layers (the default one included) beneath `layer`, highest first: where its
    /// transparent keys fall through to.
    pub fn below(&self, layer: u8) -> Vec<u8> {
        let mut active = self.on.clone();
        active.insert(self.default);
        active.range(..layer).rev().copied().collect()
    }

    /// A key went down; `key` is what it does on the current stack, if known.
    pub fn key_pressed(&mut self, key: Option<&Key>) {
        self.pending_moves.clear();
        let Some(key) = key else {
            return;
        };
        if let Some(layer) = key
            .tap
            .as_ref()
            .filter(|action| action.code == "DF")
            .and_then(|action| action.layer)
        {
            self.default = layer;
        }
        let slots = [&key.tap, &key.hold, &key.double_tap, &key.tap_hold];
        for action in slots.into_iter().flatten() {
            if action.code == "TO"
                && let Some(layer) = action.layer
            {
                self.pending_moves.insert(layer);
            }
        }
    }

    /// The keyboard reported `layer` as its highest active layer.
    pub fn layer_reported(&mut self, layer: u8) {
        if self.pending_moves.remove(&layer) {
            self.pending_moves.clear();
            self.on.clear();
        } else {
            self.on.retain(|&on| on <= layer);
        }
        if layer != self.default {
            self.on.insert(layer);
        }
    }
}
