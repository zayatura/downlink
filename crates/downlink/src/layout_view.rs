use iced::Element;
use iced::widget::{column, container, text};

use crate::app::{App, Message};

pub fn view(app: &App) -> Element<'_, Message> {
    let tray = app.engine.tray_view(|id| app.source_name(id));
    container(column![text(tray.status).size(18), text(tray.detail)].spacing(8))
        .padding(20)
        .into()
}
