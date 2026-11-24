use crate::scene::{Palette, Stop};

pub type Lut = [[u8; 3]; 256];

pub(crate) fn validate(palette: &Palette) -> Result<(), String> {
    if palette.stops.is_empty() || palette.stops.len() > 256 {
        return Err("palette needs 1 to 256 stops".into());
    }
    let mut last = f64::NEG_INFINITY;
    for stop in &palette.stops {
        let in_range = |v: f64| v.is_finite() && (0.0..=1.0).contains(&v);
        if !in_range(stop.pos) || !stop.color.iter().all(|c| in_range(*c)) {
            return Err("palette positions and colors must be between 0 and 1".into());
        }
        if stop.pos < last {
            return Err("palette stops must be in ascending order".into());
        }
        last = stop.pos;
    }
    Ok(())
}

pub fn lut(palette: &Palette) -> Lut {
    let stops = &palette.stops;
    let mut out = [[0u8; 3]; 256];
    for (i, entry) in out.iter_mut().enumerate() {
        let pos = i as f64 / 255.0;
        let next = stops.iter().position(|s| s.pos >= pos).unwrap_or(stops.len());
        let color = if next == 0 {
            stops[0].color
        } else if next == stops.len() {
            stops[next - 1].color
        } else {
            let (a, b) = (&stops[next - 1], &stops[next]);
            let span = b.pos - a.pos;
            let t = if span > 0.0 { (pos - a.pos) / span } else { 1.0 };
            [0, 1, 2].map(|ch| a.color[ch] + (b.color[ch] - a.color[ch]) * t)
        };
        *entry = color.map(|c| (c * 255.0 + 0.5) as u8);
    }
    out
}

fn hex(pos: f64, rgb: u32) -> Stop {
    let ch = |shift: u32| f64::from((rgb >> shift) & 0xFF) / 255.0;
    Stop {
        pos,
        color: [ch(16), ch(8), ch(0)],
    }
}

fn build(colors: &[u32]) -> Palette {
    let last = (colors.len() - 1).max(1) as f64;
    Palette {
        stops: colors
            .iter()
            .enumerate()
            .map(|(i, c)| hex(i as f64 / last, *c))
            .collect(),
    }
}

pub const NAMES: [&str; 8] = [
    "aurora", "emerald", "ember", "violet", "ice", "sunset", "orchid", "gold",
];

// Dark palettes of one hue after the posters of rezonanca.art: the first color is the background they are laid on,
// the one before the last the warm light from below. A solid reads them as its light.
pub const STUDIO: [&str; 9] = [
    "amethyst", "cobalt", "garnet", "abyss", "lagoon", "frost", "rosewood", "bronze", "forest",
];

// What the app offers to color a layer with: the first eight, then those of one hue.
pub const SWATCHES: [&str; 17] = [
    NAMES[0], NAMES[1], NAMES[2], NAMES[3], NAMES[4], NAMES[5], NAMES[6], NAMES[7], STUDIO[0], STUDIO[1], STUDIO[2],
    STUDIO[3], STUDIO[4], STUDIO[5], STUDIO[6], STUDIO[7], STUDIO[8],
];

const AT: [f64; 6] = [0.0, 0.2, 0.45, 0.7, 0.88, 1.0];

const BACKDROPS: [(&str, [u32; 6]); 9] = [
    ("amethyst", [0x0A061E, 0x281E78, 0x7A4DFF, 0xB9A6F2, 0xFF6E8C, 0xFFD6D2]),
    ("cobalt", [0x030614, 0x142878, 0x285CE8, 0x9DB6F0, 0xFFA546, 0xFFE8C8]),
    ("garnet", [0x100408, 0x460C1A, 0xAA1E28, 0xE07A6E, 0xFF963C, 0xFFE6BE]),
    ("abyss", [0x031030, 0x0B2E70, 0x226DBC, 0x8FC4EC, 0x4CD0DC, 0xE0FAFF]),
    ("lagoon", [0x04181F, 0x0C3F44, 0x1F8C88, 0x8AD8CE, 0x48D8C8, 0xD8FFF6]),
    ("frost", [0x0C111A, 0x1E3352, 0x527199, 0xB4C8E0, 0x8BAED4, 0xEEF5FF]),
    ("rosewood", [0x140812, 0x3A0C2A, 0x8E2260, 0xE06A9A, 0xFFB070, 0xFFF0F0]),
    ("bronze", [0x0E0A06, 0x3A2410, 0x8A5A22, 0xD8A860, 0xFF8A50, 0xFFF2DC]),
    ("forest", [0x040E0A, 0x0E3224, 0x1E6E4A, 0x86C8A0, 0xE8B45A, 0xF0FFF2]),
];

pub fn is_backdrop(color: [f64; 3]) -> bool {
    BACKDROPS.iter().any(|(_, colors)| hex(0.0, colors[0]).color == color)
}

pub fn backdrop(name: &str) -> Option<[f64; 3]> {
    let (_, colors) = BACKDROPS.iter().find(|(each, _)| *each == name)?;
    Some(hex(0.0, colors[0]).color)
}

pub fn named(name: &str) -> Option<Palette> {
    if let Some((_, colors)) = BACKDROPS.iter().find(|(each, _)| *each == name) {
        return Some(Palette {
            stops: AT.iter().zip(colors).map(|(pos, color)| hex(*pos, *color)).collect(),
        });
    }
    let colors: &[u32] = match name {
        "aurora" => &[0x2B3A8C, 0x3F6FD8, 0x35C9C2, 0x7CF2E0, 0x3FD67A, 0x1E8F5A],
        "emerald" => &[0x3A3F8F, 0x2F8F6B, 0x55E05A, 0xB6FF6A, 0x4FD0A0, 0xD070E0],
        "ember" => &[0x7A1E0E, 0xD9480F, 0xFF8A1F, 0xFFD166, 0xFFF3C4, 0xC2410C],
        "violet" => &[0x2A1B6B, 0x5B3FD0, 0x9B6BFF, 0xE0A8FF, 0xFF7ACB, 0x5A2EA6],
        "ice" => &[0x1B3B6F, 0x2F7FCB, 0x7FD4FF, 0xE8FBFF, 0x9AD0F5, 0x3A62B8],
        "sunset" => &[0x40208A, 0xB0308A, 0xFF5E62, 0xFF9966, 0xFFD27F, 0x7A3FB0],
        "orchid" => &[0x1F6F78, 0x3CC6B0, 0xB8F2C8, 0xF2A6D8, 0xC850C0, 0x4158D0],
        "gold" => &[0x5A3A0A, 0xB8860B, 0xFFD700, 0xFFF4B0, 0xE0A030, 0x8A5A12],
        _ => return None,
    };
    Some(build(colors))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lut_hits_the_stops() {
        let palette = Palette {
            stops: vec![
                Stop {
                    pos: 0.0,
                    color: [0.0, 0.0, 0.0],
                },
                Stop {
                    pos: 1.0,
                    color: [1.0, 0.5, 0.0],
                },
            ],
        };
        let lut = lut(&palette);
        assert_eq!(lut[0], [0, 0, 0]);
        assert_eq!(lut[255], [255, 128, 0]);
        assert!(lut[128][0] > 120 && lut[128][0] < 136);
    }

    #[test]
    fn named_palettes_are_valid() {
        for name in NAMES.into_iter().chain(STUDIO) {
            let palette = named(name).expect(name);
            assert!(validate(&palette).is_ok(), "{name}");
        }
    }
}
