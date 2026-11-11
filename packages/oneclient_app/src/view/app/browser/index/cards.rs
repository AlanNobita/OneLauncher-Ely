use super::*;

use std::collections::{HashMap, HashSet};

use freya::router::RouterContext;
use oneclient_content::packages::types::ProjectSummary;
use oneclient_content::packages::{ContentType, ProviderId};
use oneclient_core::{SeenStatus, clusters::ModpackSource};

use crate::components::{
    Button, Icon, IconType, PackageEntry, disable_request, guard_disable, run_guarded,
    use_shared_deletes,
};
use crate::hooks::{
    ALL_VERSIONS, ClusterAction, EssentialGuardKind, VERSIONS_PAGE_SIZE, content_type_for_slug,
    disable_warnings, mutation_is_running, use_browser_compat, use_cluster, use_cluster_mutation,
    use_disable_confirm, use_disable_warnings, use_dispatch, use_essential_guard,
    use_installs_snapshot, use_package_versions_when, version_list,
};
use crate::i18n::{t, tf};
use crate::routes::Route;
use crate::theme::{colors, rounded, text_size};
use crate::ui::border_all_color;
use crate::view::app::browser::cluster_version;
use crate::view::app::browser::{InstalledVersion, installed_badge};

type InstalledMap = HashMap<(ProviderId, String), Installed>;

/// Height of the install control, matched by the installed pill that replaces it
const INSTALL_BUTTON_H: f32 = 28.;

/// The card control mirrors the cluster list: a provenance pill, an enable or
/// disable toggle, and a remove button that works for bundle-provided or
/// hand-installed mods alike
type CardInstalled = Installed;

fn installed_for(installed: &InstalledMap, item: &ProjectSummary) -> Option<CardInstalled> {
    installed.get(&(item.provider, item.id.clone())).cloned()
}

pub(super) fn grid_row(
    row: Vec<ProjectSummary>,
    cluster_id: i64,
    package_type: &str,
    installed: &InstalledMap,
    cols: usize,
    add_to_cluster: bool,
) -> impl IntoElement {
    let package_type = package_type.to_string();
    let fill = cols.saturating_sub(row.len());
    let installed: Vec<Option<CardInstalled>> = row
        .iter()
        .map(|item| installed_for(installed, item))
        .collect();

    rect()
        .horizontal()
        .width(Size::fill())
        .height(Size::px(CARD_H))
        .spacing(GRID_SPACING)
        .content(Content::Flex)
        .children(
            row.into_iter()
                .zip(installed)
                .map(move |(item, installed)| {
                    PackageCard::new(
                        item,
                        cluster_id,
                        package_type.clone(),
                        installed,
                        add_to_cluster,
                    )
                    .into_element()
                }),
        )
        .maybe(fill > 0, move |mut el| {
            for _ in 0..fill {
                el = el.child(rect().width(Size::flex(1.0)));
            }
            el
        })
        .into_element()
}

fn open_package(
    cluster_id: i64,
    package_type: &str,
    provider: ProviderId,
    id: &str,
    add_to_cluster: bool,
) {
    let _ = RouterContext::get().push(Route::BrowserPackage {
        cluster_id,
        package_type: package_type.to_string(),
        package_id: encode_package_id(provider, id),
        add_to_cluster,
    });
}

#[derive(PartialEq)]
struct PackageCard {
    item: ProjectSummary,
    cluster_id: i64,
    package_type: String,
    installed: Option<CardInstalled>,
    add_to_cluster: bool,
}

impl PackageCard {
    fn new(
        item: ProjectSummary,
        cluster_id: i64,
        package_type: String,
        installed: Option<CardInstalled>,
        add_to_cluster: bool,
    ) -> Self {
        Self {
            item,
            cluster_id,
            package_type,
            installed,
            add_to_cluster,
        }
    }
}

