//! Draws the menu-bar dot: one shape and one colour per verdict, so state never relies on colour alone.

use std::f64::consts::PI;

use domain::{Grade, Verdict};

/// Pixel size of the drawn icon; the OS scales it to the menu-bar height.
pub const ICON_SIZE: u32 = 44;
const RING_WIDTH: f64 = 4.0;
const DASHES: f64 = 12.0;
/// Empty border around the shapes, so anti-aliased edges are not clipped.
const MARGIN: f64 = 3.0;
/// The triangle sits a little higher, so its base aligns with the bottom of the discs.
const TRIANGLE_BASE_LIFT: f64 = 2.0;

/// State colours, mirrored from the light values of `src/styles/tokens.css` (guarded by a test).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateColour {
    Solid,
    Unstable,
    Degraded,
    Measuring,
    Offline,
}

impl StateColour {
    #[cfg(test)]
    pub const ALL: [Self; 5] = [
        Self::Solid,
        Self::Unstable,
        Self::Degraded,
        Self::Measuring,
        Self::Offline,
    ];

    #[cfg(test)]
    pub const fn to_token_name(self) -> &'static str {
        match self {
            Self::Solid => "--solid",
            Self::Unstable => "--unstable",
            Self::Degraded => "--degraded",
            Self::Measuring => "--measuring",
            Self::Offline => "--offline",
        }
    }

    /// (lightness 0..1, chroma, hue in degrees)
    #[must_use]
    pub const fn to_oklch(self) -> (f64, f64, f64) {
        match self {
            Self::Solid => (0.56, 0.13, 150.0),
            Self::Unstable => (0.62, 0.14, 70.0),
            Self::Degraded => (0.55, 0.18, 27.0),
            Self::Measuring => (0.56, 0.1, 250.0),
            Self::Offline => (0.56, 0.01, 85.0),
        }
    }

    /// OKLCH to gamma-encoded sRGB (Björn Ottosson's reference matrices). O(1).
    #[must_use]
    pub fn to_srgb(self) -> [u8; 3] {
        let (lightness, chroma, hue) = self.to_oklch();
        let (green_red, blue_yellow) = (
            chroma * hue.to_radians().cos(),
            chroma * hue.to_radians().sin(),
        );
        let long =
            (lightness + 0.396_337_777_4 * green_red + 0.215_803_757_3 * blue_yellow).powi(3);
        let medium =
            (lightness - 0.105_561_345_8 * green_red - 0.063_854_172_8 * blue_yellow).powi(3);
        let short = (lightness - 0.089_484_177_5 * green_red - 1.291_485_548 * blue_yellow).powi(3);
        let linear = [
            4.076_741_662_1 * long - 3.307_711_591_3 * medium + 0.230_969_929_2 * short,
            -1.268_438_004_6 * long + 2.609_757_401_1 * medium - 0.341_319_396_5 * short,
            -0.004_196_086_3 * long - 0.703_418_614_7 * medium + 1.707_614_701 * short,
        ];
        linear.map(encode_srgb_channel)
    }
}

impl From<Verdict> for StateColour {
    fn from(verdict: Verdict) -> Self {
        match verdict {
            Verdict::Graded(Grade::Solid) => Self::Solid,
            Verdict::Graded(Grade::Unstable) => Self::Unstable,
            Verdict::Graded(Grade::Degraded) => Self::Degraded,
            Verdict::Measuring => Self::Measuring,
            Verdict::Offline => Self::Offline,
        }
    }
}

fn encode_srgb_channel(linear: f64) -> u8 {
    let encoded = if linear <= 0.003_130_8 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    // The value is clamped to 0..=255 first, so the cast cannot truncate or wrap.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let channel = (encoded.clamp(0.0, 1.0) * 255.0).round() as u8;
    channel
}

/// Returns `size`×`size` RGBA pixels for the verdict. O(size²) time and space.
#[must_use]
pub fn render_verdict_icon(verdict: Verdict, size: u32) -> Vec<u8> {
    let colour = StateColour::from(verdict).to_srgb();
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let point = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let distance = compute_signed_distance(verdict, point, f64::from(size));
            // Anti-aliasing: full coverage one half-pixel inside the edge, none one half-pixel outside.
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let alpha = ((0.5 - distance).clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba.extend_from_slice(&[colour[0], colour[1], colour[2], alpha]);
        }
    }
    rgba
}

/// Negative inside the shape, positive outside, in pixels. O(1).
fn compute_signed_distance(verdict: Verdict, (x, y): (f64, f64), size: f64) -> f64 {
    let centre = size / 2.0;
    let radius = centre - MARGIN;
    let from_centre = (x - centre).hypot(y - centre);
    let disc = from_centre - radius;
    let ring = (from_centre - (radius - RING_WIDTH / 2.0)).abs() - RING_WIDTH / 2.0;
    match verdict {
        Verdict::Graded(Grade::Solid) => disc,
        Verdict::Graded(Grade::Unstable) => ring.min(disc.max(centre - x)),
        Verdict::Graded(Grade::Degraded) => compute_triangle_distance((x, y), size),
        Verdict::Offline => {
            // A "/" through the centre, kept inside the disc.
            let slash =
                ((x - centre) + (y - centre)).abs() / std::f64::consts::SQRT_2 - RING_WIDTH / 2.0;
            ring.min(slash.max(disc))
        }
        Verdict::Measuring => {
            let turn = ((y - centre).atan2(x - centre) + PI) / (2.0 * PI);
            let in_dash = (turn * DASHES).floor().rem_euclid(2.0) < 1.0;
            if in_dash { ring } else { f64::MAX }
        }
    }
}

