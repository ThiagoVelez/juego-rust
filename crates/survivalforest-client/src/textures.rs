//! Texturas de 16×16 pintadas por código con la paleta propia del juego
//! (tierra roja, verdes cálidos, borde tallado). Si existe
//! `assets/textures/<nombre>.png` de 16×16, se usa en su lugar.

use survivalforest_core::block::TILE_NAMES;

pub const T: usize = 16;
pub type Tile = Vec<u8>;

struct Rng(u64);
impl Rng {
    fn f(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f32) / (1u64 << 31) as f32
    }
}

fn hex(h: &str) -> [f32; 3] {
    let v = u32::from_str_radix(h.trim_start_matches('#'), 16).unwrap_or(0);
    [
        ((v >> 16) & 255) as f32,
        ((v >> 8) & 255) as f32,
        (v & 255) as f32,
    ]
}

/// Pinta todas las capas en el orden de `TILE_NAMES`.
pub fn paint_tiles() -> Vec<Tile> {
    TILE_NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| paint(n, i as u64))
        .collect()
}

fn paint(name: &str, salt: u64) -> Tile {
    let mut rng = Rng(0x5EED ^ salt.wrapping_mul(0x9E37_79B9));
    let mut out = vec![0u8; T * T * 4];
    let edge: Vec<usize> = (0..T).map(|_| 3 + (rng.f() * 3.0) as usize).collect();
    let cherries: Vec<(usize, usize)> = (0..9)
        .map(|_| (1 + (rng.f() * 13.0) as usize, 1 + (rng.f() * 13.0) as usize))
        .collect();
    let bevel_on = !matches!(
        name,
        "water" | "leaves" | "coffee_bush" | "torch" | "frailejon" | "ice"
    );
    for y in 0..T {
        for x in 0..T {
            let (r1, r2) = (rng.f(), rng.f());
            let v = |c: [f32; 3], a: f32| {
                let d = (r1 - 0.5) * a;
                [c[0] + d, c[1] + d, c[2] + d * 0.8, 255.0]
            };
            let (fx, fy) = (x as f32, y as f32);
            let c: [f32; 4] = match name {
                "grass_top" => {
                    let w = (fx * 0.9 + fy * 0.4).sin() + (fy * 0.8 - fx * 0.3).cos();
                    if w > 1.1 {
                        v(hex("#93cf5c"), 10.0)
                    } else if w < -1.2 {
                        v(hex("#5aa33f"), 8.0)
                    } else {
                        v(hex("#6fb84a"), 9.0)
                    }
                }
                "grass_side" | "snow_side" => {
                    let top = if name == "grass_side" {
                        hex("#6fb84a")
                    } else {
                        hex("#f4f6f8")
                    };
                    if y < edge[x] {
                        v(top, 10.0)
                    } else if y == edge[x] {
                        v(
                            if name == "grass_side" {
                                hex("#4f8f36")
                            } else {
                                hex("#cfd8e0")
                            },
                            8.0,
                        )
                    } else if (y + x * 3) % 7 == 0 {
                        v(hex("#93402a"), 8.0)
                    } else {
                        v(hex("#b5532f"), 10.0)
                    }
                }
                "red_soil" => {
                    if (y * 5 + x * 3) % 9 == 0 {
                        v(hex("#93402a"), 8.0)
                    } else {
                        v(hex("#b5532f"), 10.0)
                    }
                }
                "stone" => {
                    if (x * 7 + y * 3) % 11 == 0 || r2 < 0.08 {
                        v(hex("#7d7569"), 10.0)
                    } else {
                        v(hex("#9a9389"), 16.0)
                    }
                }
                "bedrock_floor" => {
                    if r2 < 0.4 {
                        v(hex("#2e2a27"), 10.0)
                    } else {
                        v(hex("#4a4440"), 14.0)
                    }
                }
                "sand" => v(hex("#e8d49b"), 14.0),
                "red_sand" => v(hex("#d9824f"), 14.0),
                "gravel" => {
                    if r2 < 0.35 {
                        v(hex("#6f6a63"), 12.0)
                    } else {
                        v(hex("#a39d94"), 18.0)
                    }
                }
                "clay" => v(hex("#a7b0bc"), 10.0),
                "mud" => {
                    if r2 < 0.2 {
                        v(hex("#3f2e22"), 8.0)
                    } else {
                        v(hex("#5a4232"), 10.0)
                    }
                }
                "snow" => v(hex("#f4f6f8"), 6.0),
                "ice" => {
                    let mut p = v(hex("#a9d6f2"), 10.0);
                    if (x + y) % 9 == 0 {
                        p = [235.0, 248.0, 255.0, 255.0];
                    }
                    p[3] = 230.0;
                    p
                }
                "water" => {
                    let mut p = if (y + x / 4) % 6 == 0 && r2 < 0.7 {
                        v([120.0, 185.0, 235.0], 10.0)
                    } else {
                        v(hex("#3d8fd4"), 10.0)
                    };
                    p[3] = 255.0;
                    p
                }
                "log_side" => {
                    if x % 4 == 0 || r2 < 0.07 {
                        v(hex("#4f311d"), 10.0)
                    } else {
                        v(hex("#6b4428"), 16.0)
                    }
                }
                "log_top" => {
                    let r = ((fx - 7.5).powi(2) + (fy - 7.5).powi(2)).sqrt();
                    if r > 6.8 {
                        v(hex("#4f311d"), 10.0)
                    } else if (r as i32) % 2 == 1 {
                        v(hex("#c89a62"), 10.0)
                    } else {
                        v(hex("#a97b48"), 10.0)
                    }
                }
                "leaves" => {
                    let k = (fx * 1.3).sin() * (fy * 1.1).cos();
                    if r2 < 0.07 {
                        [0.0; 4]
                    } else if k > 0.45 {
                        v(hex("#58ad4f"), 8.0)
                    } else if k < -0.45 {
                        v(hex("#2f7a3a"), 8.0)
                    } else {
                        v(hex("#3c8f45"), 8.0)
                    }
                }
                "coffee_bush" => {
                    if cherries
                        .iter()
                        .any(|&(cx, cy)| x >= cx && x <= cx + 1 && y >= cy && y <= cy + 1)
                    {
                        v(hex("#d23a3f"), 24.0)
                    } else if r2 < 0.1 {
                        [0.0; 4]
                    } else {
                        v(hex("#2c6e3a"), 20.0)
                    }
                }
                "bamboo" => {
                    if y == 4 || y == 12 {
                        v(hex("#6f7f28"), 8.0)
                    } else if x == 3 || x == 11 {
                        v(hex("#8fa83b"), 6.0)
                    } else {
                        v(hex("#a9c24a"), 8.0)
                    }
                }
                "frailejon" => {
                    if (x + y) % 3 == 0 {
                        v(hex("#c9cfa8"), 10.0)
                    } else {
                        v(hex("#8f9a6a"), 12.0)
                    }
                }
                "planks" => {
                    if y % 4 == 3 || (if y % 8 < 4 { x == 5 } else { x == 12 }) {
                        v(hex("#93602f"), 8.0)
                    } else {
                        v(hex("#b67d46"), 14.0)
                    }
                }
                "whitewash" => {
                    if y > 11 {
                        v(hex("#2f6fb3"), 8.0)
                    } else if y == 11 {
                        [30.0, 70.0, 120.0, 255.0]
                    } else {
                        v(hex("#f3eee4"), 8.0)
                    }
                }
                "roof_tile" => {
                    if y % 4 == 3 || (x + (y / 4 % 2) * 3) % 6 == 0 {
                        v(hex("#9c3f28"), 10.0)
                    } else {
                        v(hex("#c4583a"), 16.0)
                    }
                }
                "coal_ore" | "iron_ore" | "copper_ore" | "gold_ore" | "quartz_ore" => {
                    let spot = ((fx - 4.0).powi(2) + (fy - 5.0).powi(2) < 4.0)
                        || ((fx - 11.0).powi(2) + (fy - 10.0).powi(2) < 5.0)
                        || ((fx - 10.0).powi(2) + (fy - 3.0).powi(2) < 2.0);
                    if spot {
                        let col = match name {
                            "coal_ore" => "#2b2b2b",
                            "iron_ore" => "#d8b39a",
                            "copper_ore" => "#d97a3a",
                            "gold_ore" => "#f2cf3a",
                            _ => "#f2eef7",
                        };
                        v(hex(col), 14.0)
                    } else {
                        v(hex("#9a9389"), 16.0)
                    }
                }
                "torch" => {
                    if (6..10).contains(&x) && y < 4 {
                        v(hex("#ffd36b"), 20.0)
                    } else if (7..9).contains(&x) {
                        v(hex("#7a4e2b"), 10.0)
                    } else {
                        [0.0; 4]
                    }
                }
                _ => v([255.0, 0.0, 255.0], 0.0),
            };
            let mut c = c;
            if bevel_on && c[3] > 0.0 {
                let e = x.min(y).min(T - 1 - x).min(T - 1 - y);
                let k = if e == 0 {
                    0.8
                } else if e == 1 {
                    0.93
                } else {
                    1.0
                };
                let hi = if (x == 1 || y == 1) && e == 1 {
                    1.07
                } else {
                    1.0
                };
                for ch in c.iter_mut().take(3) {
                    *ch *= k * hi;
                }
            }
            let o = (y * T + x) * 4;
            for k in 0..4 {
                out[o + k] = c[k].clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_tile_per_texture_name() {
        let t = paint_tiles();
        assert_eq!(t.len(), TILE_NAMES.len());
        assert!(t.iter().all(|x| x.len() == T * T * 4));
    }

    #[test]
    fn leaves_have_holes_and_stone_is_opaque() {
        let t = paint_tiles();
        let idx = |n: &str| TILE_NAMES.iter().position(|x| *x == n).unwrap();
        let alpha = |tile: &Tile| tile.chunks(4).filter(|p| p[3] < 128).count();
        assert!(alpha(&t[idx("leaves")]) > 0);
        assert_eq!(alpha(&t[idx("stone")]), 0);
    }

    #[test]
    fn red_soil_is_reddish() {
        let t = paint_tiles();
        let s = &t[TILE_NAMES.iter().position(|x| *x == "red_soil").unwrap()];
        let (mut r, mut g) = (0u32, 0u32);
        for p in s.chunks(4) {
            r += p[0] as u32;
            g += p[1] as u32;
        }
        assert!(r > g + 20 * 256, "r={r} g={g}");
    }
}