impl Component for PackageCard {
    fn render(&self) -> impl IntoElement {
        let id = self.item.id.clone();
        let provider = self.item.provider;
        let package_type = self.package_type.clone();
        let icon_url = self.item.icon_url.clone();
        let banner = match self.item.banner_url.clone() {
            Some(url) => PackageBanner::new(Some(url), BANNER_H).sharp(),
            None => PackageBanner::new(icon_url.clone(), BANNER_H),
        };
        let cluster_id = self.cluster_id;
        let add_to_cluster = self.add_to_cluster;

        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let focused = focus().is_focused();

        rect()
            .vertical()
            .width(Size::flex(1.0))
            .height(Size::px(CARD_H))
            .corner_radius(rounded(8.))
            .background(colors::component_bg())
            .border(border_all_color(
                1.,
                if focused {
                    colors::brand()
                } else {
                    colors::component_border()
                },
            ))
            .overflow(Overflow::Clip)
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .cursor(CursorIcon::Pointer)
            .on_press(move |_| {
                open_package(cluster_id, &package_type, provider, &id, add_to_cluster)
            })
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(BANNER_H))
                    .overflow(Overflow::Clip)
                    // No installed badge, the control at the foot of the card already states it
                    .child(banner.backdrop_only()),
            )
            .child(
                rect()
                    .position(
                        Position::new_absolute()
                            .top(BANNER_H - CARD_ICON + CARD_ICON_OVERHANG)
                            .left(14.),
                    )
                    .layer(Layer::Relative(7))
                    .child(Thumbnail::new(icon_url, CARD_ICON).radius(6.)),
            )
            .child(
                rect()
                    .vertical()
                    .width(Size::fill())
                    .height(Size::flex(1.0))
                    .padding(Gaps::new(CARD_ICON_OVERHANG + 6., 14., 12., 14.))
                    .main_align(Alignment::SpaceBetween)
                    .child(
                        rect()
                            .vertical()
                            .width(Size::fill())
                            .spacing(6.)
                            .child(
                                label()
                                    .text(self.item.name.clone())
                                    .font_size(text_size(14.))
                                    .font_weight(FontWeight::SEMI_BOLD)
                                    .max_lines(1)
                                    .color(colors::fg_primary()),
                            )
                            .child(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(5.)
                                    .child(
                                        label()
                                            .text(tf(
                                                "oneclient.browser.index.card.author_downloads",
                                                &[
                                                    &self.item.author,
                                                    &abbreviate_number(self.item.downloads),
                                                ],
                                            ))
                                            .font_size(text_size(10.))
                                            .max_lines(1)
                                            .color(colors::fg_primary().with_a(140)),
                                    )
                                    .child(Icon::new(self.item.provider).size(11.)),
                            )
                            .child(
                                label()
                                    .text(self.item.summary.clone())
                                    .font_size(text_size(11.))
                                    .line_height(1.45)
                                    .max_lines(2)
                                    .width(Size::fill())
                                    .color(colors::fg_primary().with_a(184)),
                            ),
                    )
                    .child(InstallButton::new(
                        &self.item,
                        self.cluster_id,
                        &self.package_type,
                        self.installed.clone(),
                        self.add_to_cluster,
                    )),
            )
    }
}

pub(super) fn list_row(
    item: ProjectSummary,
    cluster_id: i64,
    package_type: &str,
    installed: &InstalledMap,
    add_to_cluster: bool,
) -> impl IntoElement {
    ListRow {
        installed: installed_for(installed, &item),
        item,
        cluster_id,
        package_type: package_type.to_string(),
        add_to_cluster,
    }
}

#[derive(PartialEq)]
struct ListRow {
    item: ProjectSummary,
    cluster_id: i64,
    package_type: String,
    installed: Option<CardInstalled>,
    add_to_cluster: bool,
}

impl Component for ListRow {
    fn render(&self) -> impl IntoElement {
        let item = self.item.clone();
        let cluster_id = self.cluster_id;
        let id = item.id.clone();
        let provider = item.provider;
        let package_type = self.package_type.clone();
        let add_to_cluster = self.add_to_cluster;

        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let focused = focus().is_focused();

        rect()
            .horizontal()
            .width(Size::fill())
            .height(Size::px(LIST_ROW_H))
            .cross_align(Alignment::Center)
            .spacing(12.)
            .padding(Gaps::new_all(16.))
            .corner_radius(rounded(8.))
            .background(colors::component_bg())
            .border(border_all_color(
                1.,
                if focused {
                    colors::brand()
                } else {
                    colors::component_border()
                },
            ))
            .overflow(Overflow::Clip)
            .content(Content::Flex)
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .cursor(CursorIcon::Pointer)
            .on_press(move |_| {
                open_package(cluster_id, &package_type, provider, &id, add_to_cluster)
            })
            .child(Thumbnail::new(item.icon_url.clone(), 48.).radius(6.))
            .child(
                rect()
                    .vertical()
                    .width(Size::flex(1.0))
                    .spacing(3.)
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(6.)
                            .child(
                                label()
                                    .text(item.name.clone())
                                    .font_size(text_size(15.))
                                    .font_weight(FontWeight::SEMI_BOLD)
                                    .max_lines(1)
                                    .color(colors::fg_primary()),
                            )
                            .child(
                                label()
                                    .text(tf(
                                        "oneclient.browser.index.card.author",
                                        &[&item.author],
                                    ))
                                    .font_size(text_size(10.))
                                    .max_lines(1)
                                    .color(colors::fg_primary().with_a(140)),
                            )
                            .child(Icon::new(item.provider).size(12.)),
                    )
                    .child(
                        label()
                            .text(item.summary.clone())
                            .font_size(text_size(11.))
                            .max_lines(2)
                            .width(Size::fill())
                            .color(colors::fg_primary().with_a(184)),
                    ),
            )
            .child(downloads_row(item.downloads))
            .child(InstallButton::new(
                &self.item,
                self.cluster_id,
                &self.package_type,
                self.installed.clone(),
                self.add_to_cluster,
            ))
    }
}

