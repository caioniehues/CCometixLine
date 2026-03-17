use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
struct OAuthCredentials {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "refreshToken")]
    refresh_token: Option<String>,
    #[serde(rename = "expiresAt")]
    expires_at: Option<u64>,
    scopes: Option<Vec<String>>,
    #[serde(rename = "subscriptionType")]
    subscription_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<OAuthCredentials>,
}

/// Resolve OAuth token trying sources in order:
/// 1. CLAUDE_CODE_OAUTH_TOKEN env var
/// 2. macOS Keychain (if `security` available)
/// 3. Linux credentials file (~/.claude/.credentials.json)
/// 4. GNOME Keyring via secret-tool (if available)
pub fn get_oauth_token() -> Option<String> {
    // 1. Explicit env var override
    if let Ok(token) = std::env::var("CLAUDE_CODE_OAUTH_TOKEN") {
        if !token.is_empty() {
            return Some(token);
        }
    }

    // 2. macOS Keychain
    if let Some(token) = get_token_from_keychain() {
        return Some(token);
    }

    // 3. Linux credentials file
    if let Some(token) = get_token_from_file() {
        return Some(token);
    }

    // 4. GNOME Keyring via secret-tool
    if let Some(token) = get_token_from_gnome_keyring() {
        return Some(token);
    }

    None
}

/// macOS Keychain via `security` command
fn get_token_from_keychain() -> Option<String> {
    use std::process::Command;

    let output = Command::new("security")
        .args([
            "find-generic-password",
            "-w",
            "-s",
            "Claude Code-credentials",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if json_str.is_empty() {
        return None;
    }

    let creds: CredentialsFile = serde_json::from_str(&json_str).ok()?;
    creds
        .claude_ai_oauth
        .map(|oauth| oauth.access_token)
        .filter(|t| !t.is_empty())
}

/// Credentials file: CLAUDE_CONFIG_DIR/.credentials.json or ~/.claude/.credentials.json
fn get_token_from_file() -> Option<String> {
    // Try CLAUDE_CONFIG_DIR first
    if let Ok(config_dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        let path = PathBuf::from(config_dir).join(".credentials.json");
        if let Some(token) = read_token_from_path(&path) {
            return Some(token);
        }
    }

    // Fall back to ~/.claude/.credentials.json
    let home = dirs::home_dir()?;
    let path = home.join(".claude").join(".credentials.json");
    read_token_from_path(&path)
}

/// GNOME Keyring via `secret-tool` command
fn get_token_from_gnome_keyring() -> Option<String> {
    use std::process::Command;

    let output = Command::new("secret-tool")
        .args(["lookup", "service", "Claude Code-credentials"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if json_str.is_empty() {
        return None;
    }

    let creds: CredentialsFile = serde_json::from_str(&json_str).ok()?;
    creds
        .claude_ai_oauth
        .map(|oauth| oauth.access_token)
        .filter(|t| !t.is_empty())
}

/// Read OAuth token from a credentials file path
fn read_token_from_path(path: &PathBuf) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let creds: CredentialsFile = serde_json::from_str(&content).ok()?;
    creds
        .claude_ai_oauth
        .map(|oauth| oauth.access_token)
        .filter(|t| !t.is_empty())
}
