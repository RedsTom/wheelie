//! gpui painting for the radial wheel, shared by the overlay window and the
//! settings preview.
//!
//! Geometry, hit testing and shared colors come from
//! [`crate::core::WheelGeometry`] and [`crate::ui::style`], so the preview and
//! the overlay window draw the same wheel. The wheel keeps its dark disc in
//! both themes: it floats over the desktop and must stay readable whatever is
//! behind it.

use crate::core::{ActionSet, WheelGeometry};
use crate::ui::{iconset, style};
use gpui::prelude::*;
use gpui::{Path, PathBuilder, Pixels, Point, Window, div, point, px};

/// Geometry for a wheel drawn inside a square of `size` points.
///
/// The wheel fills `min(width, height) * 0.78` of its box.
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

/// Baseline row of the selected set name beneath the wheel.
pub(crate) fn title_y(center: Point<Pixels>, geometry: &WheelGeometry) -> f32 {
    f32::from(center.y) + geometry.outer_radius + 22.0
}

fn polar(center: Point<Pixels>, radius: f32, angle: f32) -> Point<Pixels> {
    point(
        center.x + px(radius * angle.cos()),
        center.y + px(radius * angle.sin()),
    )
}

/// Adds one annular segment. A full circle needs two segments because an SVG
/// arc cannot start and end at the same point.
fn add_segment(
    builder: &mut PathBuilder,
    center: Point<Pixels>,
    inner: f32,
    outer: f32,
    start: f32,
    end: f32,
    first: bool,
) {
    let sweep = end - start;
    let outer_start = polar(center, outer, start);
    if first {
        builder.move_to(outer_start);
    } else {
        builder.line_to(outer_start);
    }
    // `arc_to` takes radii, not a diameter, and `large_arc` becomes relevant
    // only above a half turn. Radii equal to the diameter flattened the wheel.
    builder.arc_to(
        point(px(outer), px(outer)),
        px(0.),
        sweep > std::f32::consts::PI,
        true,
        polar(center, outer, end),
    );
    builder.line_to(polar(center, inner, end));
    if inner > 0.0 {
        builder.arc_to(
            point(px(inner), px(inner)),
            px(0.),
            sweep > std::f32::consts::PI,
            false,
            polar(center, inner, start),
        );
    } else {
        builder.line_to(polar(center, 0.0, start));
    }
    builder.close();
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
    if end - start >= std::f32::consts::TAU - f32::EPSILON {
        let half = std::f32::consts::PI;
        add_segment(
            &mut builder,
            center,
            inner,
            outer,
            start,
            start + half,
            true,
        );
        add_segment(
            &mut builder,
            center,
            inner,
            outer,
            start + half,
            start + std::f32::consts::TAU,
            false,
        );
    } else {
        add_segment(&mut builder, center, inner, outer, start, end, true);
    }
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
        paint_disc(window, geometry, center);
        return;
    }

    for (index, _) in sets.iter().enumerate() {
        let Some((start, end)) = geometry.sector_angles(index, sets.len()) else {
            continue;
        };
        let fill = if selected == index {
            style::ACCENT
        } else if hovered == Some(index) {
            style::WHEEL_HOVER
        } else {
            style::WHEEL_BASE
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

    paint_disc(window, geometry, center);
}

/// Center disc, drawn as two half arcs so gpui never needs a full-circle arc.
fn paint_disc(window: &mut Window, geometry: &WheelGeometry, center: Point<Pixels>) {
    for half in [
        (0.0, std::f32::consts::PI),
        (std::f32::consts::PI, std::f32::consts::TAU),
    ] {
        if let Some(path) = sector_path(center, 0.0, geometry.inner_radius, half.0, half.1) {
            window.paint_path(path, style::WHEEL_BASE);
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
    let mut out: Vec<gpui::AnyElement> = Vec::new();

    for (index, set) in sets.iter().enumerate() {
        if let Some((icon_center, icon_size)) = sector_icon(geometry, sets, index, origin) {
            let color = if index == selected {
                style::ON_WHEEL
            } else {
                style::ON_WHEEL_DIM
            };
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
                .child(iconset::element(set.icon, hub_size, style::ON_WHEEL))
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
                            .bg(style::WHEEL_TITLE_BG)
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_size(px(16.0))
                            .text_color(style::ON_WHEEL)
                            .child(crate::ui::wheel::truncate(&set.name, 32)),
                    )
                    .into_any_element(),
            );
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ActionSet;

    fn center() -> Point<Pixels> {
        point(px(200.0), px(200.0))
    }

    #[test]
    fn geometry_fits_the_wheel_box() {
        let geometry = geometry(400.0);
        assert_eq!(geometry.center_x, 156.0);
        assert!((geometry.outer_radius - 143.52).abs() < 0.001);
        assert!((geometry.inner_radius - 56.16).abs() < 0.001);
    }

    #[test]
    fn sector_paths_exist_for_one_two_and_many_sets() {
        let geometry = geometry(400.0);
        for count in [1, 2, 3, 8] {
            for index in 0..count {
                let (start, end) = geometry.sector_angles(index, count).expect("sector angles");
                assert!(
                    sector_path(
                        center(),
                        geometry.inner_radius,
                        geometry.outer_radius,
                        start,
                        end
                    )
                    .is_some(),
                    "missing path for sector {index}/{count}"
                );
            }
        }
    }

    #[test]
    fn a_single_set_gets_a_full_ring_path() {
        let geometry = geometry(400.0);
        let (start, end) = geometry.sector_angles(0, 1).expect("sector");
        assert!(end - start >= std::f32::consts::TAU - f32::EPSILON);
        assert!(
            sector_path(
                center(),
                geometry.inner_radius,
                geometry.outer_radius,
                start,
                end
            )
            .is_some()
        );
    }

    #[test]
    fn first_sector_icon_sits_above_the_hub() {
        let geometry = geometry(400.0);
        let sets = vec![ActionSet::new(
            "one".to_owned(),
            "One".to_owned(),
            crate::core::SetIcon::Star,
        )];
        let (icon, _) =
            sector_icon(&geometry, &sets, 0, origin(center(), &geometry)).expect("icon");
        assert!(icon.y < center().y);
        assert_eq!(
            title_y(center(), &geometry),
            200.0 + geometry.outer_radius + 22.0
        );
    }
}
