//! Iconos de 32×32 de los objetos que no son bloques (herramientas, materiales y comida),
//! pintados por código con la paleta del juego.

use crate::textures::{T, Tile, fbm, hex, mix3, shade};
use survivalforest_core::item::{ToolClass, item, item_by_key};

/// Lienzo de 32×32 con colores en coma flotante.
struct Canvas(Vec<[f32; 4]>);

impl Canvas {
    fn new() -> Self {
        Self(vec![[0.0; 4]; T * T])
    }
    fn set(&mut self, x: i32, y: i32, c: [f32; 3]) {
        if (0..T as i32).contains(&x) && (0..T as i32).contains(&y) {
            self.0[y as usize * T + x as usize] = [c[0], c[1], c[2], 255.0];
        }
    }
    /// Rellena los píxeles donde `inside` es cierto, con un poco de textura.
    fn fill(&mut self, a: [f32; 3], b: [f32; 3], seed: u32, inside: impl Fn(f32, f32) -> bool) {
        for y in 0..T as i32 {
            for x in 0..T as i32 {
                let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                if inside(fx, fy) {
                    self.set(x, y, mix3(a, b, fbm(fx, fy, seed)));
                }
            }
        }
    }
    /// Línea gruesa de (x0, y0) a (x1, y1).
    fn line(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        w: f32,
        a: [f32; 3],
        b: [f32; 3],
        seed: u32,
    ) {
        self.fill(a, b, seed, |x, y| {
            let (dx, dy) = (x1 - x0, y1 - y0);
            let t = (((x - x0) * dx + (y - y0) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            ((x - x0 - t * dx).powi(2) + (y - y0 - t * dy).powi(2)).sqrt() <= w / 2.0
        });
    }
    /// Contorno oscuro de un píxel alrededor de lo pintado (como las texturas talladas).
    fn outline(&mut self) {
        let src = self.0.clone();
        for y in 0..T as i32 {
            for x in 0..T as i32 {
                if src[y as usize * T + x as usize][3] > 0.0 {
                    continue;
                }
                let near = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                    let (nx, ny) = (x + dx, y + dy);
                    (0..T as i32).contains(&nx)
                        && (0..T as i32).contains(&ny)
                        && src[ny as usize * T + nx as usize][3] > 0.0
                });
                if near {
                    self.0[y as usize * T + x as usize] = [40.0, 28.0, 22.0, 255.0];
                }
            }
        }
    }
    fn tile(mut self) -> Tile {
        self.outline();
        self.0
            .iter()
            .flat_map(|c| c.map(|v| v.clamp(0.0, 255.0) as u8))
            .collect()
    }
}

/// Colores (oscuro, claro) de cada material de herramienta.
fn material(key: &str) -> ([f32; 3], [f32; 3]) {
    if key.ends_with("_wood") {
        (hex("#c8a868"), hex("#ecd49a"))
    } else if key.ends_with("_stone") {
        (hex("#6f6a62"), hex("#a39c90"))
    } else if key.ends_with("_copper") {
        (hex("#b84a1a"), hex("#f07a30"))
    } else {
        (hex("#a8a8b4"), hex("#eeeef4"))
    }
}

fn stick(c: &mut Canvas, x0: f32, y0: f32, x1: f32, y1: f32) {
    c.line(x0, y0, x1, y1, 3.0, hex("#6e4524"), hex("#9a6a3a"), 3);
}

