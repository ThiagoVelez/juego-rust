//! Campos de ruido del clima: continentalidad, erosión, temperatura, humedad y rareza.

use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

/// Mezcla de semillas (splitmix64): una semilla distinta por campo.
pub fn mix(seed: u64, salt: u64) -> u64 {
    let mut z = seed.wrapping_add(salt.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn fbm(seed: u64, salt: u64, octaves: usize, scale: f64) -> Fbm<Perlin> {
    Fbm::<Perlin>::new(mix(seed, salt) as u32)
        .set_octaves(octaves)
        .set_frequency(1.0 / scale)
}

#[derive(Clone, Copy, Debug)]
pub struct ClimateSample {
    pub continental: f64,
    pub erosion: f64,
    pub temperature: f64,
    pub humidity: f64,
    pub weird: f64,
    pub detail: f64,
}

pub struct Climate {
    continental: Fbm<Perlin>,
    erosion: Fbm<Perlin>,
    temperature: Fbm<Perlin>,
    humidity: Fbm<Perlin>,
    weird: Fbm<Perlin>,
    detail: Fbm<Perlin>,
}

impl Climate {
    pub fn new(seed: u64) -> Self {
        Climate {
            continental: fbm(seed, 1, 5, 2048.0),
            erosion: fbm(seed, 2, 4, 1024.0),
            temperature: fbm(seed, 3, 3, 1536.0),
            humidity: fbm(seed, 4, 3, 1536.0),
            weird: fbm(seed, 5, 3, 512.0),
            detail: fbm(seed, 6, 3, 64.0),
        }
    }

    pub fn sample(&self, x: i32, z: i32) -> ClimateSample {
        let p = [x as f64, z as f64];
        let w = |v: f64| (v * 1.6).clamp(-1.0, 1.0);
        ClimateSample {
            continental: w(self.continental.get(p)),
            erosion: w(self.erosion.get(p)),
            temperature: w(self.temperature.get(p)),
            humidity: w(self.humidity.get(p)),
            weird: w(self.weird.get(p)),
            detail: self.detail.get(p),
        }
    }
}
