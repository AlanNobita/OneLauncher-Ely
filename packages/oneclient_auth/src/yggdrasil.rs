//! Ely.by (Yggdrasil) authentication + authlib-injector provisioning.
//!
//! Ely.by speaks the Yggdrasil protocol at `https://authserver.ely.by`:
//! `POST /auth/authenticate`, `/auth/refresh`, `/auth/validate`.
//! In-game session/skin resolution is redirected to Ely.by by the
//! authlib-injector javaagent, which the launcher downloads (verified by
//! SHA-256) from `https://authlib-injector.yushi.moe/` and passes at launch
//! as `-javaagent:{jar}=https://authserver.ely.by` plus a prefetched
//! `-Dauthlibinjector.yggdrasil.prefetched={base64 metadata}`.
//!
//! Passwords are never stored — only accessToken + clientToken, like the
//! Microsoft refresh token.

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use chrono::{Duration, Utc};
use reqwest::Client;
use serde::Deserialize;
use uuid::Uuid;

use super::data::{AccountKind, MinecraftAccount};
use super::error::{AuthError, AuthResult};
use crate::error::MinecraftAuthError;

pub const ELY_API_URL: &str = "https://authserver.ely.by";
pub const ELY_METADATA_URL: &str = "https://authserver.ely.by/api/authlib-injector";
pub const AUTHLIB_DOWNLOAD_API: &str = "https://authlib-injector.yushi.moe";

#[derive(Debug, Clone, Deserialize)]
struct Profile {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "clientToken")]
    client_token: String,
    #[serde(rename = "selectedProfile")]
    selected_profile: Option<Profile>,
    #[serde(default, rename = "availableProfiles")]
    available_profiles: Vec<Profile>,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    #[serde(default, rename = "errorMessage")]
    error_message: String,
    #[serde(default)]
    error: String,
}

#[derive(Debug, Deserialize)]
struct LatestArtifact {
    download_url: String,
    checksums: Checksums,
}

#[derive(Debug, Deserialize)]
struct Checksums {
    sha256: String,
}

fn unhyphenated_uuid(raw: &str) -> Option<Uuid> {
    let clean: String = raw.chars().filter(|c| *c != '-').collect();
    Uuid::parse_str(&clean).ok()
}

fn account_from_response(
    resp: AuthResponse,
    fallback_username: &str,
) -> AuthResult<MinecraftAccount> {
    let profile = resp
        .selected_profile
        .clone()
        .or_else(|| resp.available_profiles.first().cloned());
    let profile = profile.ok_or_else(|| AuthError::ElyByFailed {
        message: "Ely.by returned no Minecraft profile for this account".into(),
    })?;
    let id = unhyphenated_uuid(&profile.id).ok_or_else(|| AuthError::ElyByFailed {
        message: format!("Ely.by returned an invalid profile UUID: {}", profile.id),
    })?;
    let _ = fallback_username;
    Ok(MinecraftAccount {
        id,
        username: profile.name,
        access_token: resp.access_token,
        refresh_token: String::new(),
        client_token: resp.client_token,
        // Ely.by issues long-lived JWT access tokens; validate/refresh on
        // launch repairs them, so a generous expiry avoids pointless churn.
        expires: Utc::now() + Duration::days(30),
        kind: AccountKind::ElyBy,
        signed_out: false,
    })
}

async fn parse_auth_response(
    resp: reqwest::Response,
) -> AuthResult<AuthResponse> {
    let status = resp.status();
    if status.is_success() {
        return resp.json::<AuthResponse>().await.map_err(|err| {
            AuthError::ElyByFailed {
                message: format!("invalid Ely.by response: {err}"),
            }
        });
    }
    let body: ErrorBody = resp.json().await.unwrap_or(ErrorBody {
        error: String::new(),
        error_message: String::new(),
    });
    if status.as_u16() == 401
        && (body.error_message.contains("two factor")
            || body.error.contains("TwoFactor"))
    {
        return Err(AuthError::ElyByTwoFactorRequired);
    }
    let message = if body.error_message.is_empty() {
        format!("Ely.by returned HTTP {}", status.as_u16())
    } else {
        body.error_message
    };
    Err(AuthError::ElyByFailed { message })
}

/// `totp` is appended to the password as `password:token` per Ely.by docs
/// when the account has two-factor authentication enabled.
pub async fn authenticate_ely_by(
    client: &Client,
    username: &str,
    password: &str,
    totp: Option<&str>,
) -> AuthResult<MinecraftAccount> {
    let password = match totp {
        Some(code) if !code.trim().is_empty() => format!("{password}:{}", code.trim()),
        _ => password.to_string(),
    };
    let client_token = Uuid::new_v4().simple().to_string();
    let resp = client
        .post(format!("{ELY_API_URL}/auth/authenticate"))
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "username": username,
            "password": password,
            "clientToken": client_token,
            "requestUser": true,
            "agent": { "name": "Minecraft", "version": 1 },
        }))
        .send()
        .await
        .map_err(|err| MinecraftAuthError::RequestError {
            step: crate::error::MinecraftAuthStep::MinecraftProfile,
            source: err,
        })?;
    let parsed = parse_auth_response(resp).await?;
    account_from_response(parsed, username)
}

