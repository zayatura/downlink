use std::time::Duration;

use downlink::settings_view::format_age;

#[test]
fn ages_read_naturally() {
    assert_eq!(format_age(Duration::from_secs(5)), "just now");
    assert_eq!(format_age(Duration::from_secs(90)), "1 min ago");
    assert_eq!(format_age(Duration::from_secs(59 * 60)), "59 min ago");
    assert_eq!(format_age(Duration::from_secs(2 * 3600 + 5)), "2 h ago");
    assert_eq!(format_age(Duration::from_secs(86_400)), "1 day ago");
    assert_eq!(format_age(Duration::from_secs(3 * 86_400)), "3 days ago");
}
