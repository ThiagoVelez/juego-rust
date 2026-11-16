//! Recetas de crafteo y del horno (`data/recipes.ron`).

use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub enum RecipeKind {
    /// Filas de la rejilla (espacio = vacío) y qué objeto es cada letra.
    Shaped {
        rows: Vec<String>,
        key: Vec<(char, String)>,
    },
    /// Ingredientes en cualquier orden y posición.
    Shapeless(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub kind: RecipeKind,
    pub output: (String, u16),
}

impl Recipe {
    /// Claves de los ingredientes (con repeticiones en las recetas sin forma).
    pub fn ingredients(&self) -> Vec<&str> {
        match &self.kind {
            RecipeKind::Shaped { rows, key } => rows
                .iter()
                .flat_map(|r| r.chars())
                .filter(|c| *c != ' ')
                .filter_map(|c| key.iter().find(|(k, _)| *k == c).map(|(_, v)| v.as_str()))
                .collect(),
            RecipeKind::Shapeless(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }
    /// Ancho y alto de la forma (las recetas sin forma cuentan por su número de ingredientes).
    pub fn size(&self) -> (usize, usize) {
        match &self.kind {
            RecipeKind::Shaped { rows, .. } => (
                rows.iter().map(|r| r.chars().count()).max().unwrap_or(0),
                rows.len(),
            ),
            RecipeKind::Shapeless(v) => (v.len().min(3), v.len().div_ceil(3)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SmeltRecipe {
    pub input: String,
    pub output: String,
    pub xp: f32,
}

#[derive(Deserialize)]
struct RecipesRon {
    crafting: Vec<Recipe>,
    smelting: Vec<SmeltRecipe>,
}

fn data() -> &'static RecipesRon {
    static D: OnceLock<RecipesRon> = OnceLock::new();
    D.get_or_init(|| {
        ron::from_str(include_str!("../data/recipes.ron"))
            .unwrap_or_else(|e| panic!("data/recipes.ron: {e}"))
    })
}

pub fn recipes() -> &'static [Recipe] {
    &data().crafting
}

pub fn smelt_recipes() -> &'static [SmeltRecipe] {
    &data().smelting
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{item_by_key, items};

    #[test]
    fn recipes_reference_existing_items() {
        for r in recipes() {
            assert!(item_by_key(&r.output.0).is_some(), "{}", r.id);
            for k in r.ingredients() {
                assert!(item_by_key(k).is_some(), "{}: {k}", r.id);
            }
        }
        for s in smelt_recipes() {
            assert!(
                item_by_key(&s.input).is_some() && item_by_key(&s.output).is_some(),
                "{}",
                s.input
            );
        }
    }

    #[test]
    fn every_tool_and_station_is_craftable() {
        let outs: Vec<&str> = recipes().iter().map(|r| r.output.0.as_str()).collect();
        for it in items().iter().filter(|i| i.tool.is_some()) {
            assert!(outs.contains(&it.key.as_str()), "{}", it.key);
        }
        for k in [
            "crafting_table",
            "furnace",
            "chest",
            "grindstone",
            "torch",
            "bricks",
            "roof_tile",
            "whitewash",
            "planks",
            "stick",
        ] {
            assert!(outs.contains(&k), "{k}");
        }
    }

    #[test]
    fn smelting_covers_spec() {
        for k in [
            "copper_ore_item",
            "iron_ore_item",
            "gold_ore_item",
            "sand",
            "red_sand",
            "clay",
            "platano",
            "log",
            "cobble",
        ] {
            assert!(smelt_recipes().iter().any(|s| s.input == k), "{k}");
        }
    }

    /// Formas normalizadas: sin filas ni columnas vacías y cada ingrediente cambiado por A, B, C…
    /// en orden de aparición.
    fn normalize(rows: &[&str]) -> Vec<String> {
        let grid: Vec<Vec<char>> = rows.iter().map(|r| r.chars().collect()).collect();
        let w = grid.iter().map(|r| r.len()).max().unwrap_or(0);
        let cell = |y: usize, x: usize| grid[y].get(x).copied().unwrap_or(' ');
        let ys: Vec<usize> = (0..grid.len())
            .filter(|&y| (0..w).any(|x| cell(y, x) != ' '))
            .collect();
        let xs: Vec<usize> = (0..w)
            .filter(|&x| (0..grid.len()).any(|y| cell(y, x) != ' '))
            .collect();
        let mut names: Vec<char> = vec![];
        ys.iter()
            .map(|&y| {
                xs.iter()
                    .map(|&x| {
                        let c = cell(y, x);
                        if c == ' ' {
                            return '.';
                        }
                        let i = names.iter().position(|&n| n == c).unwrap_or_else(|| {
                            names.push(c);
                            names.len() - 1
                        });
                        (b'A' + i as u8) as char
                    })
                    .collect()
            })
            .collect()
    }

    fn mirror(rows: &[String]) -> Vec<String> {
        rows.iter().map(|r| r.chars().rev().collect()).collect()
    }

    #[test]
    fn no_forbidden_shapes() {
        // Formas de las recetas equivalentes de Minecraft (no se pueden repetir).
        let forbidden: &[(&str, &[&str])] = &[
            ("pick_", &["MMM", " S ", " S "]),
            ("axe_", &["MM", "MS", " S"]),
            ("shovel_", &["M", "S", "S"]),
            ("machete_", &["M", "M", "S"]),
            ("crafting_table", &["PP", "PP"]),
            ("furnace", &["CCC", "C C", "CCC"]),
            ("chest", &["PPP", "P P", "PPP"]),
            ("torch", &["C", "S"]),
            ("stick", &["P", "P"]),
            ("bricks", &["BB", "BB"]),
        ];
        for r in recipes() {
            let RecipeKind::Shaped { rows, .. } = &r.kind else {
                continue;
            };
            let rows: Vec<&str> = rows.iter().map(|s| s.as_str()).collect();
            let mine = normalize(&rows);
            for (prefix, shape) in forbidden {
                if r.output.0.starts_with(prefix) {
                    let f = normalize(shape);
                    assert!(
                        mine != f && mine != mirror(&f) && mirror(&mine) != f,
                        "{} repite una forma prohibida",
                        r.id
                    );
                }
            }
        }
        // La prueba detecta de verdad una forma prohibida.
        assert_eq!(
            normalize(&["MMM", " S ", " S "]),
            normalize(&["III", " T ", " T "])
        );
    }

    #[test]
    fn shapes_fit_3x3_and_keys_complete() {
        for r in recipes() {
            if let RecipeKind::Shaped { rows, key } = &r.kind {
                assert!(
                    rows.len() <= 3 && rows.iter().all(|s| s.chars().count() <= 3),
                    "{}",
                    r.id
                );
                for c in rows.iter().flat_map(|s| s.chars()).filter(|c| *c != ' ') {
                    assert!(key.iter().any(|(k, _)| *k == c), "{}: falta '{c}'", r.id);
                }
            }
        }
    }
}
