// SPDX-FileCopyrightText: 2020 Commoble
// SPDX-FileCopyrightText: 2024 klikli-dev
// SPDX-License-Identifier: MIT
// Wire curve geometry. Java 26.3: logistics/WireSlackHelper.java
// #getInterpolatedDifferences and logistics/WireRenderer.java#renderWire.

/// `WireSlackHelper.getInterpolatedDifferences`: 17 points (16 segments) from
/// the origin to `d`, sagging by 0.3 in the middle.
pub fn slack_points(d: [f64; 3]) -> [[f32; 3]; 17] {
    let mut points = [[0.0; 3]; 17];
    for (index, point) in points.iter_mut().enumerate() {
        let lerp = index as f64 / 16.0;
        let sag = 0.3 * (1.0 - 4.0 * (lerp - 0.5).powi(2));
        let y = if d[1] != 0.0 { (lerp.powf((d[1].abs() + 3.0).ln()) - sag) * d[1] } else { -sag };
        *point = [(lerp * d[0]) as f32, y as f32, (lerp * d[2]) as f32];
    }
    points
}

/// The four side quads of a square tube of half-width `half` around the
/// segment `a`..`b`: Horizon has no line render type, so each
/// `WireRenderer.renderWire` line becomes a thin tube along the segment.
pub fn segment_quads(a: [f32; 3], b: [f32; 3], half: f32) -> [[[f32; 3]; 4]; 4] {
    let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    // Any axis not parallel to the segment gives a valid perpendicular pair.
    let helper = if dir[0].abs() + dir[2].abs() < 1e-6 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let cross = |p: [f32; 3], q: [f32; 3]| [p[1] * q[2] - p[2] * q[1], p[2] * q[0] - p[0] * q[2], p[0] * q[1] - p[1] * q[0]];
    let unit = |v: [f32; 3]| {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
        v.map(|c| c / len * half)
    };
    let u = unit(cross(dir, helper));
    let v = unit(cross(dir, u));
    let at = |p: [f32; 3], su: f32, sv: f32| [0, 1, 2].map(|i| p[i] + su * u[i] + sv * v[i]);
    let ring = [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)];
    [0, 1, 2, 3].map(|i| {
        let (p, q) = (ring[i], ring[(i + 1) % 4]);
        [at(a, p.0, p.1), at(a, q.0, q.1), at(b, q.0, q.1), at(b, p.0, p.1)]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slack_matches_java_endpoints_and_sag() {
        let level = slack_points([4.0, 0.0, -2.0]);
        assert_eq!(level[0], [0.0, 0.0, 0.0]);
        assert_eq!(level[16], [4.0, 0.0, -2.0]);
        assert_eq!(level[8], [2.0, -0.3, -1.0]);
        // dy != 0: y = (lerp^ln(|dy|+3) - sag) * dy; ends are exact.
        let rising = slack_points([0.0, 2.0, 3.0]);
        assert_eq!(rising[16], [0.0, 2.0, 3.0]);
        let expect = (0.5f64.powf(5.0f64.ln()) - 0.3) * 2.0;
        assert!((rising[8][1] as f64 - expect).abs() < 1e-6);
    }

    // The old renderer drew each segment's bounding box: a diagonal wire
    // became block-sized panels. Every vertex must stay within `half`*sqrt(2).
    #[test]
    fn tube_stays_thin_on_diagonal_and_vertical_segments() {
        for (a, b) in [([0.0, 0.0, 0.0], [3.0, 2.0, -5.0]), ([1.0, 0.0, 1.0], [1.0, 4.0, 1.0])] {
            let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let len2: f32 = dir.iter().map(|c| c * c).sum();
            for quad in segment_quads(a, b, 0.03) {
                for p in quad {
                    let rel = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
                    let t = (rel[0] * dir[0] + rel[1] * dir[1] + rel[2] * dir[2]) / len2;
                    let off: f32 = (0..3).map(|i| (rel[i] - t * dir[i]).powi(2)).sum::<f32>().sqrt();
                    assert!((off - 0.03 * 2f32.sqrt()).abs() < 1e-4, "offset {off}");
                    assert!(t.abs() < 1e-4 || (t - 1.0).abs() < 1e-4);
                }
            }
        }
    }
}