fn tool_icon(key: &str, class: ToolClass) -> Tile {
    let mut c = Canvas::new();
    let (dark, light) = material(key);
    match class {
        ToolClass::Pickaxe => {
            stick(&mut c, 6.0, 27.0, 20.0, 12.0);
            c.fill(dark, light, 11, |x, y| {
                let d = ((x - 20.0).powi(2) + (y - 18.0).powi(2)).sqrt();
                (12.0..15.5).contains(&d) && y < 15.0 && x > 6.0
            });
        }
        ToolClass::Axe => {
            stick(&mut c, 7.0, 28.0, 21.0, 6.0);
            c.fill(dark, light, 12, |x, y| {
                x > 16.0 && x < 27.0 && y > 4.0 && y < 16.0 && (x - 16.0) > (y - 4.0) * 0.3
            });
        }
        ToolClass::Shovel => {
            stick(&mut c, 7.0, 27.0, 18.0, 14.0);
            c.fill(dark, light, 13, |x, y| {
                ((x - 21.0).powi(2) / 30.0 + (y - 10.0).powi(2) / 42.0) < 1.0
            });
        }
        ToolClass::Machete => {
            stick(&mut c, 5.0, 28.0, 10.0, 22.0);
            c.line(10.0, 22.0, 27.0, 5.0, 4.5, dark, light, 14);
            c.line(
                8.0,
                19.0,
                13.0,
                24.0,
                2.0,
                hex("#3a2a20"),
                hex("#5a3a22"),
                2,
            );
        }
    }
    c.tile()
}

fn lump(a: &str, b: &str, fleck: Option<&str>, seed: u32) -> Tile {
    let mut c = Canvas::new();
    c.fill(hex(a), hex(b), seed, |x, y| {
        ((x - 16.0) / 11.0).powi(2) + ((y - 17.0) / 9.0).powi(2) + fbm(x, y, seed) * 0.25 < 1.0
    });
    if let Some(f) = fleck {
        for (x, y) in [(12, 14), (19, 16), (15, 20), (21, 21), (11, 19)] {
            c.set(x, y, hex(f));
            c.set(x + 1, y, shade(hex(f), 1.15));
        }
    }
    c.tile()
}

fn ingot(a: &str, b: &str) -> Tile {
    let mut c = Canvas::new();
    c.fill(hex(a), hex(b), 21, |x, y| {
        (10.0..24.0).contains(&y) && x > 4.0 + (24.0 - y) * 0.3 && x < 28.0 - (24.0 - y) * 0.3
    });
    c.fill(shade(hex(b), 1.1), hex(b), 22, |x, y| {
        (10.0..13.0).contains(&y) && x > 9.0 && x < 23.0
    });
    c.tile()
}

fn platano(a: &str, b: &str) -> Tile {
    let mut c = Canvas::new();
    c.fill(hex(a), hex(b), 31, |x, y| {
        let d = ((x - 16.0).powi(2) + (y - 4.0).powi(2)).sqrt();
        (15.0..21.0).contains(&d) && y > 10.0 && x > 5.0 && x < 28.0
    });
    c.set(5, 13, hex("#4a3a20"));
    c.set(6, 14, hex("#4a3a20"));
    c.tile()
}

