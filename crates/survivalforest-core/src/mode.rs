use serde::{Deserialize, Serialize};

/// Modo de juego de un mundo.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
}
