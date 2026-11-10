use freya::prelude::*;

use super::local_texture::{LocalTexture, pick_and_save, remove};
use super::{section_header, settings_page, settings_row};
use crate::components::{Button, Dropdown, Icon, IconType, PlayerModel, toggle};
use crate::hooks::{
    try_accounts, try_default_account, use_accounts, use_current_account, use_dispatch,
    use_settings_snapshot,
};
use crate::i18n::t;
use crate::routes::Route;
use crate::theme::{colors, rounded, text_size};
use crate::ui::{border_all_color, note};
use oneclient_auth::MinecraftAccount;
use uuid::Uuid;

const PREVIEW_WIDTH: f32 = 150.;
const PREVIEW_HEIGHT: f32 = 225.;

#[derive(PartialEq)]
pub struct SettingsSkins;

impl Component for SettingsSkins {
    fn render(&self) -> impl IntoElement {
        let accounts_query = use_accounts();
        let default_query = use_current_account();
        let settings = use_settings_snapshot().settings;
        let dispatch = use_dispatch();

        let selected = use_state(|| None::<Uuid>);
        let texture_revision = use_state(|| 0u64);
        let local_mods = use_state({
            let v = settings.auto_install_local_skin_mods;
            move || v
        });
        let mut first = use_state(|| true);

        let accounts = try_accounts(&accounts_query).unwrap_or_default();
        let default = try_default_account(&default_query).map(|account| account.id);

        let selected_id = selected
            .read()
            .and_then(|id| accounts.iter().find(|account| account.id == id).map(|account| account.id))
            .or_else(|| default.filter(|id| accounts.iter().any(|account| account.id == *id)))
            .or_else(|| accounts.first().map(|account| account.id));
        let account = selected_id.and_then(|id| accounts.iter().find(|account| account.id == id));

        let pointer = account.map(|account| account.id);
        let set_dispatch = dispatch.clone();
        let set_local = move |texture| {
            let Some(id) = pointer else {
                return;
            };
            let mut revision = texture_revision;
            let dispatch = set_dispatch.clone();
            spawn(async move {
                let result = match pick_and_save(texture, id).await {
                    Ok(true) => Ok(()),
                    Ok(false) => return,
                    Err(err) => Err(err),
                };
                match result {
                    Ok(()) => {
                        let next = *revision.peek() + 1;
                        revision.set(next);
                        let (title, body) = match texture {
                            LocalTexture::Skin => (
                                t("oneclient.settings.skins.skin_saved"),
                                t("oneclient.settings.skins.skin_saved_body"),
                            ),
                            LocalTexture::Cape => (
                                t("oneclient.settings.skins.cape_saved"),
                                t("oneclient.settings.skins.cape_saved_body"),
                            ),
                        };
                        dispatch.notify(title.to_string()).body(body.to_string()).send();
                    }
                    Err(err) => {
                        dispatch
                            .notify(t("oneclient.settings.skins.texture_failed"))
                            .body(err)
                            .error()
                            .send();
                    }
                }
            });
        };

        let skin_dispatch = dispatch.clone();
        let cape_dispatch = dispatch.clone();
        let reset_skin_dispatch = skin_dispatch.clone();
        let reset_cape_dispatch = cape_dispatch.clone();
        let reset = move |texture| {
            let Some(id) = pointer else {
                return;
            };
            let mut revision = texture_revision;
            let dispatch = match texture {
                LocalTexture::Skin => reset_skin_dispatch.clone(),
                LocalTexture::Cape => reset_cape_dispatch.clone(),
            };
            spawn(async move {
                match remove(texture, id).await {
                    Ok(()) => {
                        let next = *revision.peek() + 1;
                        revision.set(next);
                        dispatch
                            .notify(t("oneclient.settings.skins.texture_removed"))
                            .body(t("oneclient.settings.skins.texture_removed_body").to_string())
                            .send();
                    }
                    Err(err) => {
                        dispatch
                            .notify(t("oneclient.settings.skins.texture_failed"))
                            .body(err)
                            .error()
                            .send();
                    }
                }
            });
        };

        use_side_effect(move || {
            let local_mods = *local_mods.read();
            if *first.peek() {
                first.set(false);
                return;
            }
            dispatch.edit_settings(|settings| {
                settings.auto_install_local_skin_mods = local_mods;
            });
        });

        let mut content = settings_page()
            .child(section_header(t("oneclient.settings.skins.section.local")))
            .child(preview_card(
                account,
                texture_revision,
                set_local,
                reset,
            ))
            .child(account_selector_row(selected, accounts.clone()))
            .child(section_header(t("oneclient.settings.skins.section.in_game")))
            .child(settings_row(
                IconType::PaintPour,
                t("oneclient.settings.launcher.local_skin_mods.title"),
                t("oneclient.settings.launcher.local_skin_mods.description"),
                toggle(local_mods),
            ))
            .child(note(t("oneclient.settings.skins.note").to_string(), colors::brand()));

        if accounts.is_empty() {
            content = content.child(settings_row(
                IconType::Users01,
                t("oneclient.settings.skins.no_offline.title"),
                t("oneclient.settings.skins.no_offline.description"),
                Button::new()
                    .secondary()
                    .small()
                    .on_press(move |_| {
                        let _ = freya::router::RouterContext::get()
                            .push(Route::SettingsAccounts {});
                    })
                    .text(t("oneclient.settings.skins.no_offline.action")),
            ));
        }

        content.into_element()
    }
}

