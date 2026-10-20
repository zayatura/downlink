use downlink::tray::{
    APP_SETTINGS, CLOSE_ALL_WINDOWS, CLOSE_WINDOW, MenuAction, PAUSE, QUIT, REOPEN, SETTINGS,
    SHOW_LAYOUT, menu_action,
};

#[test]
fn cmd_w_closes_the_window_and_cmd_q_closes_every_window_not_the_app() {
    assert_eq!(menu_action(CLOSE_WINDOW), Some(MenuAction::CloseWindow));
    assert_eq!(
        menu_action(CLOSE_ALL_WINDOWS),
        Some(MenuAction::CloseAllWindows)
    );
}

#[test]
fn only_the_menu_bar_quit_item_quits() {
    assert_eq!(menu_action(QUIT), Some(MenuAction::Quit));
    for id in [
        CLOSE_WINDOW,
        CLOSE_ALL_WINDOWS,
        SETTINGS,
        APP_SETTINGS,
        SHOW_LAYOUT,
        PAUSE,
        REOPEN,
    ] {
        assert_ne!(
            menu_action(id),
            Some(MenuAction::Quit),
            "{id} must not quit"
        );
    }
}

#[test]
fn the_other_items_map_to_their_actions() {
    assert_eq!(menu_action(SHOW_LAYOUT), Some(MenuAction::ShowLayout));
    assert_eq!(menu_action(SETTINGS), Some(MenuAction::ShowSettings));
    assert_eq!(menu_action(APP_SETTINGS), Some(MenuAction::ShowSettings));
    assert_eq!(menu_action(PAUSE), Some(MenuAction::TogglePause));
    assert_eq!(menu_action(REOPEN), Some(MenuAction::Reopen));
    assert_eq!(menu_action("something-else"), None);
}