/// Icono de un objeto que no es bloque (None para los bloques y claves desconocidas).
pub fn item_icon(key: &str) -> Option<Tile> {
    let it = item(item_by_key(key)?);
    if it.places.is_some() {
        return None;
    }
    if let Some(t) = &it.tool {
        return Some(tool_icon(key, t.class));
    }
    let t = match key {
        "stick" => {
            let mut c = Canvas::new();
            stick(&mut c, 8.0, 26.0, 24.0, 6.0);
            c.tile()
        }
        "guadua" => {
            let mut c = Canvas::new();
            c.line(
                16.0,
                3.0,
                16.0,
                29.0,
                6.0,
                hex("#94ad3c"),
                hex("#b9cf5a"),
                4,
            );
            for y in [10, 19] {
                c.line(
                    13.0,
                    y as f32,
                    19.0,
                    y as f32,
                    1.5,
                    hex("#6c7f26"),
                    hex("#6c7f26"),
                    5,
                );
            }
            c.tile()
        }
        "coal" => lump("#1c1c1c", "#3d3d3d", Some("#6a6a6a"), 41),
        "charcoal" => lump("#2a1a12", "#4a3020", Some("#7a5a40"), 42),
        "copper_ore_item" => lump("#6f6a62", "#a39c90", Some("#d9804a"), 43),
        "iron_ore_item" => lump("#6f6a62", "#a39c90", Some("#e2c3ad"), 44),
        "gold_ore_item" => lump("#6f6a62", "#a39c90", Some("#ffd23a"), 45),
        "quartz" => {
            let mut c = Canvas::new();
            c.fill(hex("#d9d2e6"), hex("#ffffff"), 46, |x, y| {
                (x - 16.0).abs() * 1.6 + (y - 16.0).abs() < 13.0
            });
            c.tile()
        }
        "copper_ingot" => ingot("#a8552a", "#e6904f"),
        "iron_ingot" => ingot("#a8a8b4", "#eeeef4"),
        "gold_ingot" => ingot("#c9961a", "#fff09a"),
        "brick" => {
            let mut c = Canvas::new();
            c.fill(hex("#a8442b"), hex("#c45a3a"), 47, |x, y| {
                (6.0..26.0).contains(&x) && (11.0..21.0).contains(&y)
            });
            c.tile()
        }
        "platano" => platano("#d9b82a", "#f4e06a"),
        "platano_asado" => platano("#8a4a1e", "#c8783a"),
        "guayaba" => {
            let mut c = Canvas::new();
            c.fill(hex("#a8c040"), hex("#e0e27a"), 48, |x, y| {
                ((x - 16.0) / 9.0).powi(2) + ((y - 17.0) / 10.0).powi(2) < 1.0
            });
            c.fill(hex("#e8708a"), hex("#f4a0b0"), 49, |x, y| {
                ((x - 19.0) / 3.0).powi(2) + ((y - 15.0) / 3.0).powi(2) < 1.0
            });
            c.tile()
        }
        "mora" => {
            let mut c = Canvas::new();
            for (cx, cy) in [
                (13.0, 14.0),
                (19.0, 14.0),
                (16.0, 19.0),
                (12.0, 20.0),
                (20.0, 20.0),
                (16.0, 24.0),
                (16.0, 10.0),
            ] {
                c.fill(hex("#2a0f2e"), hex("#7a3a7e"), 50, |x, y| {
                    (x - cx).powi(2) + (y - cy).powi(2) < 9.0
                });
            }
            c.line(16.0, 3.0, 16.0, 8.0, 2.0, hex("#3c8f45"), hex("#6fb84a"), 6);
            c.tile()
        }
        _ => return None,
    };
    Some(t)
}

