use std::future::Future;
use std::path::PathBuf;
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

/// Build Classroom and Drive API hubs from previously saved tokens.
pub async fn build_hubs() -> Result<(ClassroomHub, DriveHubType, CalendarHubType), AppError> {
    if let Some(profile) = active_profile() {
        tracing::info!("Using profile: {profile}");
    }

    let profile_hint = match active_profile() {
        Some(p) => format!(" for profile '{p}' — run `PGM_PROFILE={p} personal-google-mcp auth`"),
        None => " — run `personal-google-mcp auth` first".into(),
    };

    let creds_path = credentials_path()?;
    if !creds_path.exists() {
        return Err(AppError::CredentialRead(format!(
            "not authenticated{profile_hint}"
        )));
    }

    let tokens = tokens_path()?;
    if !tokens.exists() {
        return Err(AppError::CredentialRead(format!(
            "not authenticated{profile_hint}"
        )));
    }

    let secret = read_application_secret(&creds_path)
        .await
        .map_err(|e| AppError::CredentialRead(format!("failed to parse credentials.json: {e}")))?;

    // Use Interactive mode (not HTTPPortRedirect) so that when the delegate
    // returns Err, the error propagates instead of blocking on wait_for_auth_code().
    // With HTTPPortRedirect, the delegate's return is discarded (`let _ =`) and
    // the server blocks forever waiting for a browser redirect that never comes.
    let auth = InstalledFlowAuthenticator::builder(
        secret,
        InstalledFlowReturnMethod::Interactive,
    )
    .persist_tokens_to_disk(&tokens)
    .flow_delegate(Box::new(ServerFlowDelegate))
    .build()
    .await
    .map_err(|e| AppError::OAuth2(e.to_string()))?;

    // Validate token at startup — catches expired/revoked tokens before any
    // MCP tool call. With an unverified Google app, refresh tokens expire after
    // 7 days, so this will fail fast with a clear re-auth message.
    match auth.token(SCOPES).await {
        Ok(_) => tracing::info!("OAuth token validated successfully"),
        Err(e) => {
            return Err(AppError::OAuth2(format!(
                "token refresh failed — re-authenticate with `personal-google-mcp auth`: {e}"
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

    let classroom_hub = Classroom::new(build_client()?, auth.clone());
    let drive_hub = DriveHub::new(build_client()?, auth.clone());
    let calendar_hub = CalendarHub::new(build_client()?, auth);

    tracing::info!("Google API hubs ready");
    Ok((classroom_hub, drive_hub, calendar_hub))
}
