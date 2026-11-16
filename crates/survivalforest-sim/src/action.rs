//! Acciones del jugador. El cliente las aplica en local; en el multijugador las enviará al servidor.

use glam::IVec3;
use survivalforest_core::BlockPos;

/// Contenedor de una casilla en pantalla.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ContainerRef {
    Inventory,
    /// Rejilla de crafteo (2×2 en el inventario, 3×3 en la mesa) y su resultado.
    CraftGrid,
    CraftResult,
    Chest(BlockPos),
    FurnaceInput(BlockPos),
    FurnaceFuel(BlockPos),
    FurnaceOutput(BlockPos),
    GrindstoneTool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    StartBreak(BlockPos),
    CancelBreak,
    Use(BlockPos, IVec3),
    SelectSlot(u8),
    Drop {
        all: bool,
    },
    SlotClick {
        container: ContainerRef,
        slot: usize,
        right: bool,
        shift: bool,
    },
    Craft,
    Respawn,
    Eat,
}
