//! gpui painting for the radial wheel, shared by the overlay window and the
//! settings preview.
//!
//! Geometry, colors and hit testing mirror the eframe `overlay::paint_wheel`
//! so the wheel looks and behaves the same after the port.

use crate::core::{ActionSet, WheelGeometry};
use crate::ui::{iconset, style};
use gpui::prelude::*;
use gpui::{Path, PathBuilder, Pixels, Point, Rgba, Window, div, point, px};

/// Sector fill for unselected, unhovered sectors (dark card color).
pub(crate) const BASE: Rgba = style::rgb(0x181C26);

/// Sector fill while the pointer rests on it.
pub(crate) const HOVER: Rgba = style::rgb(0x262C3A);

/// Geometry for a wheel drawn inside a square of `size` points.
///
/// Matches the eframe overlay: `min(width, height) * 0.78`.
pub(crate) fn geometry(size: f32) -> WheelGeometry {
    WheelGeometry::from_size(size * 0.78)
}

/// Top-left of the geometry square for a wheel centered at `center`.
pub(crate) fn origin(center: Point<Pixels>, geometry: &WheelGeometry) -> Point<Pixels> {
    point(
        center.x - px(geometry.center_x),
        center.y - px(geometry.center_y),
    )
}

fn polar(center: Point<Pixels>, radius: f32, angle: f32) -> Point<Pixels> {
    point(
        center.x + px(radius * angle.cos()),
        center.y + px(radius * angle.sin()),
    )
}

/// Filled annular sector between two angles (radians).
fn sector_path(
    center: Point<Pixels>,
    inner: f32,
    outer: f32,
    start: f32,
    end: f32,
) -> Option<Path<Pixels>> {
    if start >= end {
        return None;
    }
    let mut builder = PathBuilder::fill();
    builder.move_to(polar(center, outer, start));
    builder.arc_to(
        point(px(outer), px(outer)),
        px(0.),
        (end - start) > std::f32::consts::PI,
        true,
        polar(center, outer, end),
    );
    builder.line_to(polar(center, inner, end));
    if inner > 0.0 {
        builder.arc_to(
            point(px(inner), px(inner)),
            px(0.),
            false,
            false,
            polar(center, inner, start),
        );
    } else {
        builder.line_to(polar(center, 0.0, start));
    }
    builder.close();
    builder.build().ok()
}

/// Paints the wheel: sectors, center disc. Icons and titles are elements
/// positioned by the caller via [`sector_icon`] / [`center_icon`].
pub(crate) fn paint(
    window: &mut Window,
    geometry: &WheelGeometry,
    center: Point<Pixels>,
    sets: &[ActionSet],
    selected: usize,
    hovered: Option<usize>,
) {
    if sets.is_empty() {
        for half in [
            (0.0, std::f32::consts::PI),
            (std::f32::consts::PI, std::f32::consts::TAU),
        ] {
            if let Some(path) = sector_path(center, 0.0, geometry.outer_radius, half.0, half.1) {
                window.paint_path(path, BASE);
            }
        }
        return;
    }

    for (index, _) in sets.iter().enumerate() {
        let Some((start, end)) = geometry.sector_angles(index, sets.len()) else {
            continue;
        };
        let fill = if selected == index {
            style::ACCENT
        } else if hovered == Some(index) {
            HOVER
        } else {
            BASE
        };
        if let Some(path) = sector_path(
            center,
            geometry.inner_radius,
            geometry.outer_radius,
            start,
            end,
        ) {
            window.paint_path(path, fill);
        }
    }

    for half in [
        (0.0, std::f32::consts::PI),
        (std::f32::consts::PI, std::f32::consts::TAU),
    ] {
        if let Some(path) = sector_path(center, 0.0, geometry.inner_radius, half.0, half.1) {
            window.paint_path(path, BASE);
        }
    }
}

/// Icon center and size for one sector, in window coordinates.
pub(crate) fn sector_icon(
    geometry: &WheelGeometry,
    sets: &[ActionSet],
    index: usize,
    origin: Point<Pixels>,
) -> Option<(Point<Pixels>, f32)> {
    let (start, end) = geometry.sector_angles(index, sets.len())?;
    let mid = (start + end) / 2.0;
    let radius = (geometry.inner_radius + geometry.outer_radius) / 2.0;
    let center = point(
        origin.x + px(geometry.center_x + radius * mid.cos()),
        origin.y + px(geometry.center_y + radius * mid.sin()),
    );
    let size = ((geometry.outer_radius - geometry.inner_radius) * 0.45).clamp(20.0, 32.0);
    Some((center, size))
}

/// Icon center and size for the wheel hub, in window coordinates.
pub(crate) fn center_icon(geometry: &WheelGeometry, origin: Point<Pixels>) -> (Point<Pixels>, f32) {
    let center = point(
        origin.x + px(geometry.center_x),
        origin.y + px(geometry.center_y),
    );
    (center, geometry.inner_radius * 1.1)
}

/// Icon and title elements for a painted wheel, in window coordinates.
///
/// Hosts place these over the [`paint`] canvas; `title_y` is the baseline row
/// for the selected set name (`width` centers it).
pub(crate) fn children(
    geometry: &WheelGeometry,
    origin: Point<Pixels>,
    title_y: f32,
    width: f32,
    sets: &[ActionSet],
    selected: usize,
    show_title: bool,
) -> Vec<gpui::AnyElement> {
    let white = style::rgb(0xFFFFFF);
    let dim = style::rgba(0xFFFFFFDC);
    let mut out: Vec<gpui::AnyElement> = Vec::new();

    for (index, set) in sets.iter().enumerate() {
        if let Some((icon_center, icon_size)) = sector_icon(geometry, sets, index, origin) {
            let color = if index == selected { white } else { dim };
            out.push(
                div()
                    .absolute()
                    .left(icon_center.x - px(icon_size / 2.0))
                    .top(icon_center.y - px(icon_size / 2.0))
                    .child(iconset::element(set.icon, icon_size, color))
                    .into_any_element(),
            );
        }
    }

    if let Some(set) = sets.get(selected).or_else(|| sets.first()) {
        let (hub_center, hub_size) = center_icon(geometry, origin);
        out.push(
            div()
                .absolute()
                .left(hub_center.x - px(hub_size / 2.0))
                .top(hub_center.y - px(hub_size / 2.0))
                .child(iconset::element(set.icon, hub_size, white))
                .into_any_element(),
        );
        if show_title {
            out.push(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(title_y - 14.0))
                    .w(px(width))
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .px_3()
                            .py(px(3.0))
                            .rounded_full()
                            .bg(style::rgba(0x0000008C))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_size(px(16.0))
                            .text_color(white)
                            .child(crate::ui::overlay::truncate(&set.name, 32)),
                    )
                    .into_any_element(),
            );
        }
    }

    out
}
