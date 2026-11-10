use chrono::{DateTime, Utc};
use freya::animation::{AnimNum, Function, OnFinish, use_animation};
use freya::prelude::*;
use freya::query::{MutationCapability, MutationStateData, UseMutation};
use oneclient_auth::{AccountKind, MinecraftAccount};
use uuid::Uuid;

use super::{section_header, settings_page};
use crate::components::{
    Avatar, Button, Icon, IconType, MicrosoftLogin, OverlayPopup, PlayerModel, TextInput,
    use_microsoft_login,
};
use crate::hooks::{
    AddElyByAccountKeys, AddOfflineAccountKeys, RefreshAccountKeys, RemoveAccountKeys,
    SetDefaultAccountKeys, try_accounts, try_default_account, use_accounts,
    use_add_ely_by_account, use_add_offline_account, use_current_account, use_refresh_account,
    use_remove_account, use_set_default_account,
};
use crate::hooks::use_dispatch;
use crate::i18n::t;
use crate::theme::{colors, rounded, text_size};
use crate::ui::border_all_color;

/// The shader scales the model off `res.y` alone so height makes the player bigger width only needs to clear the arms
const MODEL_WIDTH_PX: f32 = 160.;
const MODEL_HEIGHT_PX: f32 = 264.;

const AVATAR_SIZE_PX: f32 = 36.;

#[derive(PartialEq)]
pub struct SettingsAccounts;

impl Component for SettingsAccounts {
    fn render(&self) -> impl IntoElement {
        let accounts_query = use_accounts();
        let default_query = use_current_account();

        let msa = use_microsoft_login();
        let add_offline = use_add_offline_account();
        let add_ely = use_add_ely_by_account();
        let set_default = use_set_default_account();
        let remove = use_remove_account();
        let refresh = use_refresh_account();

        let mut username = use_state(String::new);
        let mut show_offline = use_state(|| false);
        let mut closing_offline = use_state(|| false);

        let mut ely_username = use_state(String::new);
        let local_skin_revision = use_state(|| 0u64);
        let mut ely_password = use_state(String::new);
        let mut ely_totp = use_state(String::new);
        let mut show_ely = use_state(|| false);
        let mut closing_ely = use_state(|| false);
        let mut ely_needs_totp = use_state(|| false);

        use_side_effect(move || {
            if !*closing_offline.read() {
                return;
            }
            match &*add_offline.read().state() {
                MutationStateData::Settled { res: Ok(_), .. } => {
                    closing_offline.set(false);
                    show_offline.set(false);
                    username.set(String::new());
                }
                MutationStateData::Settled { res: Err(_), .. } => {
                    closing_offline.set(false);
                }
                _ => {}
            }
        });

        use_side_effect(move || {
            if !*closing_ely.read() {
                return;
            }
            match &*add_ely.read().state() {
                MutationStateData::Settled { res: Ok(_), .. } => {
                    closing_ely.set(false);
                    show_ely.set(false);
                    ely_username.set(String::new());
                    ely_password.set(String::new());
                    ely_totp.set(String::new());
                    ely_needs_totp.set(false);
                }
                MutationStateData::Settled {
                    res: Err(err),
                    ..
                } => {
                    closing_ely.set(false);
                    if err.to_string().contains("two-factor") {
                        ely_needs_totp.set(true);
                    }
                }
                _ => {}
            }
        });

        let accounts = try_accounts(&accounts_query).unwrap_or_default();
        let default_account = try_default_account(&default_query);
        let default_id = default_account.as_ref().map(|a| a.id);

        let offline_name = username.read().trim().to_string();
        let offline_uuid = (!offline_name.is_empty())
            .then(|| oneclient_auth::offline_uuid(&offline_name).to_string());
        // Live validation reuses the same hint_line as post-submit errors.
        let offline_validation_error = (!offline_name.is_empty())
            .then(|| oneclient_auth::validate_offline_username(&offline_name))
            .and_then(|r| r.err().map(|e| e.to_string()));

        let offline_error = mutation_err_text(&add_offline);

        let on_confirm_offline = move |_| {
            let name = username.peek().trim().to_string();
            if name.is_empty() {
                return;
            }
            add_offline.mutate(AddOfflineAccountKeys { username: name });
            closing_offline.set(true);
        };

        let ely_error = mutation_err_text(&add_ely);
        let on_confirm_ely = move |_| {
            let name = ely_username.peek().trim().to_string();
            let password = ely_password.peek().clone();
            if name.is_empty() || password.is_empty() {
                return;
            }
            let totp = ely_totp.peek().trim().to_string();
            add_ely.mutate(AddElyByAccountKeys {
                username: name,
                password,
                totp: (!totp.is_empty()).then_some(totp),
            });
            closing_ely.set(true);
        };

        let mut rows: Vec<Element> = accounts
            .iter()
            .map(|account| {
                AccountRow {
                    id: account.id,
                    username: account.username.clone(),
                    kind: account.kind,
                    expires: account.expires,
                    signed_out: account.needs_sign_in(),
                    is_default: Some(account.id) == default_id,
                    set_default,
                    remove,
                    refresh,
                    sign_in: msa.clone(),
                    ely_username,
                    show_ely,
                    local_skin_revision,
                }
                .into_element()
            })
            .collect();
        if rows.is_empty() {
            rows.push(empty_state());
        }

        settings_page()
            .child(hero(
                default_account,
                *local_skin_revision.read(),
                msa.pending,
                msa.error.clone(),
                move |_| show_offline.set(true),
                move |_| show_ely.set(true),
                {
                    let msa = msa.clone();
                    move |_| msa.start()
                },
            ))
            .child(section_header(t(
                "oneclient.settings.accounts.section.your_accounts",
            )))
            .children(rows)
            .maybe_child(show_offline.read().then(|| {
                offline_dialog(
                    username,
                    offline_uuid,
                    offline_validation_error,
                    offline_error,
                    on_confirm_offline,
                    show_offline,
                )
            }))
            .maybe_child(show_ely.read().then(|| {
                ely_dialog(
                    ely_username,
                    ely_password,
                    ely_totp,
                    *ely_needs_totp.read(),
                    ely_error,
                    on_confirm_ely,
                    show_ely,
                )
            }))
            .maybe_child(msa.popup())
            .into_element()
    }
}

