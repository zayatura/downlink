//! The layout window (spec §6.2): layer chips, the live keyboard and a status footer.

use std::collections::BTreeSet;

use companion_core::{Connection, LayoutState};
use iced::advanced::{Layout, Renderer as _, Widget, layout, renderer, widget};
use iced::widget::canvas::{Frame, Geometry};
use iced::widget::{button, canvas, column, row, stack, text};
use iced::{
    Border, Color, Element, Fill, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
    alignment, mouse,
};
use oryx_layout::geometry::{self, KeyGeom};
use oryx_layout::labels::{self, KeyLegend, LegendKind, ModifierStyle};
use oryx_protocol::Model;

use crate::app::{App, Message};
use crate::theme::{self, Colors};

/// The text over the keyboard while there is nothing (or not yet anything) to show.
pub fn overlay_message(layout: &LayoutState, connected: bool, retrying: bool) -> Option<String> {
    match (layout, connected) {
        (LayoutState::Loading, _) => Some("Loading layout…".to_owned()),
        (LayoutState::Unavailable(_), _) if retrying => {
            Some("Layout unavailable — will retry when online.".to_owned())
        }
        (LayoutState::Unavailable(reason), _) => Some(format!("Layout unavailable — {reason}")),
        (_, false) => Some("Waiting for Voyager…".to_owned()),
        _ => None,
    }
}

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
        .map(|layout| {
            let below = engine.layers_below(shown);
            labels::layer_legends(layout, shown, &below, modifier_style())
        })
        .unwrap_or_default();

    let message = overlay_message(
        engine.layout(),
        connected,
        engine.layout_retry_interval().is_some(),
    );
    let board = Keyboard {
        legends,
        pressed: if connected {
            engine.pressed().clone()
        } else {
            BTreeSet::new()
        },
        colors,
        dimmed: !connected,
        message,
    };
    let keyboard = stack![
        Element::new(Keycaps(board.clone())),
        canvas(board).width(Fill).height(Fill),
    ]
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

#[derive(Clone)]
struct Keyboard {
    legends: Vec<KeyLegend>,
    pressed: BTreeSet<usize>,
    colors: Colors,
    dimmed: bool,
    message: Option<String>,
}

impl Keyboard {
    /// Every key laid out in an area of `size`, with its legend and how it's painted.
    fn keys(&self, size: Size) -> Vec<(Rectangle, Option<&KeyLegend>, KeyPaint)> {
        let (width, height) = geometry::board_size(Model::Voyager);
        key_rects(
            size,
            geometry::keys(Model::Voyager),
            Size::new(width, height),
        )
        .into_iter()
        .enumerate()
        .map(|(index, rect)| {
            let legend = self.legends.get(index);
            let pressed = self.pressed.contains(&index);
            (
                rect,
                legend,
                key_paint(legend, pressed, self.dimmed, &self.colors),
            )
        })
        .collect()
    }
}

/// The legends and the overlay message, drawn over the keycaps (`Keycaps`).
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
        for (rect, legend, paint) in self.keys(bounds.size()) {
            if let Some(legend) = legend {
                draw_legend(&mut frame, &rect, legend, paint.ink, paint.sub);
            }
        }
        if let Some(message) = &self.message {
            frame.fill_text(canvas::Text {
                content: message.clone(),
                position: frame.center(),
                color: self.colors.sub_text,
                size: 16.0.into(),
                align_x: text::Alignment::Center,
                align_y: alignment::Vertical::Center,
                ..canvas::Text::default()
            });
        }
        vec![frame.into_geometry()]
    }
}

/// The keycaps, drawn as quads like iced's own containers: the quad shader computes how much of
/// each pixel the rounded rectangle covers. A canvas tessellates outlines into triangles instead,
/// and their 1-pixel curves break up at fractional display scales such as 1.25×.
struct Keycaps(Keyboard);

impl Widget<Message, Theme, Renderer> for Keycaps {
    fn size(&self) -> Size<Length> {
        Size::new(Fill, Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, Fill, Fill)
    }

    fn draw(
        &self,
        _tree: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let area = layout.bounds();
        let offset = Vector::new(area.x, area.y);
        for (rect, _, paint) in self.0.keys(area.size()) {
            let (bounds, radius) =
                keycap_quad(rect + offset, rect.width * 0.16, paint.border_width);
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border {
                        color: paint.border,
                        width: paint.border_width,
                        radius: radius.into(),
                    },
                    ..renderer::Quad::default()
                },
                paint.fill,
            );
        }
    }
}

/// How strongly the legend of a key borrowed from a layer beneath shows.
const INHERITED_ALPHA: f32 = 0.6;

/// How one key is painted. Fill and border are opaque (faded keys blended over the background);
/// legend colours keep their fade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyPaint {
    pub fill: Color,
    pub border: Color,
    pub border_width: f32,
    pub ink: Color,
    pub sub: Color,
}

pub fn key_paint(legend: Option<&KeyLegend>, pressed: bool, dimmed: bool, c: &Colors) -> KeyPaint {
    let kind = legend.map_or(LegendKind::Blank, |l| l.kind);
    let quiet = matches!(kind, LegendKind::Transparent | LegendKind::Blank) && !pressed;
    let alpha = if dimmed {
        0.45
    } else if quiet {
        0.55
    } else {
        1.0
    };
    let inherited = legend.is_some_and(|l| l.inherited);
    let (fill, ink, sub) = if pressed {
        (c.press, c.on_press, c.on_press)
    } else if inherited {
        // Falls through to a layer beneath: greyed out, as in Oryx.
        let grey = fade(c.sub_text, INHERITED_ALPHA);
        (c.key, grey, grey)
    } else {
        (c.key, c.key_text, c.sub_text)
    };
    let (border, border_width) = if kind == LegendKind::Layer && !inherited {
        (c.accent, 1.5)
    } else {
        (c.border, 1.0)
    };
    KeyPaint {
        fill: blend_over(fill, alpha, c.background),
        border: blend_over(border, alpha, c.background),
        border_width,
        ink: fade(ink, alpha),
        sub: fade(sub, alpha),
    }
}

/// `color`, faded by `alpha`, painted over `background`: an opaque colour, which looks the same
/// whichever surface format the GPU blends in.
pub fn blend_over(color: Color, alpha: f32, background: Color) -> Color {
    let a = color.a * alpha;
    let mix = |fg: f32, bg: f32| fg * a + bg * (1.0 - a);
    Color::from_rgb(
        mix(color.r, background.r),
        mix(color.g, background.g),
        mix(color.b, background.b),
    )
}

/// The quad for a key with a `radius` corner and a `width` border: a quad draws its border inside
/// its bounds, so it grows by half the width to match a border centred on the key's outline.
pub fn keycap_quad(key: Rectangle, radius: f32, width: f32) -> (Rectangle, f32) {
    let half = width / 2.0;
    let bounds = Rectangle::new(
        Point::new(key.x - half, key.y - half),
        Size::new(key.width + width, key.height + width),
    );
    (bounds, radius + half)
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