fn account_selector_row(
    mut selected: State<Option<Uuid>>,
    accounts: Vec<MinecraftAccount>,
) -> impl IntoElement {
    let options: Vec<String> = accounts
        .iter()
        .map(|account| account.username.clone())
        .collect();
    let labels = options.clone();
    let selected_idx = selected
        .read()
        .and_then(|id| accounts.iter().position(|account| account.id == id))
        .unwrap_or(0);
    let label = options.get(selected_idx).cloned().unwrap_or_else(|| {
        t("oneclient.settings.skins.no_offline.title").to_string()
    });

    settings_row(
        IconType::Users01,
        t("oneclient.settings.skins.account.title"),
        t("oneclient.settings.skins.account.description"),
        Dropdown::new(label, labels)
            .width(Size::px(220.))
            .height(Size::px(34.))
            .on_select(move |idx| {
                if let Some(account) = accounts.iter().nth(idx) {
                    selected.set(Some(account.id));
                }
            }),
    )
}

fn preview_card(
    account: Option<&MinecraftAccount>,
    texture_revision: State<u64>,
    on_set: impl FnMut(LocalTexture) + 'static + Clone,
    on_reset: impl FnMut(LocalTexture) + 'static + Clone,
) -> impl IntoElement {
    let mut on_set_skin = on_set.clone();
    let mut on_set_cape = on_set.clone();
    let mut on_reset_skin = on_reset.clone();
    let mut on_reset_cape = on_reset;

    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(16.)
        .padding(Gaps::new_all(16.))
        .corner_radius(rounded(12.))
        .background(colors::page_elevated())
        .border(border_all_color(1., colors::component_border()))
        .child(
            rect()
                .width(Size::px(PREVIEW_WIDTH))
                .height(Size::px(PREVIEW_HEIGHT))
                .center()
                .corner_radius(rounded(12.))
                .background(colors::component_bg())
                .overflow(Overflow::Clip)
                .child(match account {
                    Some(account) => PlayerModel::new(account.id.to_string())
                        .yaw(-0.5)
                        .local_skin(*texture_revision.read())
                        .cape(crate::hooks::use_player_cape(&account.id.to_string()))
                        .width(Size::fill())
                        .height(Size::fill())
                        .into_element(),
                    None => Icon::new(IconType::Users01)
                        .size(28.)
                        .into_element(),
                }),
        )
        .child(
            rect()
                .vertical()
                .width(Size::flex(1.0))
                .spacing(12.)
                .child(
                    label()
                        .text(t("oneclient.settings.skins.active_offline"))
                        .font_size(text_size(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .color(colors::fg_secondary()),
                )
                .child(
                    label()
                        .text(account.map(|account| account.username.clone()).unwrap_or_else(|| {
                            t("oneclient.settings.skins.no_offline.title").to_string()
                        }))
                        .font_size(text_size(22.))
                        .font_weight(FontWeight::SEMI_BOLD)
                        .max_lines(1)
                        .color(colors::fg_primary()),
                )
                .child(
                    rect()
                        .horizontal()
                        .width(Size::fill())
                        .content(Content::wrap_spacing(8.))
                        .spacing(8.)
                        .child(
                            Button::new()
                                .primary()
                                .small()
                                .on_press(move |_| on_set_skin(LocalTexture::Skin))
                                .child(Icon::new(IconType::PaintPour).size(16.))
                                .text(t("oneclient.settings.skins.set_skin")),
                        )
                        .child(
                            Button::new()
                                .secondary()
                                .small()
                                .on_press(move |_| on_set_cape(LocalTexture::Cape))
                                .child(Icon::new(IconType::Square).size(16.))
                                .text(t("oneclient.settings.skins.set_cape")),
                        ),
                )
                .child(
                    rect()
                        .horizontal()
                        .width(Size::fill())
                        .spacing(8.)
                        .child(
                            Button::new()
                                .ghost()
                                .small()
                                .on_press(move |_| on_reset_skin(LocalTexture::Skin))
                                .text(t("oneclient.settings.skins.clear_skin")),
                        )
                        .child(
                            Button::new()
                                .ghost()
                                .small()
                                .on_press(move |_| on_reset_cape(LocalTexture::Cape))
                                .text(t("oneclient.settings.skins.clear_cape")),
                        ),
                ),
        )
}