fn hero(
    account: Option<MinecraftAccount>,
    local_skin_revision: u64,
    microsoft_pending: bool,
    error: Option<String>,
    on_open_offline: impl FnMut(Event<PressEventData>) + 'static,
    on_open_ely: impl FnMut(Event<PressEventData>) + 'static,
    on_add_microsoft: impl FnMut(Event<PressEventData>) + 'static,
) -> impl IntoElement {
    let (name, subtitle) = match &account {
        Some(account) if account.needs_sign_in() => (
            account.username.clone(),
            t("oneclient.settings.accounts.hero.signed_out"),
        ),
        Some(account) => (account.username.clone(), kind_label(account.kind)),
        None => (
            t("oneclient.settings.accounts.hero.no_account").to_string(),
            t("oneclient.settings.accounts.hero.no_account_hint"),
        ),
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(16.)
        .padding(Gaps::new_all(16.))
        .corner_radius(rounded(12.))
        .background(colors::page_elevated())
        .child(model_frame(
            account.as_ref().map(|account| account.id),
            local_skin_revision,
        ))
        .child(
            rect()
                .vertical()
                .width(Size::flex(1.0))
                .spacing(16.)
                .child(
                    rect()
                        .vertical()
                        .width(Size::fill())
                        .spacing(2.)
                        .child(
                            label()
                                .text(t("oneclient.settings.accounts.hero.active_account"))
                                .font_size(text_size(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .color(colors::fg_secondary()),
                        )
                        .child(
                            label()
                                .text(name)
                                .font_size(text_size(24.))
                                .font_weight(FontWeight::SEMI_BOLD)
                                .max_lines(1)
                                .color(colors::fg_primary()),
                        )
                        .child(
                            label()
                                .text(subtitle)
                                .font_size(text_size(12.))
                                .color(colors::fg_secondary()),
                        ),
                )
                .child(
                    rect()
                        .vertical()
                        .width(Size::fill())
                        .spacing(8.)
                        .child(
                            // The two buttons overflow the ~245px beside the model at the 800px minimum so they wrap
                            rect()
                                .horizontal()
                                .width(Size::fill())
                                .content(Content::wrap_spacing(8.))
                                .spacing(8.)
                                .child(
                                    Button::new()
                                        .primary()
                                        .enabled(!microsoft_pending)
                                        .on_press(on_add_microsoft)
                                        .child(Icon::new(IconType::Globe01).size(16.))
                                        .text(if microsoft_pending {
                                            t("oneclient.settings.accounts.signing_in")
                                        } else {
                                            t("oneclient.settings.accounts.add_microsoft")
                                        }),
                                )
                                .child(
                                    Button::new()
                                        .secondary()
                                        .on_press(on_open_offline)
                                        .child(Icon::new(IconType::Plus).size(16.))
                                        .text(t("oneclient.settings.accounts.add_offline")),
                                )
                                .child(
                                    Button::new()
                                        .secondary()
                                        .on_press(on_open_ely)
                                        .child(Icon::new(IconType::Globe01).size(16.))
                                        .text(t("oneclient.settings.accounts.add_ely_by")),
                                ),
                        )
                        .map(error, |el, msg| {
                            el.child(hint_line(IconType::AlertTriangle, msg, colors::danger()))
                        }),
                ),
        )
        .into_element()
}

fn model_frame(id: Option<Uuid>, local_skin_revision: u64) -> impl IntoElement {
    rect()
        .vertical()
        .cross_align(Alignment::Center)
        .spacing(6.)
        .child(
            rect()
                .width(Size::px(MODEL_WIDTH_PX))
                .height(Size::px(MODEL_HEIGHT_PX))
                .center()
                .overflow(Overflow::Clip)
                .corner_radius(rounded(12.))
                .background(colors::component_bg())
                .border(border_all_color(1., colors::component_border()))
                .child(match id {
                    Some(id) => PlayerModel::new(id)
                        .yaw(-0.5)
                        .local_skin(local_skin_revision)
                        .cape(crate::hooks::use_player_cape(&id.to_string()))
                        .width(Size::fill())
                        .height(Size::fill())
                        .into_element(),
                    None => Icon::new(IconType::Users01)
                        .size(28.)
                        .color(colors::fg_secondary())
                        .into_element(),
                }),
        )
        .maybe_child(id.map(|_| {
            label()
                .text(t("oneclient.settings.accounts.drag_to_rotate"))
                .font_size(text_size(10.))
                .color(colors::fg_secondary())
                .into_element()
        }))
        .into_element()
}

fn mutation_err_text<M>(mutation: &UseMutation<M>) -> Option<String>
where
    M: MutationCapability,
    M::Err: std::fmt::Display,
{
    match &*mutation.read().state() {
        MutationStateData::Settled { res: Err(err), .. } => Some(err.to_string()),
        MutationStateData::Loading {
            res: Some(Err(err)),
        } => Some(err.to_string()),
        _ => None,
    }
}

pub(crate) fn offline_dialog(
    username: State<String>,
    uuid_preview: Option<String>,
    validation_error: Option<String>,
    error: Option<String>,
    on_confirm: impl FnMut(Event<PressEventData>) + 'static,
    mut show_offline: State<bool>,
) -> impl IntoElement {
    // Live validation wins over the post-submit mutation error.
    let shown_error = validation_error.or(error);
    OverlayPopup::new()
        .on_close(move |()| show_offline.set(false))
        .child(
            rect()
                .width(Size::window_percent(100.))
                .height(Size::window_percent(100.))
                .center()
                .child(
                    rect()
                        .vertical()
                        .width(Size::px(380.))
                        .max_width(Size::window_percent(90.))
                        .spacing(16.)
                        .padding(Gaps::new_all(20.))
                        .corner_radius(rounded(16.))
                        .background(colors::page_elevated())
                        .border(border_all_color(1., colors::component_border()))
                        .child(
                            label()
                                .text(t("oneclient.settings.accounts.offline.title"))
                                .font_size(text_size(18.))
                                .font_weight(FontWeight::SEMI_BOLD)
                                .color(colors::fg_primary()),
                        )
                        .child(
                            rect()
                                .vertical()
                                .width(Size::fill())
                                .spacing(6.)
                                .child(field_label(t(
                                    "oneclient.settings.accounts.offline.username",
                                )))
                                .child(TextInput::new(username).placeholder(t(
                                    "oneclient.settings.accounts.offline.username_placeholder",
                                ))),
                        )
                        .child(
                            rect()
                                .vertical()
                                .width(Size::fill())
                                .spacing(6.)
                                .child(field_label("UUID"))
                                .child(
                                    label()
                                        .text(uuid_preview.unwrap_or_else(|| "-".to_string()))
                                        .font_size(text_size(12.))
                                        .color(colors::fg_secondary()),
                                ),
                        )
                        .map(shown_error, |el, msg| {
                            el.child(hint_line(IconType::AlertTriangle, msg, colors::danger()))
                        })
                        .child(hint_line(
                            IconType::InfoCircle,
                            t("oneclient.settings.accounts.offline.note").to_string(),
                            colors::fg_secondary(),
                        ))
                        .child(
                            rect()
                                .horizontal()
                                .width(Size::fill())
                                .main_align(Alignment::End)
                                .spacing(8.)
                                .child(
                                    Button::new()
                                        .ghost()
                                        .on_press(move |_| show_offline.set(false))
                                        .text(t("oneclient.common.cancel")),
                                )
                                .child(
                                    Button::new()
                                        .primary()
                                        .on_press(on_confirm)
                                        .child(Icon::new(IconType::Plus).size(16.))
                                        .text(t("oneclient.settings.accounts.offline.confirm")),
                                ),
                        ),
                ),
        )
        .into_element()
}

pub(crate) fn ely_dialog(
    username: State<String>,
    password: State<String>,
    totp: State<String>,
    needs_totp: bool,
    error: Option<String>,
    on_confirm: impl FnMut(Event<PressEventData>) + 'static,
    mut show_ely: State<bool>,
) -> impl IntoElement {
    OverlayPopup::new()
        .on_close(move |()| show_ely.set(false))
        .child(
            rect()
                .width(Size::window_percent(100.))
                .height(Size::window_percent(100.))
                .center()
                .child(
                    rect()
                        .vertical()
                        .width(Size::px(380.))
                        .max_width(Size::window_percent(90.))
                        .spacing(16.)
                        .padding(Gaps::new_all(20.))
                        .corner_radius(rounded(16.))
                        .background(colors::page_elevated())
                        .border(border_all_color(1., colors::component_border()))
                        .child(
                            label()
                                .text(t("oneclient.settings.accounts.ely_by.title"))
                                .font_size(text_size(18.))
                                .font_weight(FontWeight::SEMI_BOLD)
                                .color(colors::fg_primary()),
                        )
                        .child(
                            rect()
                                .vertical()
                                .width(Size::fill())
                                .spacing(6.)
                                .child(field_label(t(
                                    "oneclient.settings.accounts.ely_by.username",
                                )))
                                .child(TextInput::new(username).placeholder(t(
                                    "oneclient.settings.accounts.ely_by.username_placeholder",
                                ))),
                        )
                        .child(
                            rect()
                                .vertical()
                                .width(Size::fill())
                                .spacing(6.)
                                .child(field_label(t(
                                    "oneclient.settings.accounts.ely_by.password",
                                )))
                                .child(
                                    TextInput::new(password)
                                        .mode(InputMode::new_password())
                                        .placeholder(t(
                                            "oneclient.settings.accounts.ely_by.password_placeholder",
                                        )),
                                ),
                        )
                        .maybe(needs_totp, |el| {
                            el.child(
                                rect()
                                    .vertical()
                                    .width(Size::fill())
                                    .spacing(6.)
                                    .child(field_label(t(
                                        "oneclient.settings.accounts.ely_by.totp",
                                    )))
                                    .child(TextInput::new(totp).placeholder("123456")),
                            )
                        })
                        .map(error, |el, msg| {
                            el.child(hint_line(IconType::AlertTriangle, msg, colors::danger()))
                        })
                        .child(hint_line(
                            IconType::InfoCircle,
                            t("oneclient.settings.accounts.ely_by.note").to_string(),
                            colors::fg_secondary(),
                        ))
                        .child(
                            rect()
                                .horizontal()
                                .width(Size::fill())
                                .main_align(Alignment::End)
                                .spacing(8.)
                                .child(
                                    Button::new()
                                        .ghost()
                                        .on_press(move |_| show_ely.set(false))
                                        .text(t("oneclient.common.cancel")),
                                )
                                .child(
                                    Button::new()
                                        .primary()
                                        .on_press(on_confirm)
                                        .child(Icon::new(IconType::Globe01).size(16.))
                                        .text(t("oneclient.settings.accounts.ely_by.confirm")),
                                ),
                        ),
                ),
        )
        .into_element()
}

fn field_label(text: &str) -> impl IntoElement {
    label()
        .text(text.to_string())
        .font_size(text_size(11.))
        .font_weight(FontWeight::MEDIUM)
        .color(colors::fg_secondary())
        .into_element()
}

/// Sits in whatever is left of the hero beside the model so the text must wrap
fn hint_line(icon: IconType, text: String, color: Color) -> impl IntoElement {
    rect()
        .horizontal()
        .width(Size::fill())
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(6.)
        .child(Icon::new(icon).size(13.).color(color))
        .child(
            label()
                .text(text)
                .font_size(text_size(12.))
                .width(Size::flex(1.0))
                .color(color),
        )
        .into_element()
}

const REFRESH_SPIN_TIME: u64 = 800;

const REFRESHING_OPACITY: f32 = 0.7;

/// Only the identity half fades the sign-in and remove buttons stay crisp
const SIGNED_OUT_OPACITY: f32 = 0.45;

struct AccountRow {
    id: Uuid,
    username: String,
    kind: AccountKind,
    expires: DateTime<Utc>,
    signed_out: bool,
    is_default: bool,
    set_default: crate::hooks::UseSetDefaultAccount,
    remove: crate::hooks::UseRemoveAccount,
    refresh: crate::hooks::UseRefreshAccount,
    sign_in: MicrosoftLogin,
    ely_username: State<String>,
    show_ely: State<bool>,
    local_skin_revision: State<u64>,
}

impl PartialEq for AccountRow {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.username == other.username
            && self.kind == other.kind
            && self.expires == other.expires
            && self.signed_out == other.signed_out
            && self.is_default == other.is_default
            && self.sign_in.pending == other.sign_in.pending
    }
}

impl Component for AccountRow {
    fn render_key(&self) -> DiffKey {
        DiffKey::from(&self.id)
    }

    fn render(&self) -> impl IntoElement {
        let id = self.id;
        let is_default = self.is_default;
        let set_default = self.set_default;
        let remove = self.remove;
        let refresh = self.refresh;
        let dispatch = use_dispatch();

        let is_microsoft = self.kind == AccountKind::Microsoft;
        let is_token_account = is_microsoft || self.kind == AccountKind::ElyBy;
        let signed_out = is_token_account && self.signed_out;
        // A signed-out entry has no token left to call expired
        let expired = is_token_account && !signed_out && self.expires <= Utc::now();
        let identity_opacity = if signed_out { SIGNED_OUT_OPACITY } else { 1. };
        let sign_in = self.sign_in.clone();

        let mut refreshing = use_state(|| false);
        let local_skin_revision = self.local_skin_revision;
        let is_refreshing = *refreshing.read();

        let spin = use_animation(|conf| {
            conf.on_finish(OnFinish::restart());
            AnimNum::new(0., 360.)
                .time(REFRESH_SPIN_TIME)
                .function(Function::Linear)
        });

        use_side_effect_with_deps(&is_refreshing, move |&is_refreshing| {
            let mut spin = spin;
            if is_refreshing {
                spin.start();
            } else {
                spin.reset();
            }
        });

        let rotation = if is_refreshing {
            spin.get().value()
        } else {
            0.
        };

        let skin_dispatch = dispatch.clone();
        let set_local_skin = move |_| {
            let id = id;
            let mut local_skin_revision = local_skin_revision;
            let dispatch = skin_dispatch.clone();
            spawn(async move {
                let handle = match rfd::AsyncFileDialog::new()
                    .set_title(t("oneclient.settings.accounts.local_skin.pick_title"))
                    .add_filter("PNG image", &["png"])
                    .pick_file()
                    .await
                {
                    Some(handle) => handle,
                    None => return,
                };

                let result = async {
                    let bytes = tokio::fs::read(handle.path())
                        .await
                        .map_err(|err| err.to_string())?;
                    crate::hooks::validate_local_skin_png(&bytes)
                        .map_err(|err| err.to_string())?;
                    let path = oneclient_common::paths::local_skin_file(&id.to_string())
                        .map_err(|err| err.to_string())?;
                    if let Some(parent) = path.parent() {
                        polyio::create_dir_all(parent)
                            .await
                            .map_err(|err| err.to_string())?;
                    }
                    tokio::fs::write(&path, bytes)
                        .await
                        .map_err(|err| err.to_string())?;
                    Ok::<_, String>(())
                }
                .await;

                match result {
                    Ok(()) => {
                        let next = *local_skin_revision.peek() + 1;
                        local_skin_revision.set(next);
                        dispatch
                            .notify(t("oneclient.settings.accounts.local_skin.saved"))
                            .body(t("oneclient.settings.accounts.local_skin.saved_body").to_string())
                            .send();
                    }
                    Err(err) => {
                        dispatch
                            .notify(t("oneclient.settings.accounts.local_skin.failed"))
                            .body(err)
                            .error()
                            .send();
                    }
                }
            });
        };

        let cape_dispatch = dispatch.clone();
        let set_local_cape = move |_| {
            let id = id;
            let mut local_skin_revision = local_skin_revision;
            let dispatch = cape_dispatch.clone();
            spawn(async move {
                let handle = match rfd::AsyncFileDialog::new()
                    .set_title(t("oneclient.settings.accounts.local_cape.pick_title"))
                    .add_filter("PNG image", &["png"])
                    .pick_file()
                    .await
                {
                    Some(handle) => handle,
                    None => return,
                };

                let result = async {
                    let bytes = tokio::fs::read(handle.path())
                        .await
                        .map_err(|err| err.to_string())?;
                    crate::hooks::validate_local_cape_png(&bytes)
                        .map_err(|err| err.to_string())?;
                    let path = oneclient_common::paths::local_cape_file(&id.to_string())
                        .map_err(|err| err.to_string())?;
                    if let Some(parent) = path.parent() {
                        polyio::create_dir_all(parent)
                            .await
                            .map_err(|err| err.to_string())?;
                    }
                    tokio::fs::write(&path, bytes)
                        .await
                        .map_err(|err| err.to_string())?;
                    Ok::<_, String>(())
                }
                .await;

                match result {
                    Ok(()) => {
                        let next = *local_skin_revision.peek() + 1;
                        local_skin_revision.set(next);
                        dispatch
                            .notify(t("oneclient.settings.accounts.local_cape.saved"))
                            .body(t("oneclient.settings.accounts.local_cape.saved_body").to_string())
                            .send();
                    }
                    Err(err) => {
                        dispatch
                            .notify(t("oneclient.settings.accounts.local_cape.failed"))
                            .body(err)
                            .error()
                            .send();
                    }
                }
            });
        };

        let border_color = if signed_out {
            colors::component_border()
        } else if expired {
            colors::danger()
        } else if is_default {
            colors::brand()
        } else {
            colors::component_border()
        };

        rect()
            .horizontal()
            .width(Size::fill())
            .cross_align(Alignment::Center)
            .content(Content::Flex)
            .spacing(12.)
            .padding(Gaps::new_all(12.))
            .corner_radius(rounded(12.))
            .background(colors::page_elevated())
            .border(border_all_color(1., border_color))
            .opacity(if is_refreshing {
                REFRESHING_OPACITY
            } else {
                1.
            })
            .a11y_role(AccessibilityRole::Button)
            .maybe(!is_default && !signed_out, |el| {
                el.on_press(move |_| set_default.mutate(SetDefaultAccountKeys { id: Some(id) }))
            })
            .child(
                rect().opacity(identity_opacity).child(
                    Avatar::new(id.to_string())
                        .local_skin(*local_skin_revision.read())
                        .width(Size::px(AVATAR_SIZE_PX))
                        .height(Size::px(AVATAR_SIZE_PX)),
                ),
            )
            .child(
                rect()
                    .vertical()
                    .width(Size::flex(1.0))
                    .spacing(4.)
                    .opacity(identity_opacity)
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(6.)
                            .child(
                                label()
                                    .text(self.username.clone())
                                    .font_size(text_size(15.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .max_lines(1)
                                    .color(colors::fg_primary()),
                            )
                            .maybe_child(is_default.then(default_badge))
                            .maybe_child(expired.then(expired_badge))
                            .maybe_child(signed_out.then(signed_out_badge)),
                    )
                    // The kind rides with the id a third badge and a full uuid do not both fit
                    .child(
                        label()
                            .text(if signed_out {
                                t("oneclient.settings.accounts.session_expired").to_string()
                            } else {
                                format!("{} · {id}", kind_label(self.kind))
                            })
                            .font_size(text_size(11.))
                            .max_lines(1)
                            .color(colors::fg_secondary()),
                    ),
            )
            .maybe_child(signed_out.then(|| {
                let pending = sign_in.pending;
                let is_ely = self.kind == AccountKind::ElyBy;
                let mut ely_username = self.ely_username;
                let mut show_ely = self.show_ely;
                let username = self.username.clone();
                Button::new()
                    .secondary()
                    .enabled(!pending)
                    .tooltip(t("oneclient.settings.accounts.sign_in_again"))
                    .on_press(move |e: Event<PressEventData>| {
                        e.stop_propagation();
                        if is_ely {
                            ely_username.set(username.clone());
                            show_ely.set(true);
                        } else {
                            sign_in.start();
                        }
                    })
                    .child(Icon::new(IconType::Globe01).size(16.))
                    .text(if pending {
                        t("oneclient.settings.accounts.signing_in")
                    } else {
                        t("oneclient.settings.accounts.sign_in")
                    })
                    .into_element()
            }))
            .maybe_child((is_token_account && !signed_out).then(|| {
                Button::new()
                    .ghost()
                    .icon()
                    .tooltip(t("oneclient.settings.accounts.refresh"))
                    .on_press(move |e: Event<PressEventData>| {
                        e.stop_propagation();
                        if *refreshing.peek() {
                            return;
                        }
                        refreshing.set(true);
                        spawn(async move {
                            refresh.mutate_async(RefreshAccountKeys { id }).await;
                            refreshing.set(false);
                        });
                    })
                    .child(
                        rect().rotate(rotation).child(
                            Icon::new(IconType::RefreshCw01)
                                .size(18.)
                                .color(if expired {
                                    colors::danger()
                                } else {
                                    colors::fg_secondary()
                                }),
                        ),
                    )
                    .into_element()
            }))
            .maybe_child((self.kind == AccountKind::Offline).then(|| {
                Button::new()
                    .ghost()
                    .icon()
                    .tooltip(t("oneclient.settings.accounts.local_skin.tooltip"))
                    .on_press(move |e: Event<PressEventData>| {
                        e.stop_propagation();
                        set_local_skin(e);
                    })
                    .child(
                        Icon::new(IconType::PaintPour)
                            .size(18.)
                            .color(colors::fg_secondary()),
                    )
                    .into_element()
            }))
            .maybe_child((self.kind == AccountKind::Offline).then(|| {
                Button::new()
                    .ghost()
                    .icon()
                    .tooltip(t("oneclient.settings.accounts.local_cape.tooltip"))
                    .on_press(move |e: Event<PressEventData>| {
                        e.stop_propagation();
                        set_local_cape(e);
                    })
                    .child(
                        Icon::new(IconType::Square)
                            .size(18.)
                            .color(colors::fg_secondary()),
                    )
                    .into_element()
            }))
            .child(
                Button::new()
                    .ghost()
                    .icon()
                    .enabled(false)
                    .tooltip(t("oneclient.common.edit"))
                    .child(
                        Icon::new(IconType::Pencil01)
                            .size(18.)
                            .color(colors::fg_secondary()),
                    ),
            )
            .child(
                Button::new()
                    .ghost()
                    .icon()
                    .tooltip(t("oneclient.settings.accounts.remove"))
                    .on_press(move |e: Event<PressEventData>| {
                        e.stop_propagation();
                        remove.mutate(RemoveAccountKeys { id });
                    })
                    .child(
                        Icon::new(IconType::Trash01)
                            .size(18.)
                            .color(colors::fg_secondary()),
                    ),
            )
            .into_element()
    }
}

fn kind_label(kind: AccountKind) -> &'static str {
    match kind {
        AccountKind::Microsoft => "Microsoft",
        AccountKind::Offline => t("oneclient.settings.accounts.kind.offline"),
        AccountKind::ElyBy => "Ely.by",
    }
}

fn default_badge() -> impl IntoElement {
    badge(
        Icon::new(IconType::CheckCircle)
            .size(12.)
            .color(colors::brand())
            .into_element(),
        t("oneclient.common.default").to_string(),
        colors::brand(),
        colors::brand(),
    )
}

fn expired_badge() -> impl IntoElement {
    badge(
        Icon::new(IconType::AlertTriangle)
            .size(12.)
            .color(colors::danger())
            .into_element(),
        t("oneclient.settings.accounts.badge.expired").to_string(),
        colors::danger(),
        colors::danger(),
    )
}

fn signed_out_badge() -> impl IntoElement {
    badge(
        Icon::new(IconType::AlertTriangle)
            .size(12.)
            .color(colors::fg_secondary())
            .into_element(),
        t("oneclient.settings.accounts.badge.signed_out").to_string(),
        colors::component_border(),
        colors::fg_secondary(),
    )
}

fn badge(icon: impl IntoElement, text: String, border: Color, fg: Color) -> impl IntoElement {
    rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(4.)
        .padding(Gaps::new_symmetric(2., 8.))
        .corner_radius(rounded(999.))
        .border(border_all_color(1., border))
        .background(colors::component_bg())
        .child(icon)
        .child(
            label()
                .text(text)
                .font_size(text_size(10.))
                .font_weight(FontWeight::MEDIUM)
                .color(fg),
        )
        .into_element()
}

fn empty_state() -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .center()
        .padding(Gaps::new_all(32.))
        .spacing(8.)
        .corner_radius(rounded(12.))
        .background(colors::page_elevated())
        .child(
            Icon::new(IconType::Users01)
                .size(32.)
                .color(colors::fg_secondary()),
        )
        .child(
            label()
                .text(t("oneclient.settings.accounts.empty"))
                .font_size(text_size(14.))
                .color(colors::fg_secondary()),
        )
        .into_element()
}
