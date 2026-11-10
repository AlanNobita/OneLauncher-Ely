//! Local offline skins and capes exposed to Minecraft through CustomSkinLoader.
//!
//! OneClient stores the source texture keyed by UUID next to its other local
//! content. CustomSkinLoader's *local files* API, by contrast, reads a fixed
//! file named after the Minecraft username from the instance directory. The
//! launcher mirrors the local UUID-keyed textures into that layout before the
//! game starts. It does not download or install CustomSkinLoader or 3D Skin
//! Layers; those are chosen separately because they are mods, not launcher data.
//!
//! A small manifest records the files OneClient owns under `LocalSkin` so stale
//! files from a previous launch can be removed without touching textures that
//! were written by the user or by another tool.

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use oneclient_common::paths;
use tokio::fs;

pub(crate) async fn prepare_local_skin_files(
    game_dir: &Path,
    username: &str,
    uuid: uuid::Uuid,
) -> Result<(), std::io::Error> {
    // The username becomes a filename in CustomSkinLoader's local skins folder.
    if username.is_empty()
        || !username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        tracing::warn!(
            %username,
            "skipping local skin sync because the username is not a safe filename"
        );
        return Ok(());
    }

    let base = game_dir.join("CustomSkinLoader").join("LocalSkin");
    polyio::create_dir_all(&base).await.map_err(io_err)?;

    let key = uuid.to_string();
    let skin = paths::local_skin_file(&key).map_err(io_err)?;
    let cape = paths::local_cape_file(&key).map_err(io_err)?;
    let textures = [("skins", skin), ("capes", cape)];

    let manifest_path = base.join(".oneclient_local_skins.json");
    let mut manifest = try_read_manifest(&manifest_path).await;
    let previous: HashSet<String> = manifest
        .get(&key)
        .into_iter()
        .flatten()
        .filter(|rel| is_safe_relative(rel))
        .cloned()
        .collect();

    let target_name = format!("{username}.png");
    let mut desired = HashSet::new();
    for (kind, source) in textures {
        if !source.is_file() {
            continue;
        }
        let rel = format!("{kind}/{target_name}");
        let target = base.join(&rel);
        sync_local_texture(&source, &target).await?;
        desired.insert(rel);
    }

    for stale in previous.difference(&desired) {
        if !is_safe_relative(stale) {
            continue;
        }
        let path = base.join(stale);
        if path.is_file() {
            fs::remove_file(path).await?;
        }
    }

    if desired.is_empty() {
        manifest.remove(&key);
    } else {
        let mut files: Vec<String> = desired.into_iter().collect();
        files.sort();
        manifest.insert(key, files);
    }

    polyio::write_json_atomic(&manifest_path, &manifest)
        .await
        .map_err(io_err)?;

    Ok(())
}

async fn sync_local_texture(source: &Path, target: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = target.parent() {
        polyio::create_dir_all(parent).await.map_err(io_err)?;
    }
    fs::copy(source, target).await.map(|_| ())
}

async fn try_read_manifest(path: &Path) -> BTreeMap<String, Vec<String>> {
    match fs::read(path).await {
        Ok(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|value| {
                let accounts = value.get("accounts").unwrap_or(&value); // tolerate the old derived shape
                accounts.as_object().cloned()
            })
            .map(|accounts| {
                accounts
                    .into_iter()
                    .filter_map(|(uuid, files)| {
                        let files = files
                            .as_array()?
                            .iter()
                            .filter_map(|file| file.as_str().map(str::to_string))
                            .collect();
                        Some((uuid, files))
                    })
                    .collect()
            })
            .unwrap_or_default(),
        Err(_) => BTreeMap::new(),
    }
}

fn is_safe_relative(rel: &str) -> bool {
    let path = PathBuf::from(rel);
    matches!(
        path.iter()
            .next()
            .map(|part| part.to_string_lossy()),
        Some(kind) if kind == "skins" || kind == "capes"
    ) && path.components().count() == 2
        && !path.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
        && path.extension().is_some_and(|ext| ext == "png")
}

fn io_err(err: impl Display) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, err.to_string())
}
