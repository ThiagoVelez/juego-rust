//! Reglas de romper y poner bloques.

use crate::physics::Aabb;
use survivalforest_core::BlockPos;
use survivalforest_world::ChunkAccess;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    OutOfWorld,
    Unbreakable,
    InsidePlayer,
    Occupied,
}

pub fn can_break(w: &impl ChunkAccess, pos: BlockPos) -> Result<(), EditError> {
    if !pos.in_height() {
        return Err(EditError::OutOfWorld);
    }
    match w.block_at(pos) {
        Some(b) if b.def().breakable => Ok(()),
        Some(_) => Err(EditError::Unbreakable),
        None => Err(EditError::OutOfWorld),
    }
}

pub fn can_place(w: &impl ChunkAccess, pos: BlockPos, player: &Aabb) -> Result<(), EditError> {
    if !pos.in_height() {
        return Err(EditError::OutOfWorld);
    }
    match w.block_at(pos) {
        Some(b) if b.is_air() || b == survivalforest_core::block::WATER => {}
        Some(_) => return Err(EditError::Occupied),
        None => return Err(EditError::OutOfWorld),
    }
    if player.intersects_block(pos) {
        return Err(EditError::InsidePlayer);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    use std::collections::HashMap;
    use survivalforest_core::block::{AIR, BEDROCK, STONE, WATER};
    use survivalforest_core::{ChunkPos, MAX_Y, MIN_Y};
    use survivalforest_world::ChunkColumn;

    fn world() -> HashMap<ChunkPos, ChunkColumn> {
        let mut c = ChunkColumn::new(ChunkPos::new(0, 0));
        c.set(BlockPos::new(0, MIN_Y, 0).local(), BEDROCK);
        c.set(BlockPos::new(1, 70, 1).local(), STONE);
        c.set(BlockPos::new(2, 70, 2).local(), WATER);
        let mut m = HashMap::new();
        m.insert(c.pos, c);
        m
    }

    #[test]
    fn cannot_place_inside_player() {
        let w = world();
        let me = Aabb::player(DVec3::new(5.5, 70.0, 5.5));
        assert_eq!(
            can_place(&w, BlockPos::new(5, 71, 5), &me),
            Err(EditError::InsidePlayer)
        );
        assert_eq!(can_place(&w, BlockPos::new(5, 72, 5), &me), Ok(()));
    }

    #[test]
    fn cannot_place_above_world_top() {
        let w = world();
        let me = Aabb::player(DVec3::new(5.5, 70.0, 5.5));
        assert_eq!(
            can_place(&w, BlockPos::new(3, MAX_Y, 3), &me),
            Err(EditError::OutOfWorld)
        );
        assert_eq!(can_place(&w, BlockPos::new(3, MAX_Y - 1, 3), &me), Ok(()));
    }

    #[test]
    fn cannot_place_below_bottom() {
        let w = world();
        let me = Aabb::player(DVec3::new(5.5, 70.0, 5.5));
        assert_eq!(
            can_place(&w, BlockPos::new(3, MIN_Y - 1, 3), &me),
            Err(EditError::OutOfWorld)
        );
    }

    #[test]
    fn place_rules_for_occupied_and_water() {
        let w = world();
        let me = Aabb::player(DVec3::new(5.5, 70.0, 5.5));
        assert_eq!(
            can_place(&w, BlockPos::new(1, 70, 1), &me),
            Err(EditError::Occupied)
        );
        assert_eq!(can_place(&w, BlockPos::new(2, 70, 2), &me), Ok(()));
        assert_eq!(
            w[&ChunkPos::new(0, 0)].get(BlockPos::new(3, 70, 3).local()),
            AIR
        );
    }

    #[test]
    fn cannot_break_bedrock() {
        let w = world();
        assert_eq!(
            can_break(&w, BlockPos::new(0, MIN_Y, 0)),
            Err(EditError::Unbreakable)
        );
        assert_eq!(can_break(&w, BlockPos::new(1, 70, 1)), Ok(()));
        assert_eq!(
            can_break(&w, BlockPos::new(1, MAX_Y, 1)),
            Err(EditError::OutOfWorld)
        );
    }
}
