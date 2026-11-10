use oneclient_common::paths;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalTexture {
    Skin,
    Cape,
}

impl LocalTexture {
    fn validate(self, bytes: &[u8]) -> Result<(), String> {
        match self {
            Self::Skin => crate::hooks::validate_local_skin_png(bytes),
            Self::Cape => crate::hooks::validate_local_cape_png(bytes),
        }
    }

    fn file(self, uuid: &str) -> oneclient_common::PathsResult<std::path::PathBuf> {
        match self {
            Self::Skin => paths::local_skin_file(uuid),
            Self::Cape => paths::local_cape_file(uuid),
        }
    }
}

pub(crate) async fn pick_and_save(
    texture: LocalTexture,
    uuid: uuid::Uuid,
) -> Result<bool, String> {
    let title = match texture {
        LocalTexture::Skin => crate::i18n::t("oneclient.settings.accounts.local_skin.pick_title"),
        LocalTexture::Cape => crate::i18n::t("oneclient.settings.accounts.local_cape.pick_title"),
    };
    let handle = match rfd::AsyncFileDialog::new()
        .set_title(title)
        .add_filter("PNG image", &["png"])
        .pick_file()
        .await
    {
        Some(handle) => handle,
        None => return Ok(false),
    };

    let bytes = tokio::fs::read(handle.path())
        .await
        .map_err(|err| err.to_string())?;
    texture.validate(&bytes)?;

    let path = texture
        .file(&uuid.to_string())
        .map_err(|err| err.to_string())?;
    if let Some(parent) = path.parent() {
        polyio::create_dir_all(parent)
            .await
            .map_err(|err| err.to_string())?;
    }
    tokio::fs::write(path, bytes).await.map_err(|err| err.to_string())?;
    Ok(true)
}

pub(crate) async fn remove(texture: LocalTexture, uuid: uuid::Uuid) -> Result<(), String> {
    let path = texture
        .file(&uuid.to_string())
        .map_err(|err| err.to_string())?;
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}
