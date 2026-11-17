//! Radial wheel geometry used by the overlay.

/// Segment of the radial wheel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WheelSegment {
    /// Left half: rotate left.
    Left,
    /// Right half: rotate right.
    Right,
    /// Center circle: press.
    Press,
}

/// Radial wheel geometry in plain coordinates, independent from any UI crate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelGeometry {
    /// Center X coordinate.
    pub center_x: f32,
    /// Center Y coordinate.
    pub center_y: f32,
    /// Radius of the outer ring.
    pub outer_radius: f32,
    /// Radius of the center press circle.
    pub inner_radius: f32,
}

impl WheelGeometry {
    /// Creates geometry from a center point and radii.
    pub fn new(center_x: f32, center_y: f32, outer_radius: f32, inner_radius: f32) -> Self {
        Self {
            center_x,
            center_y,
            outer_radius,
            inner_radius,
        }
    }

    /// Creates geometry for a square of the given size, centered on it.
    pub fn from_size(size: f32) -> Self {
        let half = size / 2.0;
        Self::new(half, half, half * 0.92, half * 0.34)
    }

    /// Returns true when the point is inside the outer radius.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        self.distance_to(x, y) <= self.outer_radius
    }

    /// Returns the segment under a point, or `None` outside the wheel.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<WheelSegment> {
        let distance = self.distance_to(x, y);
        if distance > self.outer_radius {
            return None;
        }
        if distance <= self.inner_radius {
            return Some(WheelSegment::Press);
        }

        if x < self.center_x {
            Some(WheelSegment::Left)
        } else {
            Some(WheelSegment::Right)
        }
    }

    fn distance_to(&self, x: f32, y: f32) -> f32 {
        let dx = x - self.center_x;
        let dy = y - self.center_y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry() -> WheelGeometry {
        WheelGeometry::new(100.0, 100.0, 80.0, 30.0)
    }

    #[test]
    fn center_is_the_press_segment() {
        let geometry = geometry();
        assert_eq!(geometry.hit_test(100.0, 100.0), Some(WheelSegment::Press));
        assert_eq!(geometry.hit_test(120.0, 85.0), Some(WheelSegment::Press));
    }

    #[test]
    fn halves_map_to_left_and_right() {
        let geometry = geometry();
        assert_eq!(geometry.hit_test(20.0, 100.0), Some(WheelSegment::Left));
        assert_eq!(geometry.hit_test(50.0, 40.0), Some(WheelSegment::Left));
        assert_eq!(geometry.hit_test(180.0, 100.0), Some(WheelSegment::Right));
        assert_eq!(geometry.hit_test(150.0, 160.0), Some(WheelSegment::Right));
    }

    #[test]
    fn points_outside_are_rejected() {
        let geometry = geometry();
        assert_eq!(geometry.hit_test(100.0, 10.0), None);
        assert_eq!(geometry.hit_test(195.0, 100.0), None);
        assert!(!geometry.contains(100.0, 5.0));
    }

    #[test]
    fn boundaries_are_inclusive() {
        let geometry = geometry();
        // Exactly on the inner radius belongs to press.
        assert_eq!(geometry.hit_test(130.0, 100.0), Some(WheelSegment::Press));
        // Exactly on the outer radius belongs to the ring.
        assert_eq!(geometry.hit_test(180.0, 100.0), Some(WheelSegment::Right));
        assert!(geometry.contains(180.0, 100.0));
    }

    #[test]
    fn from_size_derives_consistent_radii() {
        let geometry = WheelGeometry::from_size(200.0);
        assert_eq!(geometry.center_x, 100.0);
        assert_eq!(geometry.center_y, 100.0);
        assert_eq!(geometry.outer_radius, 92.0);
        assert_eq!(geometry.inner_radius, 34.0);
        assert_eq!(geometry.hit_test(100.0, 100.0), Some(WheelSegment::Press));
    }
}