#[derive(PartialEq)]
struct InstallButton {
    provider: ProviderId,
    project_id: String,
    project_name: String,
    cluster_id: i64,
    content_type: ContentType,
    installed: Option<CardInstalled>,
    add_to_cluster: bool,
}

impl InstallButton {
    fn new(
        item: &ProjectSummary,
        cluster_id: i64,
        package_type: &str,
        installed: Option<CardInstalled>,
        add_to_cluster: bool,
    ) -> Self {
        let content_type = content_type_for_slug(package_type);
        Self {
            provider: item.provider,
            project_id: item.id.clone(),
            project_name: item.name.clone(),
            cluster_id,
            content_type,
            installed,
            add_to_cluster: add_to_cluster && content_type == ContentType::Modpack,
        }
    }
}

impl Component for InstallButton {
    fn render(&self) -> impl IntoElement {
        let cluster_id = self.cluster_id;
        let provider = self.provider;
        let project_id = self.project_id.clone();

        let dispatch = use_dispatch();
        let compat = *use_browser_compat().read();
        let cluster = use_cluster(cluster_id);

        // The same narrowing the package page does so both agree on what "latest" installs
        let is_modpack = self.content_type == ContentType::Modpack;
        let (game_version, loader) = match (compat && !is_modpack, &cluster) {
            (true, Some(c)) => (
                Some(c.mc_version.clone()),
                (self.content_type == ContentType::Mod).then_some(c.mc_loader),
            ),
            _ => (None, None),
        };

        let versions = version_list(&use_package_versions_when(
            self.installed.is_none(),
            provider,
            project_id.clone(),
            game_version,
            loader,
            0,
            if is_modpack {
                ALL_VERSIONS
            } else {
                VERSIONS_PAGE_SIZE
            },
        ));
        let add_to_cluster = self.add_to_cluster;
        let latest = if add_to_cluster {
            cluster.as_ref().and_then(|c| cluster_version(&versions, c))
        } else {
            preferred_version(&versions, self.content_type)
        }
        .map(|v| v.version_id.clone());
        let is_datapack = self.content_type == ContentType::DataPack;
        let mut world_prompt = use_state(|| None::<String>);
        let mut modpack_prompt = use_state(|| false);
        let mut choices = use_state(Vec::new);

        // Nothing to start twice while an install is running or before versions arrive
        let (installing, waiting) =
            use_installs_snapshot().package_busy(is_modpack, cluster_id, provider, &project_id);

        if let Some(installed) = &self.installed {
            return InstalledActions {
                cluster_id,
                provider,
                project_id: project_id.clone(),
                project_name: self.project_name.clone(),
                installed: installed.clone(),
                busy: installing || waiting,
            }
            .into_element();
        }

        let incompatible = add_to_cluster && latest.is_none() && !versions.is_empty();

        rect()
            // The card behind is one big press target stop here so the package page doesn't open on top of the install
            .on_press(|e: Event<PressEventData>| e.stop_propagation())
            .child(
                Button::new()
                    .primary()
                    .small()
                    .height(Size::px(INSTALL_BUTTON_H))
                    .padding(Gaps::new_symmetric(0., 11.))
                    .enabled(latest.is_some() && !installing && !waiting)
                    .on_press({
                        let project_id = project_id.clone();
                        move |_| {
                            let Some(version_id) = latest.clone() else {
                                return;
                            };
                            if add_to_cluster {
                                dispatch.import_modpack(
                                    cluster_id,
                                    ModpackSource::Provider {
                                        provider,
                                        project_id: project_id.clone(),
                                        version_id,
                                    },
                                );
                            } else if is_datapack {
                                world_prompt.set(Some(version_id));
                            } else if is_modpack {
                                let picks = minecraft_choices(&versions);
                                if picks.len() > 1 {
                                    choices.set(picks);
                                    modpack_prompt.set(true);
                                } else {
                                    dispatch.install_modpack(ModpackSource::Provider {
                                        provider,
                                        project_id: project_id.clone(),
                                        version_id,
                                    });
                                }
                            } else {
                                dispatch.install_package(
                                    cluster_id,
                                    provider,
                                    project_id.clone(),
                                    version_id,
                                    None,
                                );
                            }
                        }
                    })
                    .child(
                        Icon::new(if installing {
                            IconType::Loading02
                        } else {
                            IconType::Plus
                        })
                        .size(12.)
                        .color(colors::fg_primary()),
                    )
                    .child(
                        label()
                            .text(match (installing, add_to_cluster) {
                                (true, true) => t("oneclient.browser.index.card.adding"),
                                (true, false) => t("oneclient.browser.index.card.installing"),
                                (false, true) if incompatible => {
                                    t("oneclient.browser.index.card.incompatible")
                                }
                                (false, true) => t("oneclient.common.add"),
                                (false, false) => t("oneclient.common.install"),
                            })
                            .font_size(text_size(11.))
                            .font_weight(FontWeight::SEMI_BOLD)
                            .max_lines(1)
                            .color(colors::fg_primary()),
                    ),
            )
            .maybe_child(world_prompt.read().is_some().then_some(WorldInstallPrompt {
                cluster_id,
                provider,
                project_id: project_id.clone(),
                pending: world_prompt,
            }))
            .maybe_child(modpack_prompt.read().then(|| ModpackVersionPrompt {
                provider,
                project_id,
                choices: choices.read().clone(),
                open: modpack_prompt,
            }))
            .into_element()
    }
}

