//! Crafteo en rejillas de 2×2 (inventario) y 3×3 (mesa de trabajo).

use crate::inventory::ItemStack;
use survivalforest_core::item::{item, item_by_key};
use survivalforest_core::recipe::{Recipe, RecipeKind, recipes};

/// Celdas ocupadas (fila, columna, clave) y el rectángulo que las encierra.
struct Occupied {
    cells: Vec<(usize, usize, &'static str)>,
    y0: usize,
    x0: usize,
    h: usize,
    w: usize,
}

fn occupied(grid: &[Option<ItemStack>], w: usize) -> Option<Occupied> {
    let cells: Vec<_> = grid
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            s.as_ref()
                .map(|s| (i / w, i % w, item(s.item).key.as_str()))
        })
        .collect();
    if cells.is_empty() {
        return None;
    }
    let y0 = cells.iter().map(|c| c.0).min()?;
    let y1 = cells.iter().map(|c| c.0).max()?;
    let x0 = cells.iter().map(|c| c.1).min()?;
    let x1 = cells.iter().map(|c| c.1).max()?;
    Some(Occupied {
        cells,
        y0,
        x0,
        h: y1 - y0 + 1,
        w: x1 - x0 + 1,
    })
}

fn shaped_matches(
    rows: &[String],
    key: &[(char, String)],
    grid: &[Option<ItemStack>],
    w: usize,
    y0: usize,
    x0: usize,
    mirrored: bool,
) -> bool {
    let rw = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    for (ry, row) in rows.iter().enumerate() {
        let chars: Vec<char> = row.chars().collect();
        for rx in 0..rw {
            let c = chars
                .get(if mirrored { rw - 1 - rx } else { rx })
                .copied()
                .unwrap_or(' ');
            let want = (c != ' ')
                .then(|| key.iter().find(|(k, _)| *k == c).map(|(_, v)| v.as_str()))
                .flatten();
            let have = grid[(y0 + ry) * w + x0 + rx]
                .as_ref()
                .map(|s| item(s.item).key.as_str());
            if want != have {
                return false;
            }
        }
    }
    true
}

/// Receta que forma la rejilla (de `w`×`w`), sin importar dónde esté ni si está reflejada.
pub fn match_recipe(grid: &[Option<ItemStack>], w: usize) -> Option<&'static Recipe> {
    let Occupied {
        cells,
        y0,
        x0,
        h,
        w: bw,
    } = occupied(grid, w)?;
    recipes().iter().find(|r| match &r.kind {
        RecipeKind::Shaped { rows, key } => {
            let rw = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
            // Las filas y columnas vacías de la receta no cuentan.
            rows.len() == h
                && rw == bw
                && (shaped_matches(rows, key, grid, w, y0, x0, false)
                    || shaped_matches(rows, key, grid, w, y0, x0, true))
        }
        RecipeKind::Shapeless(list) => {
            let mut want: Vec<&str> = list.iter().map(|s| s.as_str()).collect();
            let mut have: Vec<&str> = cells.iter().map(|c| c.2).collect();
            want.sort();
            have.sort();
            want == have
        }
    })
}

/// Hace la receta: gasta uno de cada casilla usada y devuelve el resultado.
pub fn craft(grid: &mut [Option<ItemStack>], w: usize) -> Option<ItemStack> {
    let r = match_recipe(grid, w)?;
    let out = ItemStack::new(item_by_key(&r.output.0)?, r.output.1);
    for s in grid.iter_mut() {
        if let Some(st) = s {
            st.count -= 1;
            if st.count == 0 {
                *s = None;
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::ItemStack;
    use survivalforest_core::item::item_by_key;

    fn st(k: &str) -> Option<ItemStack> {
        Some(ItemStack::new(item_by_key(k).unwrap(), 1))
    }

    fn grid(w: usize, cells: &[(usize, &str)]) -> Vec<Option<ItemStack>> {
        let mut g = vec![None; w * w];
        for (i, k) in cells {
            g[*i] = st(k);
        }
        g
    }

    #[test]
    fn planks_from_log_anywhere_in_2x2() {
        for i in 0..4 {
            let g = grid(2, &[(i, "log")]);
            assert_eq!(match_recipe(&g, 2).unwrap().id, "planks");
        }
    }

    #[test]
    fn pickaxe_needs_3x3_and_matches() {
        // M M / MSM / .S.
        let g = grid(
            3,
            &[
                (0, "planks"),
                (2, "planks"),
                (3, "planks"),
                (4, "stick"),
                (5, "planks"),
                (7, "stick"),
            ],
        );
        assert_eq!(match_recipe(&g, 3).unwrap().id, "pick_wood");
        // En 2×2 no cabe.
        let small = grid(2, &[(0, "planks"), (1, "planks"), (2, "stick")]);
        assert_ne!(
            match_recipe(&small, 2).map(|r| r.id.as_str()),
            Some("pick_wood")
        );
    }

    #[test]
    fn shifted_shape_matches() {
        // Pala «MM / S. / S.» desplazada una columna a la derecha.
        let g = grid(
            3,
            &[(1, "cobble"), (2, "cobble"), (4, "stick"), (7, "stick")],
        );
        assert_eq!(match_recipe(&g, 3).unwrap().id, "shovel_stone");
    }

    #[test]
    fn mirrored_shape_matches() {
        // Pala reflejada: «MM / .S / .S».
        let g = grid(
            3,
            &[(0, "cobble"), (1, "cobble"), (4, "stick"), (7, "stick")],
        );
        assert_eq!(match_recipe(&g, 3).unwrap().id, "shovel_stone");
    }

    #[test]
    fn shapeless_matches_any_order() {
        let a = grid(2, &[(0, "coal"), (3, "stick")]);
        let b = grid(2, &[(1, "stick"), (2, "coal")]);
        assert_eq!(match_recipe(&a, 2).unwrap().id, "torch_coal");
        assert_eq!(match_recipe(&b, 2).unwrap().id, "torch_coal");
        let extra = grid(2, &[(0, "coal"), (1, "stick"), (2, "stick")]);
        assert!(match_recipe(&extra, 2).is_none());
    }

    #[test]
    fn craft_consumes_one_each() {
        let mut g = grid(2, &[(0, "log")]);
        g[0].as_mut().unwrap().count = 3;
        let out = craft(&mut g, 2).unwrap();
        assert_eq!(out.item, item_by_key("planks").unwrap());
        assert_eq!(out.count, 3);
        assert_eq!(g[0].as_ref().unwrap().count, 2);
        let mut empty = grid(2, &[]);
        assert!(craft(&mut empty, 2).is_none());
    }
}
