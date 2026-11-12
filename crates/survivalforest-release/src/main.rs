//! Genera el `manifest.json` de una versión de SurvivalForest para el Launcher de CoffeeWorks.
//!
//! sf-release --version 0.1.0 --repo ThiagoVelez/juego-rust \
//!     --windows dist/survivalforest-0.1.0-windows-x64.zip \
//!     --linux dist/survivalforest-0.1.0-linux-x64.tar.gz > manifest.json

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Serialize, Debug, PartialEq)]
pub struct PlatformFile {
    pub file: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    /// Ejecutable dentro del archivo.
    pub exe: String,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub stage: String,
    pub platforms: BTreeMap<String, PlatformFile>,
}

/// Etapa del juego que se muestra en el Launcher.
pub const STAGE: &str = "Etapa 1 (mundo)";

fn platform_file(
    repo: &str,
    version: &str,
    path: &Path,
    exe: &str,
) -> Result<PlatformFile, String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let file = path
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or("nombre de archivo no válido")?
        .to_string();
    Ok(PlatformFile {
        url: format!("https://github.com/{repo}/releases/download/v{version}/{file}"),
        file,
        size: bytes.len() as u64,
        sha256: Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        exe: exe.to_string(),
    })
}

pub fn build_manifest(
    version: &str,
    repo: &str,
    windows: Option<&Path>,
    linux: Option<&Path>,
) -> Result<Manifest, String> {
    let mut platforms = BTreeMap::new();
    if let Some(p) = windows {
        platforms.insert(
            "windows-x64".to_string(),
            platform_file(repo, version, p, "survivalforest.exe")?,
        );
    }
    if let Some(p) = linux {
        platforms.insert(
            "linux-x64".to_string(),
            platform_file(repo, version, p, "survivalforest")?,
        );
    }
    Ok(Manifest {
        id: "survivalforest".into(),
        name: "SurvivalForest".into(),
        version: version.into(),
        stage: STAGE.into(),
        platforms,
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |k: &str| {
        args.iter()
            .position(|a| a == k)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let (Some(version), Some(repo)) = (get("--version"), get("--repo")) else {
        eprintln!(
            "Uso: sf-release --version X.Y.Z --repo dueño/repo [--windows ARCHIVO] [--linux ARCHIVO]"
        );
        std::process::exit(2);
    };
    let win = get("--windows").map(std::path::PathBuf::from);
    let lin = get("--linux").map(std::path::PathBuf::from);
    match build_manifest(&version, &repo, win.as_deref(), lin.as_deref()) {
        Ok(m) => println!("{}", serde_json::to_string_pretty(&m).expect("json")),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_lists_both_platforms_with_hashes() {
        let d = tempfile::tempdir().unwrap();
        let win = d.path().join("survivalforest-0.1.0-windows-x64.zip");
        let lin = d.path().join("survivalforest-0.1.0-linux-x64.tar.gz");
        std::fs::write(&win, b"abc").unwrap();
        std::fs::write(&lin, b"hola").unwrap();
        let m = build_manifest("0.1.0", "ThiagoVelez/juego-rust", Some(&win), Some(&lin)).unwrap();
        assert_eq!(m.id, "survivalforest");
        assert_eq!(m.version, "0.1.0");
        let w = &m.platforms["windows-x64"];
        assert_eq!(
            w.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(w.size, 3);
        assert_eq!(w.exe, "survivalforest.exe");
        assert_eq!(
            w.url,
            "https://github.com/ThiagoVelez/juego-rust/releases/download/v0.1.0/survivalforest-0.1.0-windows-x64.zip"
        );
        assert_eq!(m.platforms["linux-x64"].exe, "survivalforest");
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"stage\""));
    }

    #[test]
    fn missing_file_is_an_error() {
        assert!(build_manifest("0.1.0", "a/b", Some(Path::new("/no/existe.zip")), None).is_err());
    }
}