/// The cluster list runs every disable through the essential guard this
/// builds the same row shape the guarded paths read from. The artifact hash
/// and the bundle pin are what the action itself needs; the rest is filler.
fn synthetic_entry(
    provider: ProviderId,
    project_id: &str,
    project_name: &str,
    version: &InstalledVersion,
) -> PackageEntry {
    let bundle = version.bundle.as_ref();
    PackageEntry {
        package_id: bundle
            .map(|pin| pin.package_id.clone())
            .unwrap_or_else(|| project_id.to_string()),
        bundle_name: bundle.map(|pin| pin.bundle_name.clone()),
        provider,
        github_hosted: false,
        github_url: None,
        name: project_name.to_string(),
        file_name: version
            .display_version
            .clone()
            .unwrap_or_else(|| project_id.to_string()),
        author: String::new(),
        version: version.display_version.clone(),
        description: String::new(),
        icon_url: None,
        size: 0,
        categories: Vec::new(),
        enabled: version.enabled,
        installed: version.hash.is_some(),
        hash: version.hash.clone(),
        manifest_default: bundle.map(|pin| pin.manifest_default).unwrap_or(true),
        hidden: false,
        opted_in: true,
        advanced: false,
        deleted: false,
        update_available: false,
        copy_status: None,
        copies: Vec::new(),
        seen_status: SeenStatus::Seen,
        essential: crate::essential::lookup(provider, project_id),
        bisect_role: None,
        locked: false,
        modpack: None,
        added_at: None,
        published_at: None,
    }
}

/// Installed state on the browse cards, shown as a provenance pill plus the
/// action the user most likely wants: Enable or Disable, and Remove. Both
/// work on bundle-provided mods, which a plain remove could not touch.
#[derive(PartialEq)]
struct InstalledActions {
    cluster_id: i64,
    provider: ProviderId,
    project_id: String,
    project_name: String,
    installed: Installed,
    busy: bool,
}

