//! Hora del mundo: sol, luz del cielo y colores del cielo.

use glam::Vec3;

pub const DAY_TICKS: u64 = 24_000;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct WorldTime {
    pub ticks: u64,
}

/// Color sRGB (0..1) a partir de bytes.
pub const fn srgb(r: u8, g: u8, b: u8) -> [f32; 3] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}

const DAY_ZENITH: [f32; 3] = srgb(0x5e, 0xa9, 0xe6);
const DAY_HORIZON: [f32; 3] = srgb(0xd7, 0xed, 0xf6);
const DUSK_ZENITH: [f32; 3] = srgb(0x6a, 0x7f, 0xc0);
const DUSK_HORIZON: [f32; 3] = srgb(0xff, 0xb0, 0x6a);
const NIGHT_ZENITH: [f32; 3] = srgb(0x0b, 0x12, 0x30);
const NIGHT_HORIZON: [f32; 3] = srgb(0x1d, 0x2a, 0x4a);

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

impl WorldTime {
    /// 0 = amanecer, 0,25 = mediodía, 0,5 = atardecer, 0,75 = medianoche.
    pub fn day_fraction(&self) -> f32 {
        (self.ticks % DAY_TICKS) as f32 / DAY_TICKS as f32
    }

    /// Altura del sol: 1 a mediodía, −1 a medianoche.
    fn elevation(&self) -> f32 {
        (self.day_fraction() * std::f32::consts::TAU).sin()
    }

    /// Dirección hacia el sol (sale por el este, +x; un poco inclinado al sur).
    pub fn sun_dir(&self) -> Vec3 {
        let a = self.day_fraction() * std::f32::consts::TAU;
        Vec3::new(a.cos(), a.sin(), 0.18).normalize()
    }

    /// Intensidad de la luz del cielo: 1 de día, 0,15 de noche.
    pub fn sun_light(&self) -> f32 {
        let s = smoothstep(-0.12, 0.2, self.elevation());
        0.15 + 0.85 * s
    }

    /// (cénit, horizonte) en sRGB 0..1.
    pub fn sky_colors(&self) -> ([f32; 3], [f32; 3]) {
        let e = self.elevation();
        let day = smoothstep(-0.15, 0.25, e);
        let mut zen = mix(NIGHT_ZENITH, DAY_ZENITH, day);
        let mut hor = mix(NIGHT_HORIZON, DAY_HORIZON, day);
        // Tonos de amanecer y atardecer cuando el sol está cerca del horizonte.
        let dusk = (1.0 - (e.abs() / 0.3)).clamp(0.0, 1.0);
        zen = mix(zen, DUSK_ZENITH, dusk * 0.55);
        hor = mix(hor, DUSK_HORIZON, dusk);
        (zen, hor)
    }

    /// Estrellas visibles (0..1).
    pub fn night(&self) -> f32 {
        1.0 - smoothstep(-0.25, 0.05, self.elevation())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_has_24000_ticks() {
        assert_eq!(DAY_TICKS, 24_000);
        assert_eq!(
            WorldTime { ticks: 0 }.day_fraction(),
            WorldTime { ticks: DAY_TICKS }.day_fraction()
        );
        assert!((WorldTime { ticks: 6000 }.day_fraction() - 0.25).abs() < 1e-6);
    }

    #[test]
    fn noon_is_brightest_midnight_darkest() {
        let l = |t| WorldTime { ticks: t }.sun_light();
        assert!((l(6000) - 1.0).abs() < 1e-4);
        assert!((l(18000) - 0.15).abs() < 1e-4);
        for t in (0..DAY_TICKS).step_by(100) {
            assert!(l(t) <= l(6000) + 1e-6 && l(t) >= l(18000) - 1e-6);
        }
        assert!(WorldTime { ticks: 6000 }.sun_dir().y > 0.9);
        assert!(WorldTime { ticks: 18000 }.sun_dir().y < -0.9);
    }

    #[test]
    fn sun_light_continuous() {
        for t in 0..DAY_TICKS {
            let a = WorldTime { ticks: t }.sun_light();
            let b = WorldTime { ticks: t + 1 }.sun_light();
            assert!((a - b).abs() < 0.01, "salto en {t}");
        }
    }

    #[test]
    fn sky_color_at_noon_matches_palette() {
        let (zen, hor) = WorldTime { ticks: 6000 }.sky_colors();
        let near = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.01);
        assert!(near(zen, srgb(0x5e, 0xa9, 0xe6)), "{zen:?}");
        assert!(near(hor, srgb(0xd7, 0xed, 0xf6)), "{hor:?}");
        let (nz, _) = WorldTime { ticks: 18000 }.sky_colors();
        assert!(near(nz, srgb(0x0b, 0x12, 0x30)), "{nz:?}");
    }
}
