use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::api::{Credential, OAuthToken};
use crate::tui::platform::Platform;

const CLIENT_ID: &str = "bc22accf2f8a13d57c8096ce8f332926";
const REDIRECT_PORT: u16 = 8787;
const REDIRECT_URI: &str = "http://localhost:8787/callback";
const SCOPES: &str = "read,write,issues:create,comments:create";
const AUTHORIZE_ENDPOINT: &str = "https://linear.app/oauth/authorize";
const TOKEN_ENDPOINT: &str = "https://api.linear.app/oauth/token";
const TIMEOUT: Duration = Duration::from_secs(300);
const REFRESH_TIMEOUT: Duration = Duration::from_secs(30);

const RESPONSE: &str = concat!(
    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n",
    "<html><body style=\"font-family: sans-serif\">",
    "<h2>Signed in</h2><p>You can close this tab and return to the terminal.</p>",
    "</body></html>",
);

pub async fn login(platform: Platform) -> Result<Credential> {
    let listener = TcpListener::bind(("127.0.0.1", REDIRECT_PORT))
        .await
        .with_context(|| {
            format!("could not start the local login server on port {REDIRECT_PORT}")
        })?;

    let verifier = random_token(32);
    let state = random_token(16);
    let url = authorize_url(&code_challenge(&verifier), &state)?;
    let opener = url.clone();

    tokio::task::spawn_blocking(move || platform.open_url(&opener))
        .await
        .context("failed to open the browser")??;

    let code = tokio::time::timeout(TIMEOUT, await_code(&listener, &state))
        .await
        .context("timed out waiting for the browser sign-in")??;

    let token = exchange(&code, &verifier).await?;

    Ok(Credential::OAuth(token))
}

pub async fn refresh(refresh_token: &str) -> Result<OAuthToken> {
    let request = async {
        let body = form_body(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", CLIENT_ID),
        ])?;

        let response = reqwest::Client::new()
            .post(TOKEN_ENDPOINT)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            let detail = response.text().await.unwrap_or_default();
            return Err(anyhow!("token refresh failed: {detail}"));
        }

        Ok(response
            .json::<TokenResponse>()
            .await?
            .into_token(refresh_token))
    };

    tokio::time::timeout(REFRESH_TIMEOUT, request)
        .await
        .context("timed out refreshing the session")?
}

fn authorize_url(challenge: &str, state: &str) -> Result<String> {
    let url = reqwest::Url::parse_with_params(
        AUTHORIZE_ENDPOINT,
        &[
            ("response_type", "code"),
            ("client_id", CLIENT_ID),
            ("redirect_uri", REDIRECT_URI),
            ("scope", SCOPES),
            ("state", state),
            ("code_challenge", challenge),
            ("code_challenge_method", "S256"),
        ],
    )?;

    Ok(url.to_string())
}

async fn await_code(listener: &TcpListener, expected_state: &str) -> Result<String> {
    loop {
        let (mut stream, _) = listener.accept().await?;
        let target = read_request_target(&mut stream).await?;
        let _ = stream.write_all(RESPONSE.as_bytes()).await;
        let _ = stream.shutdown().await;

        if !target.starts_with("/callback") {
            continue;
        }

        let url = reqwest::Url::parse(&format!("http://localhost{target}"))?;
        let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

        if let Some(error) = params.get("error") {
            return Err(anyhow!("authorisation was denied: {error}"));
        }
        if params.get("state").map(String::as_str) != Some(expected_state) {
            return Err(anyhow!("login state did not match"));
        }

        return params
            .get("code")
            .cloned()
            .ok_or_else(|| anyhow!("no authorisation code was returned"));
    }
}

async fn read_request_target(stream: &mut TcpStream) -> Result<String> {
    let mut buffer = [0u8; 4096];
    let read = stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..read]);

    request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .map(str::to_string)
        .ok_or_else(|| anyhow!("malformed callback request"))
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

impl TokenResponse {
    fn into_token(self, fallback_refresh: &str) -> OAuthToken {
        let expires_at = self.expires_in.map(|seconds| now_unix() + seconds);
        let refresh_token = self
            .refresh_token
            .or_else(|| Some(fallback_refresh.to_string()));

        OAuthToken::new(self.access_token, refresh_token, expires_at)
    }
}

async fn exchange(code: &str, verifier: &str) -> Result<OAuthToken> {
    let body = form_body(&[
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
        ("client_id", CLIENT_ID),
        ("code_verifier", verifier),
    ])?;

    let response = reqwest::Client::new()
        .post(TOKEN_ENDPOINT)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await?;

    if !response.status().is_success() {
        let detail = response.text().await.unwrap_or_default();
        return Err(anyhow!("token exchange failed: {detail}"));
    }

    let response: TokenResponse = response.json().await?;
    let expires_at = response.expires_in.map(|seconds| now_unix() + seconds);

    Ok(OAuthToken::new(
        response.access_token,
        response.refresh_token,
        expires_at,
    ))
}

fn form_body(params: &[(&str, &str)]) -> Result<String> {
    Ok(
        reqwest::Url::parse_with_params("http://localhost/", params)?
            .query()
            .unwrap_or_default()
            .to_string(),
    )
}

fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn code_challenge(verifier: &str) -> String {
    base64url(Sha256::digest(verifier.as_bytes()).as_slice())
}

fn random_token(bytes: usize) -> String {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer).expect("operating system RNG");
    base64url(&buffer)
}

fn base64url(input: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);

    for chunk in input.chunks(3) {
        let bytes = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let packed = (u32::from(bytes[0]) << 16) | (u32::from(bytes[1]) << 8) | u32::from(bytes[2]);

        for index in 0..=chunk.len() {
            let sextet = (packed >> (18 - index * 6)) & 0b11_1111;
            out.push(ALPHABET[sextet as usize] as char);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_matches_rfc_examples() {
        assert_eq!(base64url(b""), "");
        assert_eq!(base64url(b"f"), "Zg");
        assert_eq!(base64url(b"fo"), "Zm8");
        assert_eq!(base64url(b"foo"), "Zm9v");
        assert_eq!(base64url(b"foob"), "Zm9vYg");
    }

    #[test]
    fn pkce_challenge_is_the_rfc_7636_vector() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            code_challenge(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }
}
