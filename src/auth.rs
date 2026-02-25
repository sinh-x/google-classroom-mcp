use std::path::PathBuf;

use google_classroom1::Classroom;
use google_drive3::DriveHub;
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod, read_application_secret};

use crate::error::AppError;

const OAUTH_REDIRECT_PORT: u16 = 8085;

pub const SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/classroom.courses.readonly",
    "https://www.googleapis.com/auth/classroom.announcements.readonly",
    "https://www.googleapis.com/auth/classroom.coursework.me.readonly",
    "https://www.googleapis.com/auth/classroom.rosters.readonly",
    "https://www.googleapis.com/auth/classroom.courseworkmaterials.readonly",
    "https://www.googleapis.com/auth/classroom.topics.readonly",
    "https://www.googleapis.com/auth/drive.readonly",
];

pub type ClassroomHub =
    Classroom<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>;

pub type DriveHubType =
    DriveHub<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>;

fn config_dir() -> Result<PathBuf, AppError> {
    let dir = dirs::config_dir()
        .ok_or_else(|| AppError::CredentialRead("cannot determine config directory".into()))?
        .join("personal-google-mcp");
    Ok(dir)
}

fn credentials_path() -> Result<PathBuf, AppError> {
    Ok(config_dir()?.join("credentials.json"))
}

fn tokens_path() -> Result<PathBuf, AppError> {
    Ok(config_dir()?.join("tokens.json"))
}

/// Run the interactive OAuth2 flow: opens a browser, waits for consent, saves tokens.
pub async fn run_auth_flow() -> Result<(), AppError> {
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
    tracing::debug!("Token expires: {:?}", token.expiration_time());

    Ok(())
}

/// Build Classroom and Drive API hubs from previously saved tokens.
pub async fn build_hubs() -> Result<(ClassroomHub, DriveHubType), AppError> {
    let creds_path = credentials_path()?;
    if !creds_path.exists() {
        return Err(AppError::NotAuthenticated);
    }

    let tokens = tokens_path()?;
    if !tokens.exists() {
        return Err(AppError::NotAuthenticated);
    }

    let secret = read_application_secret(&creds_path)
        .await
        .map_err(|e| AppError::CredentialRead(format!("failed to parse credentials.json: {e}")))?;

    let auth = InstalledFlowAuthenticator::builder(
        secret,
        InstalledFlowReturnMethod::HTTPPortRedirect(OAUTH_REDIRECT_PORT),
    )
    .persist_tokens_to_disk(&tokens)
    .build()
    .await
    .map_err(|e| AppError::OAuth2(e.to_string()))?;

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
    let drive_hub = DriveHub::new(build_client()?, auth);

    tracing::info!("Google API hubs ready");
    Ok((classroom_hub, drive_hub))
}
