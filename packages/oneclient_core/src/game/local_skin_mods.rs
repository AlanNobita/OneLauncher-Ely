//! Installs the client-side mods needed for local offline skins and capes.
//!
//! This is opt-in via `LauncherSettings::auto_install_local_skin_mods`. It is
//! deliberately not part of the game launch critical path: a network outage is
//! logged and the game still starts.

use std::collections::HashSet;

use oneclient_common::domain::{GameLoader, ProviderId};
use oneclient_common::paths;
use oneclient_content::ContentResult;
use oneclient_content::packages::provider::PackageProvider;
use oneclient_content::packages::store::PackageStore;
use oneclient_content::packages::types::ProjectDetail;
use oneclient_content::ContentCtx;

use crate::clusters::Cluster;
use crate::state::LauncherState;

const CUSTOM_SKIN_LOADER_PROJECT: &str = "idMHQ4n2";
const SKIN_LAYERS_PROJECT: &str = "zV5r3pPn";
const SKIN_LAYERS_BRIDGE_PROJECT: &str = "VeAwPNRN";

pub(crate) async fn prepare_local_skin_mods(
    state: &LauncherState,
    cluster: &Cluster,
    account_id: uuid::Uuid,
) {
    if !state
        .settings
        .read()
        .auto_install_local_skin_mods
    {
        return;
    }

    if !has_local_texture(account_id) {
        return;
    }

    if !cluster.mc_loader.is_modded() {
        return;
    }

    let ctx = state.services.content();
    let linked = match PackageStore::list_linked_artifacts(cluster.id, &ctx).await {
        Ok(linked) => linked,
        Err(err) => {
            tracing::warn!(
                cluster_id = cluster.id,
                error = %err,
                "could not inspect linked mods for local skins; continuing launch"
            );
            return;
        }
    };
    let installed: HashSet<String> = linked
        .into_iter()
        .filter(|artifact| artifact.provider == Some(ProviderId::Modrinth))
        .filter_map(|artifact| artifact.project_id)
        .collect();

    let loaders = install_loader_preference(cluster.mc_loader);
    let mut custom_skin_loader_ready = installed.contains(CUSTOM_SKIN_LOADER_PROJECT);
    let mut skin_layers_ready = installed.contains(SKIN_LAYERS_PROJECT);

    if !custom_skin_loader_ready {
        if let Err(err) = install_project(CUSTOM_SKIN_LOADER_PROJECT, cluster, &loaders, &ctx).await {
            tracing::warn!(cluster_id = cluster.id, error = %err, "could not install CustomSkinLoader; continuing launch");
        } else {
            custom_skin_loader_ready = true;
        }
    }

    if !skin_layers_ready {
        if let Err(err) = install_project(SKIN_LAYERS_PROJECT, cluster, &loaders, &ctx).await {
            tracing::warn!(cluster_id = cluster.id, error = %err, "could not install 3D Skin Layers; continuing launch");
        } else {
            skin_layers_ready = true;
        }
    }

    // Fabric/Quilt need a bridge for the two mods to coexist on modern versions.
    if matches!(cluster.mc_loader, GameLoader::Fabric | GameLoader::Quilt)
        && custom_skin_loader_ready
        && skin_layers_ready
        && !installed.contains(SKIN_LAYERS_BRIDGE_PROJECT)
        && let Err(err) = install_project(SKIN_LAYERS_BRIDGE_PROJECT, cluster, &loaders, &ctx).await
    {
        tracing::warn!(
            cluster_id = cluster.id,
            error = %err,
            "could not install SkinLayers3D-CustomSkinLoader-Bridge; continuing launch"
        );
    }
}

fn has_local_texture(uuid: uuid::Uuid) -> bool {
    paths::local_skin_file(&uuid.to_string()).is_ok_and(|path| path.is_file())
        || paths::local_cape_file(&uuid.to_string()).is_ok_and(|path| path.is_file())
}

fn install_loader_preference(loader: GameLoader) -> Vec<GameLoader> {
    match loader {
        GameLoader::Quilt => vec![GameLoader::Quilt, GameLoader::Fabric],
        GameLoader::Ornithe => vec![GameLoader::Ornithe, GameLoader::Fabric],
        other => vec![other],
    }
}

async fn install_project(
    project_id: &str,
    cluster: &Cluster,
    loaders: &[GameLoader],
    ctx: &ContentCtx,
) -> ContentResult<()> {
    let provider: &dyn PackageProvider = ctx.providers.get(ProviderId::Modrinth)?;
    let project: ProjectDetail = provider.get_project(project_id, ctx).await?;
    let mc_version = &cluster.mc_version;

    let mut version = None;
    for loader in loaders {
        let page = provider
            .list_versions(project_id, Some(mc_version), Some(*loader), 0, 1, ctx)
            .await?;
        if let Some(summary) = page.items.into_iter().next() {
            version = Some(provider.get_version(project_id, &summary.version_id, ctx).await?);
            break;
        }
    }

    let Some(version) = version else {
        return Ok(());
    };

    PackageStore::install_to_cluster(
        ProviderId::Modrinth,
        &project,
        &version,
        cluster.id,
        false,
        false,
        None,
        ctx,
    )
    .await?;

    Ok(())
}
