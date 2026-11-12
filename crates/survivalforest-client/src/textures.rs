//! Texturas de 32×32 pintadas por código con la paleta propia del juego
//! (tierra roja, verdes cálidos, borde tallado). Si existe
//! `assets/textures/<nombre>.png` de 32×32 (o de 16×16, que se amplía), se usa en su lugar.

use survivalforest_core::block::TILE_NAMES;

pub const T: usize = 32;
pub type Tile = Vec<u8>;

type C = [f32; 4];

fn hex(h: &str) -> [f32; 3] {
    let v = u32::from_str_radix(h.trim_start_matches('#'), 16).unwrap_or(0);
    [
        ((v >> 16) & 255) as f32,
        ((v >> 8) & 255) as f32,
        (v & 255) as f32,
    ]
}

fn rgba(c: [f32; 3]) -> C {
    [c[0], c[1], c[2], 255.0]
}

fn shade(c: [f32; 3], k: f32) -> [f32; 3] {
    [c[0] * k, c[1] * k, c[2] * k]
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Hash entero → 0..1 (determinista).
fn h(x: i32, y: i32, s: u32) -> f32 {
    let mut n = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ s.wrapping_mul(2_246_822_519);
    n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
    ((n ^ (n >> 16)) & 0xFFFF) as f32 / 65535.0
}

/// Ruido de valor que se repite cada `T` píxeles (las texturas encajan entre bloques).
fn vn(x: f32, y: f32, cell: f32, s: u32) -> f32 {
    let period = (T as f32 / cell).round() as i32;
    let (fx, fy) = (x / cell, y / cell);
    let (ix, iy) = (fx.floor() as i32, fy.floor() as i32);
    let (tx, ty) = (fx - ix as f32, fy - iy as f32);
    let (ux, uy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let g = |a: i32, b: i32| h(a.rem_euclid(period), b.rem_euclid(period), s);
    let top = g(ix, iy) + (g(ix + 1, iy) - g(ix, iy)) * ux;
    let bot = g(ix, iy + 1) + (g(ix + 1, iy + 1) - g(ix, iy + 1)) * ux;
    top + (bot - top) * uy
}

/// Ruido de dos octavas (0..1).
fn fbm(x: f32, y: f32, s: u32) -> f32 {
    vn(x, y, 8.0, s) * 0.65 + vn(x, y, 4.0, s + 7) * 0.35
}

/// Pinta todas las capas en el orden de `TILE_NAMES`.
pub fn paint_tiles() -> Vec<Tile> {
    TILE_NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| paint(n, i as u32 * 31 + 5))
        .collect()
}

/// Distancia al centro de la celda más cercana de un patrón de puntos (Voronoi simple).
fn cells(x: f32, y: f32, size: f32, s: u32) -> (f32, f32, (i32, i32)) {
    let period = (T as f32 / size).round() as i32;
    let (cx, cy) = ((x / size).floor() as i32, (y / size).floor() as i32);
    let (mut d1, mut d2, mut id) = (9.0f32, 9.0f32, (0, 0));
    for oy in -1..=1 {
        for ox in -1..=1 {
            let (gx, gy) = (cx + ox, cy + oy);
            let (wx, wy) = (gx.rem_euclid(period), gy.rem_euclid(period));
            let px = (gx as f32 + 0.15 + h(wx, wy, s) * 0.7) * size;
            let py = (gy as f32 + 0.15 + h(wx, wy, s + 1) * 0.7) * size;
            let d = ((x - px).powi(2) + (y - py).powi(2)).sqrt() / size;
            if d < d1 {
                d2 = d1;
                d1 = d;
                id = (wx, wy);
            } else if d < d2 {
                d2 = d;
            }
        }
    }
    (d1, d2, id)
}

fn soil(x: f32, y: f32, s: u32) -> [f32; 3] {
    let n = fbm(x, y, s);
    let mut c = mix3(hex("#9c4429"), hex("#c0603a"), n);
    let (d1, _, id) = cells(x, y, 6.0, s + 3);
    if d1 < 0.22 && h(id.0, id.1, s + 9) > 0.55 {
        c = mix3(hex("#7e3524"), hex("#a8968a"), h(id.0, id.1, s + 4) * 0.5);
    }
    c
}

fn stone_px(x: f32, y: f32, s: u32) -> [f32; 3] {
    let (d1, d2, id) = cells(x, y, 9.0, s);
    let base = mix3(hex("#8a8378"), hex("#aaa398"), h(id.0, id.1, s + 2));
    let mut c = shade(base, 0.92 + fbm(x, y, s + 5) * 0.16);
    let edge = d2 - d1;
    if edge < 0.07 {
        c = shade(c, 0.66);
    } else if edge < 0.13 {
        c = shade(c, 0.86);
    }
    c
}

fn paint(name: &str, s: u32) -> Tile {
    let mut out = vec![0u8; T * T * 4];
    let bevel_on = !matches!(
        name,
        "water" | "leaves" | "coffee_bush" | "torch" | "frailejon" | "ice" | "bamboo"
    );
    for y in 0..T {
        for x in 0..T {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let n = fbm(fx, fy, s);
            let r = h(x as i32, y as i32, s + 99);
            let c: C = match name {
                "grass_top" => {
                    let mut c = mix3(hex("#5ea53f"), hex("#82c653"), n);
                    // Briznas: trazos cortos más claros y más oscuros.
                    let blade = h(x as i32, y as i32 / 3, s + 1);
                    if blade > 0.86 {
                        c = shade(c, 1.15);
                    } else if blade < 0.1 {
                        c = shade(c, 0.82);
                    }
                    if r > 0.985 {
                        c = hex("#f2d860");
                    }
                    rgba(c)
                }
                "grass_side" | "snow_side" => {
                    let cap = 6.0
                        + vn(fx, 0.0, 4.0, s) * 5.0
                        + if h(x as i32, 0, s + 3) > 0.8 {
                            3.0
                        } else {
                            0.0
                        };
                    if fy < cap {
                        let c = if name == "grass_side" {
                            mix3(hex("#5ea53f"), hex("#82c653"), n)
                        } else {
                            mix3(hex("#e4ebf0"), hex("#ffffff"), n)
                        };
                        rgba(if fy > cap - 1.5 { shade(c, 0.8) } else { c })
                    } else {
                        rgba(soil(fx, fy, s))
                    }
                }
                "red_soil" => rgba(soil(fx, fy, s)),
                "stone" => rgba(stone_px(fx, fy, s)),
                "bedrock_floor" => {
                    let (d1, d2, _) = cells(fx, fy, 7.0, s);
                    rgba(shade(
                        mix3(hex("#2b2724"), hex("#545049"), n),
                        if d2 - d1 < 0.1 { 0.6 } else { 1.0 },
                    ))
                }
                "sand" | "red_sand" => {
                    let (a, b) = if name == "sand" {
                        (hex("#dcc58a"), hex("#f0dcaa"))
                    } else {
                        (hex("#c56f3e"), hex("#e39462"))
                    };
                    let ripple = ((fy * 0.55 + vn(fx, fy, 8.0, s) * 4.0).sin() * 0.5 + 0.5) * 0.35;
                    let mut c = mix3(a, b, n * 0.65 + ripple);
                    if r > 0.93 {
                        c = shade(c, 0.85);
                    }
                    rgba(c)
                }
                "gravel" => {
                    let (d1, _, id) = cells(fx, fy, 6.0, s);
                    let tone = mix3(hex("#6c665e"), hex("#b9b2a6"), h(id.0, id.1, s + 1));
                    rgba(if d1 > 0.45 {
                        hex("#4e4943")
                    } else {
                        shade(tone, 1.08 - d1 * 0.5)
                    })
                }
                "clay" => {
                    let band = ((fy / 5.0).floor() as i32 % 2) as f32 * 0.06;
                    rgba(shade(mix3(hex("#97a1ae"), hex("#b5bfcc"), n), 0.97 + band))
                }
                "mud" => {
                    let mut c = mix3(hex("#3b2b20"), hex("#5e4636"), n);
                    if vn(fx, fy, 8.0, s + 4) > 0.72 {
                        c = mix3(c, hex("#6f6a5e"), 0.4);
                    }
                    rgba(c)
                }
                "snow" => {
                    let mut c = mix3(hex("#dfe8ef"), hex("#ffffff"), n);
                    if r > 0.97 {
                        c = hex("#cfe6ff");
                    }
                    rgba(c)
                }
                "ice" => {
                    let (d1, d2, _) = cells(fx, fy, 11.0, s);
                    let mut c = mix3(hex("#9fd0ef"), hex("#cdeafb"), n);
                    if d2 - d1 < 0.05 {
                        c = hex("#eef8ff");
                    }
                    [c[0], c[1], c[2], 225.0]
                }
                "water" => {
                    let wave = ((fx * 0.4 + fy * 0.9 + vn(fx, fy, 8.0, s) * 5.0).sin() * 0.5 + 0.5)
                        .powi(6);
                    let c = mix3(
                        mix3(hex("#2f7fc4"), hex("#4a9ee0"), n),
                        hex("#a8d8f5"),
                        wave * 0.7,
                    );
                    rgba(c)
                }
                "log_side" => {
                    let groove =
                        ((fx * 0.9 + vn(fx, fy, 8.0, s) * 3.0).sin() * 0.5 + 0.5).powf(3.0);
                    let mut c = mix3(hex("#5d3a21"), hex("#7b5130"), n);
                    c = shade(c, 1.05 - groove * 0.4);
                    if h(x as i32 / 2, y as i32 / 6, s + 5) > 0.93 {
                        c = shade(c, 0.7);
                    }
                    rgba(c)
                }
                "log_top" => {
                    let (dx, dy) = (fx - 16.0, fy - 16.0);
                    let rr = (dx * dx + dy * dy).sqrt() + vn(fx, fy, 8.0, s) * 1.6;
                    if rr > 14.0 {
                        rgba(shade(hex("#5d3a21"), 0.9 + n * 0.2))
                    } else {
                        let ring = ((rr * 1.1).sin() * 0.5 + 0.5) * 0.25;
                        let mut c = mix3(hex("#b98a55"), hex("#d4a872"), n * 0.6 + ring);
                        // Grieta radial
                        if dy.abs() < 0.8 && dx > 2.0 && dx < 10.0 {
                            c = shade(c, 0.7);
                        }
                        rgba(c)
                    }
                }
                "leaves" | "coffee_bush" => {
                    let (d1, _, id) = cells(fx, fy, 5.0, s);
                    let dark = name == "coffee_bush";
                    let (a, b) = if dark {
                        (hex("#215a2f"), hex("#3a8247"))
                    } else {
                        (hex("#2f7d3a"), hex("#5cb153"))
                    };
                    let leaf = mix3(a, b, h(id.0, id.1, s + 2));
                    let mut c: C = if d1 > 0.62 && h(id.0, id.1, s + 6) > 0.55 {
                        [0.0; 4]
                    } else {
                        rgba(shade(leaf, 1.12 - d1 * 0.45))
                    };
                    if dark {
                        // Granos de café: racimos redondos rojos y anaranjados.
                        for k in 0..7 {
                            let cx = 3.0 + h(k, 0, s + 11) * 26.0;
                            let cy = 3.0 + h(k, 1, s + 11) * 26.0;
                            for (ox, oy) in [(0.0, 0.0), (2.6, 1.2), (0.8, 2.6)] {
                                let d = ((fx - cx - ox).powi(2) + (fy - cy - oy).powi(2)).sqrt();
                                if d < 1.6 {
                                    let ripe = if h(k, 2, s) > 0.3 {
                                        hex("#d1323b")
                                    } else {
                                        hex("#e8822f")
                                    };
                                    c = rgba(if d < 0.7 {
                                        mix3(ripe, hex("#ffffff"), 0.35)
                                    } else {
                                        ripe
                                    });
                                }
                            }
                        }
                    }
                    c
                }
                "bamboo" => {
                    let node = (y % 16) as i32;
                    let mut c = mix3(hex("#94ad3c"), hex("#b9cf5a"), n);
                    if x % 16 < 2 || x % 16 > 13 {
                        c = shade(c, 0.78);
                    } else if x % 16 == 5 || x % 16 == 6 {
                        c = shade(c, 1.12);
                    }
                    if node == 0 || node == 1 {
                        c = hex("#6c7f26");
                    }
                    rgba(c)
                }
                "frailejon" => {
                    let (d1, _, id) = cells(fx, fy, 4.0, s);
                    let fuzz = mix3(hex("#8f9a6a"), hex("#d3d8b4"), h(id.0, id.1, s + 1));
                    rgba(shade(fuzz, 1.1 - d1 * 0.4))
                }
                "planks" => {
                    let board = y / 8;
                    let off = h(board as i32, 0, s) * 32.0;
                    let grain = ((fx + off) * 0.35 + vn(fx + off, fy, 4.0, s + board as u32) * 4.0)
                        .sin()
                        * 0.5
                        + 0.5;
                    let mut c = mix3(hex("#a26d3a"), hex("#c48a51"), grain * 0.6 + n * 0.4);
                    c = shade(c, 0.92 + h(board as i32, 1, s) * 0.14);
                    let seam = ((x as f32 + off) as usize) % 32;
                    if y % 8 == 7 || (y % 8 > 0 && seam == 0) {
                        c = hex("#6e4524");
                    }
                    if y % 8 == 3 && (seam == 3 || seam == 29) {
                        c = hex("#4a3a30");
                    }
                    rgba(c)
                }
                "whitewash" => {
                    if y >= 24 {
                        rgba(mix3(hex("#24609f"), hex("#3a7cc0"), n))
                    } else if y == 23 {
                        rgba(hex("#1d4472"))
                    } else {
                        let mut c = mix3(hex("#ece5d8"), hex("#faf6ee"), n);
                        if vn(fx, fy, 8.0, s + 3) > 0.78 {
                            c = shade(c, 0.93);
                        }
                        rgba(c)
                    }
                }
                "roof_tile" => {
                    // Tejas curvas en filas desplazadas.
                    let row = y / 8;
                    let shift = if row % 2 == 1 { 4 } else { 0 };
                    let u = ((x + shift) % 8) as f32 / 8.0;
                    let v = (y % 8) as f32 / 8.0;
                    let curve = (u * std::f32::consts::PI).sin();
                    let mut c = mix3(hex("#a8442b"), hex("#d06a45"), curve * 0.7 + n * 0.3);
                    c = shade(c, 1.05 - v * 0.3);
                    if v > 0.86 {
                        c = hex("#6e2a1a");
                    }
                    rgba(c)
                }
                "coal_ore" | "iron_ore" | "copper_ore" | "gold_ore" | "quartz_ore" => {
                    let (d1, _, id) = cells(fx, fy, 7.0, s + 13);
                    let has = h(id.0, id.1, s + 17) > 0.45;
                    if has && d1 < 0.34 {
                        let (dark, light) = match name {
                            "coal_ore" => (hex("#1c1c1c"), hex("#3d3d3d")),
                            "iron_ore" => (hex("#b48c74"), hex("#e2c3ad")),
                            "copper_ore" => (hex("#b9602c"), hex("#4fb39a")),
                            "gold_ore" => (hex("#d9a91f"), hex("#fff09a")),
                            _ => (hex("#d9d2e6"), hex("#ffffff")),
                        };
                        rgba(mix3(
                            dark,
                            light,
                            (0.34 - d1) / 0.34 * 0.9 + if r > 0.9 { 0.3 } else { 0.0 },
                        ))
                    } else {
                        rgba(stone_px(fx, fy, s))
                    }
                }
                "torch" => {
                    let cx = (fx - 16.0).abs();
                    let flame_w = 0.6
                        + 3.6
                            * (std::f32::consts::PI * (fy + 0.5) / 12.0).sin().max(0.0)
                            * (fy / 12.0).sqrt();
                    if fy < 11.0 && cx < flame_w {
                        let t = fy / 11.0;
                        rgba(mix3(
                            hex("#fff4b0"),
                            hex("#ff8a2a"),
                            (t + cx / 6.0).min(1.0),
                        ))
                    } else if cx < 2.0 && fy >= 10.0 {
                        rgba(shade(hex("#7a4e2b"), 0.85 + n * 0.3))
                    } else {
                        [0.0; 4]
                    }
                }
                _ => [255.0, 0.0, 255.0, 255.0],
            };
            let mut c = c;
            if bevel_on && c[3] > 0.0 {
                // Borde tallado de 2 píxeles, con brillo arriba a la izquierda.
                let e = x.min(y).min(T - 1 - x).min(T - 1 - y);
                let k = match e {
                    0 => 0.78,
                    1 => 0.9,
                    _ => 1.0,
                };
                let hi = if (x == 2 || y == 2) && e == 2 {
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

/// Ajusta una textura de un TexturePack a `T`×`T`: 32 tal cual; 16 se amplía ×2 sin suavizar.
pub fn fit_tile(rgba: &[u8], size: usize) -> Option<Tile> {
    if size == T && rgba.len() == T * T * 4 {
        return Some(rgba.to_vec());
    }
    if size * 2 != T || rgba.len() != size * size * 4 {
        return None;
    }
    let mut out = vec![0u8; T * T * 4];
    for y in 0..T {
        for x in 0..T {
            let src = ((y / 2) * size + x / 2) * 4;
            out[(y * T + x) * 4..(y * T + x) * 4 + 4].copy_from_slice(&rgba[src..src + 4]);
        }
    }
    Some(out)
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
    fn tiles_are_32x32() {
        assert_eq!(T, 32);
        assert!(paint_tiles().iter().all(|t| t.len() == 32 * 32 * 4));
    }

    #[test]
    fn tiles_have_detail() {
        // Con 32×32 cada textura debe tener bastantes tonos distintos (no un color liso ampliado).
        for (i, t) in paint_tiles().iter().enumerate() {
            let mut c: Vec<[u8; 3]> = t
                .chunks(4)
                .filter(|p| p[3] > 0)
                .map(|p| [p[0] / 8, p[1] / 8, p[2] / 8])
                .collect();
            c.sort();
            c.dedup();
            assert!(
                c.len() >= 6,
                "{} solo tiene {} tonos",
                TILE_NAMES[i],
                c.len()
            );
        }
    }

    #[test]
    fn pack_16_upscales_to_32() {
        let mut small = vec![0u8; 16 * 16 * 4];
        small[0..4].copy_from_slice(&[255, 0, 0, 255]);
        let big = fit_tile(&small, 16).expect("16 sirve");
        assert_eq!(big.len(), 32 * 32 * 4);
        assert_eq!(&big[0..4], &[255, 0, 0, 255]);
        assert_eq!(&big[4..8], &[255, 0, 0, 255], "ampliado sin suavizar");
        assert_eq!(&big[32 * 4..32 * 4 + 4], &[255, 0, 0, 255]);
        assert!(fit_tile(&vec![0u8; 20 * 20 * 4], 20).is_none());
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
        assert!(r > g + 20 * (T * T) as u32, "r={r} g={g}");
    }

    /// `SF_ATLAS=/ruta.rgba cargo test dump_atlas -- --ignored`: vuelca todas las texturas en vertical.
    #[test]
    #[ignore]
    fn dump_atlas() {
        let path = std::env::var("SF_ATLAS").expect("SF_ATLAS");
        std::fs::write(path, paint_tiles().concat()).unwrap();
    }
}
