//! Where the window goes on the screen. Pure geometry, in physical pixels.

/// A rectangle on the screen: the usable area of a monitor, without the
/// menu bar and the Dock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The top-left position that puts a window of `size` in the bottom right
/// corner of `area`, `margin` away from its edges.
pub fn bottom_right(area: Area, size: (f64, f64), margin: f64) -> (f64, f64) {
    let position = (
        area.x + area.width - size.0 - margin,
        area.y + area.height - size.1 - margin,
    );
    keep_inside(area, position, size)
}

/// The top-left position that hangs a window of `size` below `anchor`,
/// centered on it and `gap` away, without leaving `area`.
pub fn below(area: Area, anchor: Area, size: (f64, f64), gap: f64) -> (f64, f64) {
    let position = (
        anchor.x + anchor.width / 2.0 - size.0 / 2.0,
        anchor.y + anchor.height + gap,
    );
    keep_inside(area, position, size)
}

/// The top-left position that puts a window of `size` in the top right
/// corner of `area`, `margin` away from its edges.
pub fn top_right(area: Area, size: (f64, f64), margin: f64) -> (f64, f64) {
    let position = (area.x + area.width - size.0 - margin, area.y + margin);
    keep_inside(area, position, size)
}

/// Moves a window as little as needed for all of it to be inside `area`.
/// A window larger than the area is aligned to the area's top-left.
pub fn keep_inside(area: Area, position: (f64, f64), size: (f64, f64)) -> (f64, f64) {
    let fit = |position: f64, size: f64, start: f64, length: f64| {
        let furthest = (start + length - size).max(start);
        position.clamp(start, furthest)
    };
    (
        fit(position.0, size.0, area.x, area.width),
        fit(position.1, size.1, area.y, area.height),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Area = Area {
        x: 0.0,
        y: 25.0,
        width: 1000.0,
        height: 700.0,
    };

    #[test]
    fn bottom_right_leaves_a_margin_from_both_edges() {
        assert_eq!(bottom_right(SCREEN, (200.0, 300.0), 20.0), (780.0, 405.0));
    }

    #[test]
    fn bottom_right_follows_a_monitor_that_does_not_start_at_zero() {
        let second = Area {
            x: 1000.0,
            y: -200.0,
            width: 800.0,
            height: 600.0,
        };
        assert_eq!(bottom_right(second, (200.0, 300.0), 0.0), (1600.0, 100.0));
    }

    #[test]
    fn below_centers_the_window_under_the_anchor() {
        let icon = Area {
            x: 500.0,
            y: 0.0,
            width: 40.0,
            height: 25.0,
        };
        assert_eq!(below(SCREEN, icon, (200.0, 300.0), 6.0), (420.0, 31.0));
    }

    #[test]
    fn below_an_anchor_near_the_edge_stays_on_the_screen() {
        let icon = Area {
            x: 950.0,
            y: 0.0,
            width: 40.0,
            height: 25.0,
        };
        assert_eq!(below(SCREEN, icon, (200.0, 300.0), 6.0), (800.0, 31.0));
    }

    #[test]
    fn top_right_leaves_a_margin_from_both_edges() {
        assert_eq!(top_right(SCREEN, (200.0, 300.0), 20.0), (780.0, 45.0));
    }

    #[test]
    fn a_window_already_inside_is_not_moved() {
        assert_eq!(
            keep_inside(SCREEN, (100.0, 100.0), (200.0, 300.0)),
            (100.0, 100.0)
        );
    }

    #[test]
    fn a_window_past_the_right_or_bottom_edge_is_pulled_back() {
        assert_eq!(
            keep_inside(SCREEN, (900.0, 600.0), (200.0, 300.0)),
            (800.0, 425.0)
        );
    }

    #[test]
    fn a_window_past_the_left_or_top_edge_is_pulled_back() {
        assert_eq!(
            keep_inside(SCREEN, (-50.0, 0.0), (200.0, 300.0)),
            (0.0, 25.0)
        );
    }

    #[test]
    fn a_window_taller_than_the_screen_starts_at_the_top() {
        assert_eq!(
            keep_inside(SCREEN, (100.0, 300.0), (200.0, 900.0)),
            (100.0, 25.0)
        );
        assert_eq!(bottom_right(SCREEN, (200.0, 900.0), 20.0), (780.0, 25.0));
    }
}
