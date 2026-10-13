use iced::Element;
use iced::widget::{container, text};

use crate::app::{App, Message};

pub fn view(app: &App) -> Element<'_, Message> {
    let path = app.paths.config_file.display().to_string();
    container(text(format!("Settings are read from {path}")))
        .padding(20)
        .into()
}
