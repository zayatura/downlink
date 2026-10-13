//! The layout window (spec §6.2): layer chips, the live keyboard and a status footer.

use std::collections::BTreeSet;

use companion_core::{Connection, LayoutState};
use iced::widget::canvas::{Frame, Geometry, Path, Stroke};
use iced::widget::{button, canvas, column, row, text};
use iced::{
    Border, Color, Element, Fill, Point, Rectangle, Renderer, Size, Theme, alignment, mouse,
};
use oryx_layout::geometry::{self, KeyGeom};
use oryx_layout::labels::{self, KeyLegend, LegendKind, ModifierStyle};
use oryx_protocol::Model;

use crate::app::{App, Message};
use crate::theme::{self, Colors};

/// Gap between neighbouring keys, in key units.
const KEY_GAP: f32 = 0.08;

/// Places every key inside `bounds`, keeping the board's aspect ratio and centring it.
pub fn key_rects(bounds: Size, keys: &[KeyGeom], board: Size) -> Vec<Rectangle> {
    if keys.is_empty()
        || board.width <= 0.0
        || board.height <= 0.0
        || bounds.width <= 0.0
        || bounds.height <= 0.0
    {
        return Vec::new();
    }
    let unit = (bounds.width / board.width).min(bounds.height / board.height);
    let left = (bounds.width - unit * board.width) / 2.0;
    let top = (bounds.height - unit * board.height) / 2.0;
    let gap = unit * KEY_GAP;
    keys.iter()
        .map(|key| Rectangle {
            x: left + key.x * unit + gap / 2.0,
            y: top + key.y * unit + gap / 2.0,
            width: unit - gap,
            height: unit - gap,
        })
        .collect()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let colors = theme::colors(app.is_dark());
    let engine = &app.engine;
    let layout = match engine.layout() {
        LayoutState::Ready(layout) => Some(layout.clone()),
        _ => None,
    };
    let connected = matches!(engine.connection(), Connection::Connected { .. });
    let shown = engine.shown_layer().unwrap_or(0);

    let mut chips: Vec<Element<'_, Message>> = Vec::new();
    if let Some(layout) = &layout {
        for layer in &layout.layers {
            let selected = layer.index == shown;
            let live = engine.active_layer() == Some(layer.index);
            chips.push(chip(
                engine.layer_title(layer.index),
                selected,
                live,
                Message::ChipPressed(layer.index),
                colors,
            ));
        }
    }
    if !engine.is_following() {
        chips.push(chip(
            "Follow live".into(),
            false,
            true,
            Message::FollowLive,
            colors,
        ));
    }

    let legends: Vec<KeyLegend> = layout
        .as_ref()
        .and_then(|layout| {
            let titles: Vec<String> = layout.layers.iter().map(|l| l.title.clone()).collect();
            layout
                .layers
                .iter()
                .find(|l| l.index == shown)
                .map(|layer| {
                    layer
                        .keys
                        .iter()
                        .map(|key| labels::key_legend(key, &titles, modifier_style()))
                        .collect()
                })
        })
        .unwrap_or_default();

    let message = match (engine.layout(), connected) {
        (LayoutState::Loading, _) => Some("Loading layout…".to_owned()),
        (LayoutState::Unavailable(reason), _) => Some(format!("Layout unavailable — {reason}")),
        (_, false) => Some("Waiting for Voyager…".to_owned()),
        _ => None,
    };
    let keyboard = canvas(Keyboard {
        legends,
        pressed: if connected {
            engine.pressed().clone()
        } else {
            BTreeSet::new()
        },
        colors,
        dimmed: !connected,
        message,
    })
    .width(Fill)
    .height(Fill);

    let status = engine.tray_view(|id| app.source_name(id)).status;
    let revision = layout
        .as_ref()
        .map(|l| format!("{} · rev {}", l.title, l.revision_id))
        .unwrap_or_default();
    let source = engine
        .current_source()
        .map(|id| app.source_name(id).unwrap_or_else(|| id.to_owned()))
        .unwrap_or_else(|| "—".into());
    let footer = row![
        text(status).size(13).color(if connected {
            colors.success
        } else {
            colors.sub_text
        }),
        text(revision).size(13).color(colors.sub_text),
        text(format!("⌨ {source}")).size(13).color(colors.key_text),
    ]
    .spacing(16);

    column![row(chips).spacing(6).wrap(), keyboard, footer]
        .spacing(12)
        .padding(16)
        .into()
}