/// Upward triangle filling the icon, as the max of the distances to its three edges.
fn compute_triangle_distance(point: (f64, f64), size: f64) -> f64 {
    let bottom = size - MARGIN - TRIANGLE_BASE_LIFT;
    let top = (size / 2.0, MARGIN);
    let left = (MARGIN, bottom);
    let right = (size - MARGIN, bottom);
    [(top, right), (right, left), (left, top)]
        .into_iter()
        .map(|(from, to)| compute_edge_distance(point, from, to))
        .fold(f64::MIN, f64::max)
}

/// Signed distance to the line through `from` and `to`, positive on the outer (left-hand) side.
fn compute_edge_distance((x, y): (f64, f64), from: (f64, f64), to: (f64, f64)) -> f64 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    ((x - from.0) * dy - (y - from.1) * dx) / dx.hypot(dy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{Grade, Verdict};

    const SIZE: u32 = ICON_SIZE;

    fn read_pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
        let index = ((y * SIZE + x) * 4) as usize;
        [
            rgba[index],
            rgba[index + 1],
            rgba[index + 2],
            rgba[index + 3],
        ]
    }

    fn is_opaque(pixel: [u8; 4]) -> bool {
        pixel[3] == 255
    }

    fn is_transparent(pixel: [u8; 4]) -> bool {
        pixel[3] == 0
    }

    #[test]
    fn draws_solid_as_a_filled_disc_in_the_solid_colour() {
        // Act
        let rgba = render_verdict_icon(Verdict::Graded(Grade::Solid), SIZE);

        // Assert
        let centre = read_pixel(&rgba, SIZE / 2, SIZE / 2);
        assert!(is_opaque(centre));
        assert_eq!(centre[..3], StateColour::Solid.to_srgb());
        assert!(is_transparent(read_pixel(&rgba, 0, 0)));
    }

    #[test]
    fn draws_unstable_as_a_ring_filled_on_its_right_half() {
        // Act
        let rgba = render_verdict_icon(Verdict::Graded(Grade::Unstable), SIZE);

        // Assert
        assert!(is_opaque(read_pixel(&rgba, SIZE / 2 + 8, SIZE / 2)));
        assert!(is_transparent(read_pixel(&rgba, SIZE / 2 - 8, SIZE / 2)));
        assert!(is_opaque(read_pixel(&rgba, 4, SIZE / 2)));
    }

    #[test]
    fn draws_degraded_as_a_triangle() {
        // Act
        let rgba = render_verdict_icon(Verdict::Graded(Grade::Degraded), SIZE);

        // Assert
        let centre = read_pixel(&rgba, SIZE / 2, SIZE / 2 + 4);
        assert!(is_opaque(centre));
        assert_eq!(centre[..3], StateColour::Degraded.to_srgb());
        assert!(is_transparent(read_pixel(&rgba, 4, 6)));
        assert!(is_transparent(read_pixel(&rgba, SIZE - 5, 6)));
    }

    #[test]
    fn draws_measuring_as_an_empty_dotted_ring() {
        // Act
        let rgba = render_verdict_icon(Verdict::Measuring, SIZE);

        // Assert
        assert!(is_transparent(read_pixel(&rgba, SIZE / 2, SIZE / 2)));
        let ring_pixels = (0..SIZE)
            .filter(|x| is_opaque(read_pixel(&rgba, *x, SIZE / 2)))
            .count();
        assert!(ring_pixels > 0);
    }

    #[test]
    fn draws_offline_as_a_crossed_out_ring_in_the_offline_colour() {
        // Act
        let rgba = render_verdict_icon(Verdict::Offline, SIZE);

        // Assert
        let on_the_slash = read_pixel(&rgba, SIZE / 2, SIZE / 2);
        assert!(is_opaque(on_the_slash));
        assert_eq!(on_the_slash[..3], StateColour::Offline.to_srgb());
        assert!(is_opaque(read_pixel(&rgba, 4, SIZE / 2)));
        assert!(is_transparent(read_pixel(
            &rgba,
            SIZE / 2 - 8,
            SIZE / 2 - 8
        )));
    }

    // The state colours exist twice (CSS for the windows, Rust for the tray); this keeps them equal.
    #[test]
    fn uses_the_same_state_colours_as_the_design_tokens() {
        // Arrange
        let tokens = include_str!("../../src/styles/tokens.css");

        // Act & Assert
        for colour in StateColour::ALL {
            let declared = read_light_token(tokens, colour.to_token_name());
            assert_eq!(
                declared,
                colour.to_oklch(),
                "{} drifted from tokens.css",
                colour.to_token_name()
            );
        }
    }

    fn read_custom_property(block: &str, name: &str) -> String {
        block
            .lines()
            .find_map(|line| line.trim().strip_prefix(&format!("{name}:")))
            .map_or_else(
                || panic!("{name} is missing from tokens.css"),
                |value| value.trim().trim_end_matches(';').to_owned(),
            )
    }

    fn read_light_token(tokens: &str, name: &str) -> (f64, f64, f64) {
        let light_block = tokens.split("@media").next().expect("a light block");
        let value = read_custom_property(light_block, name);
        // Neutrals share their hue through `var(--hue)`; resolve it like the browser does.
        let inside = value
            .strip_prefix("oklch(")
            .and_then(|rest| rest.strip_suffix(')'))
            .expect("an oklch value")
            .replace("var(--hue)", &read_custom_property(light_block, "--hue"));
        let mut parts = inside.split_whitespace();
        let lightness = parts
            .next()
            .and_then(|p| p.trim_end_matches('%').parse::<f64>().ok())
            .expect("L")
            / 100.0;
        let chroma = parts.next().and_then(|p| p.parse().ok()).expect("C");
        let hue = parts.next().and_then(|p| p.parse().ok()).expect("h");
        (lightness, chroma, hue)
    }
}