impl Component for InstalledActions {
    fn render(&self) -> impl IntoElement {
        let cluster_id = self.cluster_id;
        let provider = self.provider;
        let project_id = self.project_id.clone();
        let project_name = self.project_name.clone();
        let busy = self.busy;
        let installed = &self.installed;

        let mutation = use_cluster_mutation();
        let running = mutation_is_running(&mutation);
        let guard = use_essential_guard();
        let pending_disable = use_disable_confirm();
        let dispatch = use_dispatch();
        let warnings_query = use_disable_warnings();
        let warnings = disable_warnings(&warnings_query);

        let entries: Vec<PackageEntry> = installed
            .versions
            .iter()
            .map(|version| synthetic_entry(provider, &project_id, &project_name, version))
            .collect();
        let hashes: Vec<String> = entries.iter().filter_map(|entry| entry.hash.clone()).collect();
        let any_enabled = entries.iter().any(|entry| entry.enabled);
        let off = busy || running;

        // Bundle-tracked hashes go through the bundle flow so a sync does
        // not put the removed mod straight back; manual mods are deleted
        let (on_remove, remove_dialog) = use_shared_deletes(cluster_id, {
            let entries = entries.clone();
            let project_name = project_name.clone();
            let warnings = warnings.clone();
            let guard = guard;
            move |hashes: Vec<String>| {
                let mutation = mutation.clone();
                let picked: HashSet<&String> = hashes.iter().collect();
                let targets: Vec<PackageEntry> = entries
                    .iter()
                    .filter(|entry| {
                        entry
                            .hash
                            .as_ref()
                            .is_some_and(|hash| picked.contains(hash))
                    })
                    .cloned()
                    .collect();
                let actions: Vec<ClusterAction> = targets
                    .iter()
                    .filter_map(|entry| {
                        entry.hash.as_ref().map(|hash| {
                            if entry.bundle_name.is_some() {
                                ClusterAction::DeleteBundledArtifact {
                                    cluster_id,
                                    hash: hash.clone(),
                                }
                            } else {
                                ClusterAction::RemoveArtifact {
                                    cluster_id,
                                    hash: hash.clone(),
                                }
                            }
                        })
                    })
                    .collect();
                if actions.is_empty() {
                    return;
                }
                let targets: Vec<&PackageEntry> = targets.iter().collect();
                run_guarded(
                    ClusterAction::Batch(actions),
                    &targets,
                    project_name.clone(),
                    EssentialGuardKind::Remove,
                    warnings.clone(),
                    mutation,
                    guard,
                );
            }
        });

        let disable = any_enabled.then(|| {
            let entries = entries.clone();
            let project_name = project_name.clone();
            let warnings = warnings.clone();
            let dispatch = dispatch.clone();
            Button::new()
                .secondary()
                .small()
                .height(Size::px(INSTALL_BUTTON_H))
                .padding(Gaps::new_symmetric(0., 11.))
                .enabled(!off)
                .on_press(move |_| {
                    let targets: Vec<PackageEntry> = entries
                        .iter()
                        .filter(|entry| entry.enabled)
                        .cloned()
                        .collect();
                    if targets.is_empty() {
                        return;
                    }
                    let refs: Vec<&PackageEntry> = targets.iter().collect();
                    guard_disable(
                        disable_request(&refs, cluster_id, project_name.clone()),
                        &refs,
                        warnings.clone(),
                        guard,
                        pending_disable,
                        dispatch.clone(),
                    );
                })
                .child(Icon::new(IconType::Minus).size(12.).color(colors::fg_primary()))
                .text(t("oneclient.browser.common.disable"))
                .into_element()
        });

        let enable = installed
            .disabled_bundled()
            .and_then(|version| version.enable_action(cluster_id))
            .map(|action| {
                EnableButton {
                    action,
                    variant: EnableVariant::Card {
                        height: INSTALL_BUTTON_H,
                    },
                }
                .into_element()
            });

        let remove = (!hashes.is_empty()).then(|| {
            Button::new()
                .ghost()
                .icon()
                .height(Size::px(INSTALL_BUTTON_H))
                .width(Size::px(INSTALL_BUTTON_H))
                .tooltip(t("oneclient.common.remove"))
                .enabled(!off)
                .on_press({
                    let project_name = project_name.clone();
                    let hashes = hashes.clone();
                    move |_| on_remove.call((project_name.clone(), hashes.clone()))
                })
                .child(Icon::new(IconType::Trash01).size(13.).color(colors::danger()))
                .into_element()
        });

        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(6.)
            .height(Size::px(INSTALL_BUTTON_H))
            .on_press(|e: Event<PressEventData>| e.stop_propagation())
            .child(installed_badge(installed.source, 10.))
            .maybe_child(enable)
            .maybe_child(disable)
            .maybe_child(remove)
            .maybe_child(remove_dialog)
            .into_element()
    }
}

fn downloads_row(downloads: u64) -> impl IntoElement {
    rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(4.)
        .child(
            Icon::new(IconType::Download01)
                .size(12.)
                .color(colors::fg_primary().with_a(140)),
        )
        .child(
            label()
                .text(abbreviate_number(downloads))
                .font_size(text_size(11.))
                .color(colors::fg_primary().with_a(140)),
        )
}

pub(super) fn empty_state() -> impl IntoElement {
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .center()
        .child(
            label()
                .text(t("oneclient.browser.index.no_results"))
                .font_size(text_size(14.))
                .color(colors::fg_secondary()),
        )
}