pub async fn validate_ely_by(client: &Client, account: &MinecraftAccount) -> bool {
    if account.access_token.is_empty() || account.client_token.is_empty() {
        return false;
    }
    let Ok(resp) = client
        .post(format!("{ELY_API_URL}/auth/validate"))
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "accessToken": account.access_token,
            "clientToken": account.client_token,
        }))
        .send()
        .await
    else {
        return false;
    };
    resp.status().is_success()
}

pub async fn refresh_ely_by(
    client: &Client,
    account: &MinecraftAccount,
) -> AuthResult<MinecraftAccount> {
    let resp = client
        .post(format!("{ELY_API_URL}/auth/refresh"))
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "accessToken": account.access_token,
            "clientToken": account.client_token,
            "requestUser": true,
        }))
        .send()
        .await
        .map_err(|err| MinecraftAuthError::RequestError {
            step: crate::error::MinecraftAuthStep::RefreshToken,
            source: err,
        })?;
    let parsed = parse_auth_response(resp).await?;
    let mut refreshed = account_from_response(parsed, &account.username)?;
    refreshed.id = account.id;
    Ok(refreshed)
}

/// Base64-encoded Yggdrasil API metadata for `-Dauthlibinjector.yggdrasil.prefetched`.
pub async fn prefetched_metadata(client: &Client) -> AuthResult<String> {
    let resp = client
        .get(ELY_METADATA_URL)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|err| MinecraftAuthError::RequestError {
            step: crate::error::MinecraftAuthStep::MinecraftProfile,
            source: err,
        })?;
    if !resp.status().is_success() {
        return Err(AuthError::ElyByFailed {
            message: format!(
                "could not fetch Ely.by authlib metadata (HTTP {})",
                resp.status().as_u16()
            ),
        });
    }
    let bytes = resp.bytes().await.map_err(|err| {
        AuthError::ElyByFailed {
            message: format!("could not read Ely.by metadata: {err}"),
        }
    })?;
    Ok(BASE64_STANDARD.encode(&bytes))
}

/// Downloads (once) and verifies authlib-injector.jar into the launcher
/// data dir. Returns the jar path.
pub async fn ensure_authlib_injector(client: &Client) -> AuthResult<std::path::PathBuf> {
    let jar = oneclient_common::paths::authlib_injector_jar()?;
    if jar.is_file() {
        return Ok(jar);
    }
    if let Some(parent) = jar.parent() {
        polyio::create_dir_all(parent).await?;
    }
    let latest: LatestArtifact = client
        .get(format!("{AUTHLIB_DOWNLOAD_API}/artifact/latest.json"))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|err| MinecraftAuthError::RequestError {
            step: crate::error::MinecraftAuthStep::MinecraftProfile,
            source: err,
        })?
        .json()
        .await
        .map_err(|err| AuthError::ElyByFailed {
            message: format!("invalid authlib-injector release metadata: {err}"),
        })?;

    let bytes = client
        .get(&latest.download_url)
        .send()
        .await
        .map_err(|err| MinecraftAuthError::RequestError {
            step: crate::error::MinecraftAuthStep::MinecraftProfile,
            source: err,
        })?
        .bytes()
        .await
        .map_err(|err| AuthError::ElyByFailed {
            message: format!("could not download authlib-injector: {err}"),
        })?;

    use sha2::Digest;
    let digest = sha2::Sha256::digest(&bytes);
    let actual: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    if actual != latest.checksums.sha256.to_lowercase() {
        return Err(AuthError::ElyByFailed {
            message: "authlib-injector checksum mismatch — refusing to launch".into(),
        });
    }
    let tmp = jar.with_extension("jar.tmp");
    tokio::fs::write(&tmp, &bytes)
        .await
        .map_err(|err| AuthError::ElyByFailed {
            message: format!("could not write authlib-injector: {err}"),
        })?;
    tokio::fs::rename(&tmp, &jar)
        .await
        .map_err(|err| AuthError::ElyByFailed {
            message: format!("could not save authlib-injector: {err}"),
        })?;
    Ok(jar)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unhyphenated_profile_uuid_parses() {
        assert_eq!(
            unhyphenated_uuid("069a79f4-44e9-4726-a5be-fca90e38aaf5"),
            Uuid::parse_str("069a79f4-44e9-4726-a5be-fca90e38aaf5").ok()
        );
        assert_eq!(
            unhyphenated_uuid("069a79f444e94726a5befca90e38aaf5"),
            Uuid::parse_str("069a79f4-44e9-4726-a5be-fca90e38aaf5").ok()
        );
        assert!(unhyphenated_uuid("not-a-uuid").is_none());
    }

    #[test]
    fn account_from_response_uses_selected_profile() {
        let resp: AuthResponse = serde_json::from_value(serde_json::json!({
            "accessToken": "tok",
            "clientToken": "cli",
            "selectedProfile": {"id": "069a79f444e94726a5befca90e38aaf5", "name": "Notch"},
            "availableProfiles": [{"id": "069a79f444e94726a5befca90e38aaf5", "name": "Notch"}]
        }))
        .unwrap();
        let account = account_from_response(resp, "ignored").unwrap();
        assert_eq!(account.username, "Notch");
        assert!(account.is_ely_by());
        assert_eq!(account.client_token, "cli");
        assert!(!account.needs_sign_in());
    }
}
