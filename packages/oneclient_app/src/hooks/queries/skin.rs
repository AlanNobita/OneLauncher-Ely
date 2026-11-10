use bytes::Bytes;
use freya::prelude::*;

use crate::AppAssets;

pub fn use_player_skin(uuid: String) -> (Bytes, bool) {
    // Always resolve the remote state first so hook order stays stable when a
    // local skin is added or removed.
    let profile = super::use_player_profile(uuid.clone(), None::<String>);

    if let Some(bytes) = local_skin_bytes(&uuid) {
        return (bytes, false);
    }

    let (skin_url, is_slim) = crate::hooks::settled_or_loading(&profile)
        .map_or((None, false), |profile| (profile.skin_url, profile.is_slim));

    let skin_query = super::use_cached_image(skin_url.clone(), 256);

    let steve = use_memo(|| AppAssets::get_bytes("steve.png").unwrap_or_default());
    let alex = use_memo(|| AppAssets::get_bytes("alex.png").unwrap_or_default());

    let default_slim = (java_string_hash(&uuid) & 1) == 1;

    match crate::hooks::loaded_image(skin_url.as_deref(), &skin_query) {
        Some((_, bytes)) => (bytes, is_slim),
        None if default_slim => (alex.read().clone(), true),
        None => (steve.read().clone(), false),
    }
}

fn java_string_hash(s: &str) -> i32 {
    let mut h: i32 = 0;
    for c in s.encode_utf16() {
        h = h.wrapping_mul(31).wrapping_add(c as i32);
    }
    h
}

fn local_skin_bytes(uuid: &str) -> Option<Bytes> {
    let path = oneclient_common::paths::local_skin_file(uuid).ok()?;
    let bytes = std::fs::read(path).ok()?;
    normalize_local_texture(Bytes::from(bytes), 64, 64)
}

pub fn local_cape_bytes(uuid: &str) -> Option<Bytes> {
    let path = oneclient_common::paths::local_cape_file(uuid).ok()?;
    let bytes = std::fs::read(path).ok()?;
    normalize_local_texture(Bytes::from(bytes), 64, 32)
}

pub fn use_player_cape(uuid: &str) -> Option<Bytes> {
    local_cape_bytes(uuid)
}

fn normalize_local_texture(bytes: Bytes, width: u32, height: u32) -> Option<Bytes> {
    let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).ok()?;
    if image.width() == width && image.height() == height {
        return Some(bytes);
    }
    let normalized = image.resize_exact(width, height, image::imageops::FilterType::Nearest);
    let mut output = std::io::Cursor::new(Vec::new());
    normalized.write_to(&mut output, image::ImageFormat::Png).ok()?;
    Some(Bytes::from(output.into_inner()))
}

/// Shared by the picker and the skin query. Checks PNG geometry before copying.
pub fn validate_local_skin_png(bytes: &[u8]) -> Result<(), String> {
    validate_local_texture_png(bytes, TextureType::Skin)
}

/// Shared by the picker and the cape query. Checks PNG geometry before copying.
pub fn validate_local_cape_png(bytes: &[u8]) -> Result<(), String> {
    validate_local_texture_png(bytes, TextureType::Cape)
}

#[derive(Clone, Copy)]
enum TextureType {
    Skin,
    Cape,
}

fn validate_local_texture_png(bytes: &[u8], texture: TextureType) -> Result<(), String> {
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|err| format!("That file is not a valid PNG: {err}"))?;
    let (width, height) = (image.width(), image.height());
    match texture {
        TextureType::Skin => {
            if width != height {
                return Err("Skin PNG must be square (64x64 to 2048x2048)".to_string());
            }
            if !is_power_of_two(width) || !(64..=2048).contains(&width) {
                return Err("Skin PNG must be 64, 128, 256, 512, 1024 or 2048 pixels per side".to_string());
            }
        }
        TextureType::Cape => {
            if width != height * 2 {
                return Err("Cape PNG must be 2:1 (for example 64x32 or 2048x1024)".to_string());
            }
            if !is_power_of_two(width) || !is_power_of_two(height)
                || !(64..=2048).contains(&width)
            {
                return Err("Cape PNG must be 64x32, 128x64, 256x128, 512x256, 1024x512 or 2048x1024 pixels".to_string());
            }
        }
    }
    Ok(())
}

fn is_power_of_two(n: u32) -> bool {
    n != 0 && n.is_power_of_two()
}
