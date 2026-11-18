//! Radial wheel geometry used by the overlay.

/// Radial wheel geometry in plain coordinates, independent from any UI crate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelGeometry {
    /// Center X coordinate.
    pub center_x: f32,
    /// Center Y coordinate.
    pub center_y: f32,
    /// Radius of the outer ring.
    pub outer_radius: f32,
    /// Radius of the center circle.
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
        Self::new(half, half, half * 0.92, half * 0.36)
    }

    /// Returns true when the point is inside the outer radius.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        self.distance_to(x, y) <= self.outer_radius
    }

    /// Returns the center point.
    pub fn center(&self) -> (f32, f32) {
        (self.center_x, self.center_y)
    }

    /// Returns true when the point is inside the center circle.
    pub fn is_center(&self, x: f32, y: f32) -> bool {
        self.distance_to(x, y) <= self.inner_radius
    }

    /// Start and end angles of a sector, in radians.
    ///
    /// The first sector is centered on the top of the wheel and sectors run
    /// clockwise; screen coordinates grow downward, so angle zero points right.
    pub fn sector_angles(&self, index: usize, count: usize) -> Option<(f32, f32)> {
        if count == 0 || index >= count {
            return None;
        }
        let step = std::f32::consts::TAU / count as f32;
        let top = -std::f32::consts::FRAC_PI_2;
        let start = top - step / 2.0 + step * index as f32;
        Some((start, start + step))
    }

    /// Sector index under a point, or `None` inside the center or outside.
    pub fn sector_at(&self, count: usize, x: f32, y: f32) -> Option<usize> {
        if count == 0 {
            return None;
        }
        let distance = self.distance_to(x, y);
        if distance <= self.inner_radius || distance > self.outer_radius {
            return None;
        }

        let dx = x - self.center_x;
        let dy = y - self.center_y;
        let angle = dy.atan2(dx);
        let top = -std::f32::consts::FRAC_PI_2;
        let step = std::f32::consts::TAU / count as f32;
        let mut relative = angle - top + step / 2.0;
        while relative < 0.0 {
            relative += std::f32::consts::TAU;
        }
        while relative >= std::f32::consts::TAU {
            relative -= std::f32::consts::TAU;
        }

        let index = (relative / step).floor() as usize;
        Some(index.min(count - 1))
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
    fn center_is_not_a_sector() {
        let geometry = geometry();
        assert!(geometry.is_center(100.0, 100.0));
        assert_eq!(geometry.sector_at(3, 100.0, 100.0), None);
        assert_eq!(geometry.sector_at(3, 125.0, 100.0), None);
    }

    #[test]
    fn sectors_split_the_ring_evenly() {
        let geometry = geometry();
        // The first sector is centered on the top of the wheel.
        assert_eq!(geometry.sector_at(3, 100.0, 30.0), Some(0));
        // Right-bottom and bottom-left are the next sectors.
        assert_eq!(geometry.sector_at(3, 160.0, 140.0), Some(1));
        assert_eq!(geometry.sector_at(3, 40.0, 140.0), Some(2));
        // Upper-left is still part of the top sector.
        assert_eq!(geometry.sector_at(3, 40.0, 60.0), Some(0));
    }

    #[test]
    fn a_single_sector_covers_the_whole_ring() {
        let geometry = geometry();
        for point in [(100.0, 30.0), (180.0, 100.0), (100.0, 170.0), (20.0, 100.0)] {
            assert_eq!(geometry.sector_at(1, point.0, point.1), Some(0));
        }
    }

    #[test]
    fn outside_the_outer_radius_is_rejected() {
        let geometry = geometry();
        assert_eq!(geometry.sector_at(3, 100.0, 5.0), None);
        assert_eq!(geometry.sector_at(3, 195.0, 100.0), None);
        assert!(!geometry.contains(100.0, 5.0));
    }

    #[test]
    fn sector_angles_are_contiguous() {
        let geometry = geometry();
        let (start, end) = geometry.sector_angles(0, 4).expect("sector");
        let quarter = std::f32::consts::FRAC_PI_2 / 2.0;
        assert!((start + std::f32::consts::FRAC_PI_2 + quarter).abs() < f32::EPSILON);
        let (next_start, _) = geometry.sector_angles(1, 4).expect("sector");
        assert!((end - next_start).abs() < f32::EPSILON);
        assert_eq!(geometry.sector_angles(4, 4), None);
        assert_eq!(geometry.sector_angles(0, 0), None);
    }

    #[test]
    fn from_size_derives_consistent_radii() {
        let geometry = WheelGeometry::from_size(200.0);
        assert_eq!(geometry.center_x, 100.0);
        assert_eq!(geometry.center_y, 100.0);
        assert_eq!(geometry.outer_radius, 92.0);
        assert_eq!(geometry.inner_radius, 36.0);
        assert!(geometry.is_center(100.0, 100.0));
    }
}
