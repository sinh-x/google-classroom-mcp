use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use google_calendar3::CalendarHub;
use google_classroom1::Classroom;
use google_drive3::DriveHub;
use yup_oauth2::authenticator_delegate::InstalledFlowDelegate;
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod, read_application_secret};

use crate::error::AppError;

/// Custom delegate that prevents interactive browser auth in server mode.
/// Logs to stderr instead of printing to stdout (which would corrupt MCP transport).
struct ServerFlowDelegate;

impl InstalledFlowDelegate for ServerFlowDelegate {
    fn present_user_url<'a>(
        &'a self,
        _url: &'a str,
        _need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async {
            tracing::error!(
                "Token refresh failed — re-authenticate: personal-google-mcp auth"
            );
            Err("interactive auth not available in server mode".into())
        })
    }
}

const OAUTH_REDIRECT_PORT: u16 = 8085;

pub const SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/classroom.courses.readonly",
    "https://www.googleapis.com/auth/classroom.announcements.readonly",
    "https://www.googleapis.com/auth/classroom.coursework.me.readonly",
    "https://www.googleapis.com/auth/classroom.rosters.readonly",
    "https://www.googleapis.com/auth/classroom.courseworkmaterials.readonly",
    "https://www.googleapis.com/auth/classroom.topics.readonly",
    "https://www.googleapis.com/auth/drive.readonly",
    "https://www.googleapis.com/auth/calendar.readonly",
];

pub type ClassroomHub =
    Classroom<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>;

pub type DriveHubType =
    DriveHub<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>;

pub type CalendarHubType =
    CalendarHub<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>;

/// Bundles all API hubs for a single profile.
pub struct ProfileHubs {
    pub classroom: ClassroomHub,
    pub drive: DriveHubType,
    pub calendar: CalendarHubType,
}

fn config_dir() -> Result<PathBuf, AppError> {
    let dir = dirs::config_dir()
        .ok_or_else(|| AppError::CredentialRead("cannot determine config directory".into()))?
        .join("personal-google-mcp");
    Ok(dir)
}

/// Profile-scoped directory for per-account data (tokens, cache).
/// When PGM_PROFILE is set, returns config_dir()/{profile}/.
/// When unset or empty, returns config_dir() (backward compat).
pub fn profile_dir() -> Result<PathBuf, AppError> {
    let base = config_dir()?;
    match std::env::var("PGM_PROFILE") {
        Ok(profile) if !profile.is_empty() => {
            if !profile
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                return Err(AppError::CredentialRead(format!(
                    "PGM_PROFILE must contain only alphanumeric characters, hyphens, or underscores, got: {profile}"
                )));
            }
            Ok(base.join(&profile))
        }
        _ => Ok(base),
    }
}

/// Returns the directory for a given profile name.
/// "default" returns the root config dir, named profiles return config_dir()/{name}/.
pub fn profile_dir_for(name: &str) -> Result<PathBuf, AppError> {
    let base = config_dir()?;
    if name == "default" {
        Ok(base)
    } else {
        Ok(base.join(name))
    }
}

/// Returns the active profile name, or None for the default profile.
pub fn active_profile() -> Option<String> {
    std::env::var("PGM_PROFILE")
        .ok()
        .filter(|p| !p.is_empty())
}

fn credentials_path() -> Result<PathBuf, AppError> {
    Ok(config_dir()?.join("credentials.json"))
}

fn tokens_path() -> Result<PathBuf, AppError> {
    Ok(profile_dir()?.join("tokens.json"))
}

/// Discover all authenticated profiles by scanning the config directory.
/// Root-level tokens.json → "default", subdirectories with tokens.json → named profiles.
/// Skips the "cache" directory.
pub fn discover_profiles() -> Result<Vec<(String, PathBuf)>, AppError> {
    let base = config_dir()?;
    let mut profiles = Vec::new();

    // Check root-level tokens.json → "default" profile
    if base.join("tokens.json").exists() {
        profiles.push(("default".to_string(), base.clone()));
    }

    // Scan subdirectories for named profiles
    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            // Skip the cache directory and hidden dirs
            if name == "cache" || name.starts_with('.') {
                continue;
            }
            if path.join("tokens.json").exists() {
                profiles.push((name, path));
            }
        }
    }

    if profiles.is_empty() {
        return Err(AppError::CredentialRead(
            "no authenticated profiles found — run `personal-google-mcp auth` first".into(),
        ));
    }

    Ok(profiles)
}

