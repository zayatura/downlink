use companion_core::LayerStack;
use oryx_layout::{Action, Key, Modifiers};

fn layer_key(code: &str, layer: u8) -> Key {
    Key {
        tap: Some(Action {
            code: code.into(),
            layer: Some(layer),
            modifiers: Modifiers::default(),
        }),
        ..Key::default()
    }
}

fn letter() -> Key {
    Key {
        tap: Some(Action {
            code: "KC_A".into(),
            layer: None,
            modifiers: Modifiers::default(),
        }),
        ..Key::default()
    }
}

fn below(stack: &LayerStack, layer: u8) -> Vec<u8> {
    stack.below(layer)
}

#[test]
fn a_fresh_stack_rests_on_the_default_layer() {
    let stack = LayerStack::default();
    assert_eq!(stack.top(), 0);
    assert_eq!(below(&stack, 6), vec![0]);
    assert_eq!(below(&stack, 0), Vec::<u8>::new());
}

#[test]
fn a_momentary_layer_stacks_on_top_of_a_moved_to_layer() {
    let mut stack = LayerStack::default();
    stack.key_pressed(Some(&layer_key("TO", 1)));
    stack.layer_reported(1);
    stack.key_pressed(Some(&layer_key("MO", 6)));
    stack.layer_reported(6);
    assert_eq!(stack.top(), 6);
    assert_eq!(below(&stack, 6), vec![1, 0]);
}

#[test]
fn releasing_a_momentary_layer_returns_to_the_layer_beneath() {
    let mut stack = LayerStack::default();
    stack.key_pressed(Some(&layer_key("TO", 1)));
    stack.layer_reported(1);
    stack.key_pressed(Some(&layer_key("MO", 6)));
    stack.layer_reported(6);
    stack.layer_reported(1);
    assert_eq!(stack.top(), 1);
    assert_eq!(below(&stack, 6), vec![1, 0]);
}

#[test]
fn moving_to_a_layer_switches_off_the_others() {
    let mut stack = LayerStack::default();
    stack.key_pressed(Some(&layer_key("TO", 1)));
    stack.layer_reported(1);
    stack.key_pressed(Some(&layer_key("TO", 2)));
    stack.layer_reported(2);
    assert_eq!(below(&stack, 2), vec![0]);
    assert_eq!(below(&stack, 6), vec![2, 0]);
}

#[test]
fn moving_back_to_the_default_layer_clears_the_stack() {
    let mut stack = LayerStack::default();
    stack.key_pressed(Some(&layer_key("TO", 2)));
    stack.layer_reported(2);
    stack.key_pressed(Some(&layer_key("TO", 0)));
    stack.layer_reported(0);
    assert_eq!(stack.top(), 0);
    assert_eq!(below(&stack, 6), vec![0]);
}

#[test]
fn a_layer_reached_without_a_move_key_stacks_on_top() {
    let mut stack = LayerStack::default();
    stack.key_pressed(Some(&layer_key("TO", 1)));
    stack.layer_reported(1);
    // A hold (LT) or a combo raised the layer; the last key pressed was a letter.
    stack.key_pressed(Some(&letter()));
    stack.layer_reported(3);
    assert_eq!(below(&stack, 3), vec![1, 0]);
}

#[test]
fn a_move_key_in_the_hold_slot_still_counts_as_a_move() {
    let mut stack = LayerStack::default();
    stack.key_pressed(Some(&layer_key("MO", 4)));
    stack.layer_reported(4);
    let mut key = letter();
    key.hold = layer_key("TO", 2).tap;
    stack.key_pressed(Some(&key));
    stack.layer_reported(2);
    assert_eq!(below(&stack, 2), vec![0]);
}

#[test]
fn a_used_up_move_does_not_affect_later_layer_changes() {
    let mut stack = LayerStack::default();
    stack.key_pressed(Some(&layer_key("TO", 1)));
    stack.layer_reported(1);
    stack.layer_reported(3);
    stack.layer_reported(1);
    stack.layer_reported(3);
    assert_eq!(below(&stack, 3), vec![1, 0]);
}

#[test]
fn a_default_layer_key_changes_the_bottom_of_the_stack() {
    let mut stack = LayerStack::default();
    stack.key_pressed(Some(&layer_key("DF", 1)));
    stack.layer_reported(1);
    assert_eq!(stack.top(), 1);
    assert_eq!(below(&stack, 6), vec![1]);
    assert_eq!(below(&stack, 1), Vec::<u8>::new());
}

#[test]
fn reset_forgets_everything() {
    let mut stack = LayerStack::default();
    stack.key_pressed(Some(&layer_key("DF", 2)));
    stack.layer_reported(2);
    stack.key_pressed(Some(&layer_key("MO", 6)));
    stack.layer_reported(6);
    stack.reset();
    assert_eq!(stack.top(), 0);
    assert_eq!(below(&stack, 6), vec![0]);
}

#[test]
fn the_first_report_after_a_reset_stacks_on_the_default_layer() {
    let mut stack = LayerStack::default();
    stack.layer_reported(1);
    assert_eq!(below(&stack, 1), vec![0]);
    assert_eq!(below(&stack, 6), vec![1, 0]);
}
