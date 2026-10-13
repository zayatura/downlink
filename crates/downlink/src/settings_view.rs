//! The settings window (spec §6.3). Every change applies and saves immediately.

use std::collections::BTreeSet;
use std::time::{Duration, SystemTime};

use companion_core::{Appearance, Connection, LayoutState};
use iced::widget::{button, column, container, pick_list, row, scrollable, text, toggler};
use iced::{Border, Center, Element, Fill, Theme};

use crate::app::{App, Message};
use crate::source_choice::SourceChoice;
use crate::theme::{self, Colors};

const TRANSIENT_NOTE: &str =
    "“No change” = transient layer: keep whatever is active. Sources are read live from the OS.";

pub fn view(app: &App) -> Element<'_, Message> {
    let colors = theme::colors(app.is_dark());
    let engine = &app.engine;
    let config = engine.config();
    let options = SourceChoice::options(&app.sources);

    let mut layer_rows = column![];
    for index in layer_indices(app) {
        let selected =
            SourceChoice::current(config.layers.get(&index).map(String::as_str), &app.sources);
        layer_rows = layer_rows.push(setting_row(
            row![
                text(index.to_string()).width(28).color(colors.sub_text),
                text(engine.layer_title(index))
            ]
            .into(),
            pick_list(options.clone(), Some(selected), move |choice| {
                Message::SetLayerSource(index, choice)
            })
            .width(240)
            .into(),
        ));
    }

    let fallback = SourceChoice::current(config.fallback.as_deref(), &app.sources);
    let fallback_row = setting_row(
        text("Switch to").into(),
        pick_list(options.clone(), Some(fallback), Message::SetFallback)
            .width(240)
            .into(),
    );

    let appearance = row![
        appearance_button("System", Appearance::System, config.appearance, colors),
        appearance_button("Light", Appearance::Light, config.appearance, colors),
        appearance_button("Dark", Appearance::Dark, config.appearance, colors),
    ]
    .spacing(2);
    let mut general = column![
        setting_row(text("Appearance").into(), appearance.into()),
        setting_row(
            text("Launch at login").into(),
            toggler(config.launch_at_login)
                .on_toggle(Message::SetLaunchAtLogin)
                .into(),
        ),
    ];
    if let Some(error) = &app.launch_error {
        general = general.push(text(error.clone()).size(12).color(colors.danger));
    }

    scrollable(
        column![
            section(
                "Input source per layer",
                layer_rows.into(),
                TRANSIENT_NOTE,
                colors
            ),
            section(
                "When the keyboard disconnects or Downlink quits",
                fallback_row,
                "",
                colors
            ),
            section("General", general.into(), "", colors),
            section("Keyboard", keyboard_section(app, colors), "", colors),
        ]
        .spacing(18)
        .padding(18),
    )
    .into()
}

/// Every layer of the loaded layout; without one, the configured and live layers.
fn layer_indices(app: &App) -> Vec<u8> {
    if let LayoutState::Ready(layout) = app.engine.layout() {
        return layout.layers.iter().map(|l| l.index).collect();
    }
    let mut indices: BTreeSet<u8> = app.engine.config().layers.keys().copied().collect();
    indices.extend(app.engine.active_layer());
    if indices.is_empty() {
        indices.extend(0..3);
    }
    indices.into_iter().collect()
}

fn keyboard_section(app: &App, colors: Colors) -> Element<'_, Message> {
    let engine = &app.engine;
    let (summary, details, ids) = match engine.connection() {
        Connection::Connected {
            model,
            ids,
            protocol,
        } => {
            let title = match engine.layout() {
                LayoutState::Ready(layout) => layout.title.clone(),
                _ => "—".into(),
            };
            let revision = ids
                .as_ref()
                .map(|i| i.revision_id().to_owned())
                .unwrap_or_else(|| "unknown".into());
            let protocol = protocol
                .map(|p| format!("protocol v{p}"))
                .unwrap_or_else(|| "protocol ?".into());
            (
                format!("{} · {title}", model.name()),
                format!("rev {revision} · {protocol}"),
                ids.clone(),
            )
        }
        _ => ("Not connected".to_owned(), String::new(), None),
    };
    let cached = ids
        .as_ref()
        .and_then(|ids| app.cache.modified(ids))
        .map(|time| {
            format!(
                "cached {}",
                format_age(SystemTime::now().duration_since(time).unwrap_or_default())
            )
        })
        .unwrap_or_else(|| "not cached".into());
    let refresh = button(text("Refresh from Oryx").size(12))
        .on_press_maybe(ids.is_some().then_some(Message::RefreshLayout));
    column![
        setting_row(
            text(summary).into(),
            text(details).color(colors.sub_text).into()
        ),
        setting_row(
            text("Layout data").into(),
            row![text(cached).color(colors.sub_text), refresh]
                .spacing(8)
                .align_y(Center)
                .into(),
        ),
    ]
    .into()
}

fn section<'a>(
    title: &'a str,
    body: Element<'a, Message>,
    note: &'a str,
    colors: Colors,
) -> Element<'a, Message> {
    let card = container(body)
        .padding([4, 12])
        .width(Fill)
        .style(move |_theme: &Theme| container::Style {
            background: Some(colors.panel.into()),
            border: Border {
                color: colors.border,
                width: 1.0,
                radius: 9.0.into(),
            },
            ..container::Style::default()
        });
    let mut content = column![
        text(title.to_uppercase()).size(12).color(colors.sub_text),
        card
    ]
    .spacing(6);
    if !note.is_empty() {
        content = content.push(text(note).size(11).color(colors.sub_text));
    }
    content.into()
}

fn setting_row<'a>(
    label: Element<'a, Message>,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    row![container(label).width(Fill), control]
        .align_y(Center)
        .padding([6, 0])
        .into()
}

fn appearance_button<'a>(
    label: &'a str,
    value: Appearance,
    current: Appearance,
    colors: Colors,
) -> Element<'a, Message> {
    let selected = value == current;
    button(text(label).size(12))
        .padding([2, 12])
        .on_press(Message::SetAppearance(value))
        .style(
            move |_theme: &Theme, _status: button::Status| button::Style {
                background: Some(if selected { colors.accent } else { colors.key }.into()),
                text_color: if selected {
                    colors.on_accent
                } else {
                    colors.key_text
                },
                border: Border {
                    radius: 6.0.into(),
                    ..Border::default()
                },
                ..button::Style::default()
            },
        )
        .into()
}

/// "just now", "5 min ago", "3 h ago", "1 day ago", "2 days ago".
pub fn format_age(age: Duration) -> String {
    let secs = age.as_secs();
    match secs {
        0..60 => "just now".into(),
        60..3600 => format!("{} min ago", secs / 60),
        3600..86_400 => format!("{} h ago", secs / 3600),
        _ => match secs / 86_400 {
            1 => "1 day ago".into(),
            days => format!("{days} days ago"),
        },
    }
}