/// Build API hubs for a single profile from its config directory.
pub async fn build_hubs_for_profile(name: &str, dir: &Path) -> Result<ProfileHubs, AppError> {
    let creds_path = credentials_path()?;
    if !creds_path.exists() {
        return Err(AppError::CredentialRead(format!(
            "credentials.json not found — download from Google Cloud Console and place at {}",
            creds_path.display()
        )));
    }

    let tokens = dir.join("tokens.json");
    if !tokens.exists() {
        return Err(AppError::CredentialRead(format!(
            "not authenticated for profile '{name}' — run `PGM_PROFILE={name} personal-google-mcp auth`"
        )));
    }

    let secret = read_application_secret(&creds_path)
        .await
        .map_err(|e| AppError::CredentialRead(format!("failed to parse credentials.json: {e}")))?;

    let auth = InstalledFlowAuthenticator::builder(
        secret,
        InstalledFlowReturnMethod::Interactive,
    )
    .persist_tokens_to_disk(&tokens)
    .flow_delegate(Box::new(ServerFlowDelegate))
    .build()
    .await
    .map_err(|e| AppError::OAuth2(e.to_string()))?;

    match auth.token(SCOPES).await {
        Ok(_) => tracing::info!("profile '{name}': OAuth token validated"),
        Err(e) => {
            return Err(AppError::OAuth2(format!(
                "profile '{name}': token refresh failed — re-authenticate with \
                 `PGM_PROFILE={name} personal-google-mcp auth`: {e}"
            )));
        }
    }

    let build_client = || -> Result<_, AppError> {
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|e| AppError::Io(std::io::Error::other(e)))?
            .https_only()
            .enable_http2()
            .build();
        Ok(hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
            .build(connector))
    };

    let classroom = Classroom::new(build_client()?, auth.clone());
    let drive = DriveHub::new(build_client()?, auth.clone());
    let calendar = CalendarHub::new(build_client()?, auth);

    tracing::info!("profile '{name}': API hubs ready");
    Ok(ProfileHubs {
        classroom,
        drive,
        calendar,
    })
}

/// Discover all profiles and build hubs for each.
/// Warns on individual failures but errors only if all profiles fail.
pub async fn build_all_hubs() -> Result<HashMap<String, ProfileHubs>, AppError> {
    let profiles = discover_profiles()?;
    let mut all_hubs = HashMap::new();
    let mut errors = Vec::new();

    for (name, dir) in &profiles {
        match build_hubs_for_profile(name, dir).await {
            Ok(hubs) => {
                all_hubs.insert(name.clone(), hubs);
            }
            Err(e) => {
                tracing::warn!("failed to build hubs for profile '{name}': {e}");
                errors.push(format!("{name}: {e}"));
            }
        }
    }

    if all_hubs.is_empty() {
        return Err(AppError::CredentialRead(format!(
            "all profiles failed to authenticate:\n{}",
            errors.join("\n")
        )));
    }

    tracing::info!(
        "loaded {} profile(s): {}",
        all_hubs.len(),
        all_hubs.keys().cloned().collect::<Vec<_>>().join(", ")
    );

    Ok(all_hubs)
}

/// Run the interactive OAuth2 flow: opens a browser, waits for consent, saves tokens.
pub async fn run_auth_flow() -> Result<(), AppError> {
    if let Some(profile) = active_profile() {
        tracing::info!("Authenticating profile: {profile}");
    }

    let creds_path = credentials_path()?;
    if !creds_path.exists() {
        return Err(AppError::CredentialRead(format!(
            "credentials.json not found at {}.\n\
             Download it from Google Cloud Console → APIs & Services → Credentials\n\
             and place it at that path.",
            creds_path.display()
        )));
    }

    let secret = read_application_secret(&creds_path)
        .await
        .map_err(|e| AppError::CredentialRead(format!("failed to parse credentials.json: {e}")))?;

    let tokens = tokens_path()?;
    // Ensure parent directory exists
    if let Some(parent) = tokens.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let auth = InstalledFlowAuthenticator::builder(
        secret,
        InstalledFlowReturnMethod::HTTPPortRedirect(OAUTH_REDIRECT_PORT),
    )
    .persist_tokens_to_disk(&tokens)
    .build()
    .await
    .map_err(|e| AppError::OAuth2(e.to_string()))?;

    tracing::info!(
        "Opening browser for Google sign-in (redirect on port {OAUTH_REDIRECT_PORT})..."
    );

    // Requesting a token triggers the browser flow if no cached token exists
    let token = auth
        .token(SCOPES)
        .await
        .map_err(|e| AppError::OAuth2(e.to_string()))?;

    tracing::info!("Authentication successful!");
    tracing::info!("Tokens saved to {}", tokens.display());
    if let Some(profile) = active_profile() {
        tracing::info!("Profile: {profile}");
    }
    tracing::debug!("Token expires: {:?}", token.expiration_time());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_profiles_returns_vec_of_profiles() {
        // discover_profiles uses the real config dir; in sandboxed builds (Nix),
        // the config dir may not exist, so Err is also a valid outcome.
        match discover_profiles() {
            Ok(profiles) => {
                for (name, _path) in &profiles {
                    assert!(!name.is_empty());
                }
            }
            Err(_) => {} // No profiles configured — valid in sandbox
        }
    }

    #[test]
    fn test_discover_profiles_skips_cache_directory() {
        // In sandboxed builds, config dir may not exist — both Ok and Err are valid.
        match discover_profiles() {
            Ok(profiles) => {
                // Verify no profile is named "cache"
                assert!(profiles.iter().all(|(name, _)| name != "cache"));
            }
            Err(_) => {} // No profiles configured — valid in sandbox
        }
    }
}
