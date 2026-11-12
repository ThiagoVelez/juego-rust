//! Ajustes del jugador (RON junto a las partidas) y semillas a partir de texto.

use serde::{Deserialize, Serialize};

#[derive(bevy::prelude::Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Chunks (4..=32).
    pub view_distance: u32,
    /// Grados (60..=110).
    pub fov: f32,
    /// 0,1..=4.
    pub sensitivity: f32,
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            view_distance: 12,
            fov: 75.0,
            sensitivity: 1.0,
            language: "es".into(),
        }
    }
}

impl Settings {
    pub fn clamped(mut self) -> Self {
        self.view_distance = self.view_distance.clamp(4, 32);
        self.fov = self.fov.clamp(60.0, 110.0);
        self.sensitivity = self.sensitivity.clamp(0.1, 4.0);
        self
    }

    fn path() -> std::path::PathBuf {
        survivalforest_world::save::data_dir().join("settings.ron")
    }

    pub fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|t| ron::from_str::<Settings>(&t).ok())
            .unwrap_or_default()
            .clamped()
    }

    pub fn save(&self) {
        let p = Self::path();
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if let Ok(t) = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            let _ = std::fs::write(p, t);
        }
    }
}

/// Semilla a partir de lo que escribe el jugador: un número tal cual o un texto
/// convertido con FNV-1a. Vacío = `None` (semilla aleatoria).
pub fn seed_from_text(text: &str) -> Option<u64> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    if let Ok(n) = t.parse::<u64>() {
        return Some(n);
    }
    Some(t.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
    }))
}

/// Semilla aleatoria (reloj del sistema mezclado).
pub fn random_seed() -> u64 {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(1);
    seed_from_text(&format!("sf-{n}")).unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_from_text_is_stable() {
        assert_eq!(seed_from_text("cafetal"), seed_from_text("cafetal"));
        assert_ne!(seed_from_text("cafetal"), seed_from_text("Cafetal"));
        assert_eq!(seed_from_text("12345"), Some(12345));
        assert_eq!(seed_from_text("  "), None);
    }

    #[test]
    fn settings_clamp() {
        let s = Settings {
            view_distance: 99,
            fov: 300.0,
            sensitivity: -1.0,
            language: "es".into(),
        }
        .clamped();
        assert_eq!(s.view_distance, 32);
        assert_eq!(s.fov, 110.0);
        assert!(s.sensitivity >= 0.1);
        assert_eq!(Settings::default().view_distance, 12);
    }
}