/// Iconos del HUD: corazones, jamones, burbuja de aire y grano de café (nivel).
pub fn hud_icon(name: &str) -> Tile {
    let mut c = Canvas::new();
    let heart = |x: f32, y: f32| {
        let (x, y) = ((x - 16.0) / 12.0, (16.0 - y) / 12.0 + 0.15);
        (x * x + y * y - 0.55).powi(3) - x * x * y * y * y < 0.0
    };
    let ham = |x: f32, y: f32| ((x - 18.0) / 10.0).powi(2) + ((y - 14.0) / 9.0).powi(2) < 1.0;
    let bone = |x: f32, y: f32| {
        (5.0..12.0).contains(&x) && (20.0..24.0).contains(&y)
            || ((x - 6.0).powi(2) + (y - 25.0).powi(2)) < 5.0
    };
    match name {
        "heart_full" | "heart_half" | "heart_empty" => {
            c.fill(hex("#3a1a1a"), hex("#4a2424"), 61, heart);
            if name != "heart_empty" {
                let half = name == "heart_half";
                c.fill(hex("#c42a2a"), hex("#ff5a4a"), 62, |x, y| {
                    heart(x, y) && (!half || x < 16.0)
                });
            }
        }
        "ham_full" | "ham_half" | "ham_empty" => {
            c.fill(hex("#3a2a20"), hex("#4a3628"), 63, |x, y| {
                ham(x, y) || bone(x, y)
            });
            if name != "ham_empty" {
                let half = name == "ham_half";
                c.fill(hex("#c0643a"), hex("#e89a6a"), 64, |x, y| {
                    ham(x, y) && (!half || x > 18.0)
                });
                c.fill(hex("#e8e0d0"), hex("#ffffff"), 65, |x, y| {
                    bone(x, y) && !half
                });
            }
        }
        "bubble" => {
            c.fill(hex("#5aa8e0"), hex("#bfe4ff"), 66, |x, y| {
                let d = ((x - 16.0).powi(2) + (y - 16.0).powi(2)).sqrt();
                (8.0..11.0).contains(&d) || ((x - 12.0).powi(2) + (y - 12.0).powi(2)) < 4.0
            });
        }
        _ => {
            // Grano de café con su surco.
            c.fill(hex("#4a2a18"), hex("#7a4a2a"), 67, |x, y| {
                ((x - 16.0) / 11.0).powi(2) + ((y - 16.0) / 14.0).powi(2) < 1.0
            });
            c.line(
                16.0,
                5.0,
                15.0,
                27.0,
                2.0,
                hex("#2a160c"),
                hex("#3a2010"),
                68,
            );
        }
    }
    c.tile()
}

pub const HUD_ICONS: [&str; 8] = [
    "heart_full",
    "heart_half",
    "heart_empty",
    "ham_full",
    "ham_half",
    "ham_empty",
    "bubble",
    "bean",
];

#[cfg(test)]
mod tests {
    use super::*;
    use survivalforest_core::item::items;

    fn opaque(t: &Tile) -> usize {
        t.chunks(4).filter(|p| p[3] > 128).count()
    }

    #[test]
    fn every_item_has_icon_not_blank() {
        for it in items().iter().filter(|i| i.places.is_none()) {
            let t = item_icon(&it.key).unwrap_or_else(|| panic!("sin icono: {}", it.key));
            assert_eq!(t.len(), T * T * 4);
            assert!(opaque(&t) >= 40, "{} casi vacío", it.key);
        }
        assert!(item_icon("stone").is_none());
        for n in HUD_ICONS {
            assert!(opaque(&hud_icon(n)) >= 40, "{n}");
        }
    }

    #[test]
    fn material_colors_distinct() {
        let mean = |k: &str| {
            let t = item_icon(k).unwrap();
            // Mitad de arriba: la cabeza de la herramienta.
            let px: Vec<&[u8]> = t[..T * T * 2].chunks(4).filter(|p| p[3] > 128).collect();
            let n = px.len().max(1) as u32;
            [0, 1, 2].map(|c| px.iter().map(|p| p[c] as u32).sum::<u32>() / n)
        };
        let m: Vec<[u32; 3]> = ["pick_wood", "pick_stone", "pick_copper", "pick_iron"]
            .iter()
            .map(|k| mean(k))
            .collect();
        for i in 0..m.len() {
            for j in i + 1..m.len() {
                let d: i64 = (0..3)
                    .map(|c| (m[i][c] as i64 - m[j][c] as i64).abs())
                    .sum();
                assert!(d > 30, "{i} y {j} se parecen: {:?} {:?}", m[i], m[j]);
            }
        }
    }
}

#[cfg(test)]
mod dump {
    /// `SF_ICONS=/ruta.rgba cargo test dump_icons -- --ignored`: vuelca los iconos en vertical.
    #[test]
    #[ignore]
    fn dump_icons() {
        let path = std::env::var("SF_ICONS").expect("SF_ICONS");
        let all: Vec<u8> = survivalforest_core::item::items()
            .iter()
            .filter_map(|i| super::item_icon(&i.key))
            .flatten()
            .collect();
        std::fs::write(path, all).unwrap();
    }
}
