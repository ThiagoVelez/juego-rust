//! Carpeta del mundo: datos del mundo y del jugador en RON.

use crate::region::write_atomic;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use survivalforest_core::GameMode;

pub const META_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldMeta {
    pub version: u32,
    pub name: String,
    pub seed: u64,
    pub mode: GameMode,
    pub time_ticks: u64,
    /// Segundos desde 1970 (texto para que sea legible en el RON).
    pub created: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerSave {
    pub pos: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub flying: bool,
}

/// `%APPDATA%\SurvivalForest\saves` o `~/.local/share/SurvivalForest/saves`.
pub fn data_dir() -> PathBuf {
    if cfg!(windows) {
        if let Some(a) = std::env::var_os("APPDATA") {
            return PathBuf::from(a).join("SurvivalForest");
        }
    }
    if let Some(x) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(x).join("SurvivalForest");
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".local").join("share").join("SurvivalForest")
}

pub fn worlds_dir() -> PathBuf {
    data_dir().join("saves")
}

fn ron_write<T: Serialize>(path: &Path, v: &T) -> io::Result<()> {
    let text = ron::ser::to_string_pretty(v, ron::ser::PrettyConfig::default())
        .map_err(io::Error::other)?;
    write_atomic(path, text.as_bytes())
}

fn ron_read<T: for<'de> Deserialize<'de>>(path: &Path) -> io::Result<T> {
    let text = fs::read_to_string(path)?;
    ron::from_str(&text).map_err(io::Error::other)
}

fn folder_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim().to_string();
    if s.is_empty() { "Mundo".into() } else { s }
}

pub fn create_world_in(root: &Path, name: &str, seed: u64, mode: GameMode) -> io::Result<PathBuf> {
    fs::create_dir_all(root)?;
    let base = folder_name(name);
    let mut dir = root.join(&base);
    let mut n = 2;
    while dir.exists() {
        dir = root.join(format!("{base} ({n})"));
        n += 1;
    }
    fs::create_dir_all(dir.join("region"))?;
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        .to_string();
    let meta = WorldMeta {
        version: META_VERSION,
        name: name.to_string(),
        seed,
        mode,
        time_ticks: 1000,
        created,
    };
    save_meta(&dir, &meta)?;
    Ok(dir)
}

pub fn create_world(name: &str, seed: u64, mode: GameMode) -> io::Result<PathBuf> {
    create_world_in(&worlds_dir(), name, seed, mode)
}

pub fn list_worlds_in(root: &Path) -> Vec<(PathBuf, WorldMeta)> {
    let Ok(rd) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut v: Vec<(PathBuf, WorldMeta)> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter_map(|p| load_meta(&p).ok().map(|m| (p, m)))
        .collect();
    v.sort_by(|a, b| a.1.name.cmp(&b.1.name));
    v
}

pub fn list_worlds() -> Vec<(PathBuf, WorldMeta)> {
    list_worlds_in(&worlds_dir())
}

pub fn load_meta(dir: &Path) -> io::Result<WorldMeta> {
    ron_read(&dir.join("world.ron"))
}

pub fn save_meta(dir: &Path, m: &WorldMeta) -> io::Result<()> {
    ron_write(&dir.join("world.ron"), m)
}

pub fn load_player(dir: &Path) -> io::Result<PlayerSave> {
    ron_read(&dir.join("player.ron"))
}

pub fn save_player(dir: &Path, p: &PlayerSave) -> io::Result<()> {
    ron_write(&dir.join("player.ron"), p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use survivalforest_core::GameMode;

    #[test]
    fn meta_roundtrip_ron() {
        let d = tempfile::tempdir().unwrap();
        let dir = create_world_in(d.path(), "Mi mundo", 42, GameMode::Creative).unwrap();
        let mut m = load_meta(&dir).unwrap();
        assert_eq!(
            (m.name.as_str(), m.seed, m.mode),
            ("Mi mundo", 42, GameMode::Creative)
        );
        m.time_ticks = 1234;
        save_meta(&dir, &m).unwrap();
        assert_eq!(load_meta(&dir).unwrap().time_ticks, 1234);
        let p = PlayerSave {
            pos: [1.5, 80.0, -3.25],
            yaw: 0.5,
            pitch: -0.2,
            flying: true,
        };
        save_player(&dir, &p).unwrap();
        assert_eq!(load_player(&dir).unwrap(), p);
    }

    #[test]
    fn list_worlds_skips_garbage_dirs() {
        let d = tempfile::tempdir().unwrap();
        create_world_in(d.path(), "Uno", 1, GameMode::Survival).unwrap();
        create_world_in(d.path(), "Uno", 2, GameMode::Survival).unwrap();
        std::fs::create_dir_all(d.path().join("basura")).unwrap();
        std::fs::write(d.path().join("roto.txt"), "x").unwrap();
        let ws = list_worlds_in(d.path());
        assert_eq!(ws.len(), 2);
        assert!(ws.iter().all(|(_, m)| m.name == "Uno"));
    }
}