fn chip<'a>(
    label: String,
    selected: bool,
    live: bool,
    on_press: Message,
    colors: Colors,
) -> Element<'a, Message> {
    let (background, foreground) = if selected {
        (colors.accent, colors.on_accent)
    } else {
        (
            colors.key,
            if live {
                colors.key_text
            } else {
                colors.sub_text
            },
        )
    };
    button(text(label).size(12))
        .padding([3, 10])
        .on_press(on_press)
        .style(
            move |_theme: &Theme, _status: button::Status| button::Style {
                background: Some(background.into()),
                text_color: foreground,
                border: Border {
                    radius: 999.0.into(),
                    width: if live && !selected { 1.0 } else { 0.0 },
                    color: colors.accent,
                },
                ..button::Style::default()
            },
        )
        .into()
}

fn modifier_style() -> ModifierStyle {
    if cfg!(target_os = "macos") {
        ModifierStyle::Symbols
    } else {
        ModifierStyle::Words
    }
}

struct Keyboard {
    legends: Vec<KeyLegend>,
    pressed: BTreeSet<usize>,
    colors: Colors,
    dimmed: bool,
    message: Option<String>,
}

impl canvas::Program<Message> for Keyboard {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (width, height) = geometry::board_size(Model::Voyager);
        let rects = key_rects(
            bounds.size(),
            geometry::keys(Model::Voyager),
            Size::new(width, height),
        );
        let c = &self.colors;
        for (index, rect) in rects.iter().enumerate() {
            let legend = self.legends.get(index);
            let pressed = self.pressed.contains(&index);
            let kind = legend.map_or(LegendKind::Blank, |l| l.kind);
            let quiet = matches!(kind, LegendKind::Transparent | LegendKind::Blank) && !pressed;
            let alpha = if self.dimmed {
                0.45
            } else if quiet {
                0.55
            } else {
                1.0
            };
            let (fill, ink, sub) = if pressed {
                (c.press, c.on_press, c.on_press)
            } else {
                (c.key, c.key_text, c.sub_text)
            };
            let layer_key = kind == LegendKind::Layer;
            let shape =
                Path::rounded_rectangle(rect.position(), rect.size(), (rect.width * 0.16).into());
            frame.fill(&shape, fade(fill, alpha));
            frame.stroke(
                &shape,
                Stroke::default()
                    .with_width(if layer_key { 1.5 } else { 1.0 })
                    .with_color(fade(if layer_key { c.accent } else { c.border }, alpha)),
            );
            if let Some(legend) = legend {
                draw_legend(&mut frame, rect, legend, fade(ink, alpha), fade(sub, alpha));
            }
        }
        if let Some(message) = &self.message {
            frame.fill_text(canvas::Text {
                content: message.clone(),
                position: frame.center(),
                color: c.sub_text,
                size: 16.0.into(),
                align_x: text::Alignment::Center,
                align_y: alignment::Vertical::Center,
                ..canvas::Text::default()
            });
        }
        vec![frame.into_geometry()]
    }
}

fn fade(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha,
        ..color
    }
}

/// Shrinks long legends so they fit on a key.
fn legend_size(base: f32, label: &str) -> f32 {
    let chars = label.chars().count() as f32;
    let size = if chars <= 4.0 {
        base
    } else {
        base * 4.0 / chars
    };
    size.max(7.0)
}

fn draw_legend(frame: &mut Frame, rect: &Rectangle, legend: &KeyLegend, ink: Color, sub: Color) {
    let center = rect.center();
    let unit = rect.width;
    let tap_y = if legend.hold.is_some() {
        center.y - unit * 0.12
    } else {
        center.y
    };
    frame.fill_text(canvas::Text {
        content: legend.tap.clone(),
        position: Point::new(center.x, tap_y),
        color: ink,
        size: legend_size(unit * 0.30, &legend.tap).into(),
        align_x: text::Alignment::Center,
        align_y: alignment::Vertical::Center,
        ..canvas::Text::default()
    });
    if let Some(hold) = &legend.hold {
        frame.fill_text(canvas::Text {
            content: hold.clone(),
            position: Point::new(center.x, center.y + unit * 0.24),
            color: sub,
            size: legend_size(unit * 0.19, hold).into(),
            align_x: text::Alignment::Center,
            align_y: alignment::Vertical::Center,
            ..canvas::Text::default()
        });
    }
}
