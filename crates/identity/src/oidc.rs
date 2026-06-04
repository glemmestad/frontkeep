//! OIDC authorization-code login. The authorization URL builder is exercised in
//! tests; live `exchange_code`/`userinfo` calls require a real IdP and are an
//! integration step (see BUILD_LOG / hand-back).

use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum OidcError {
    #[error("http: {0}")]
    Http(String),
    #[error("decode: {0}")]
    Decode(String),
}

#[derive(Debug, Clone)]
pub struct OidcConfig {
    pub authorize_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub id_token: Option<String>,
    #[serde(default)]
    pub token_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserInfo {
    pub sub: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub preferred_username: Option<String>,
}

impl OidcConfig {
    pub fn authorization_url(&self, state: &str, nonce: &str) -> String {
        format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&nonce={}",
            self.authorize_endpoint,
            enc(&self.client_id),
            enc(&self.redirect_uri),
            enc(&self.scopes.join(" ")),
            enc(state),
            enc(nonce),
        )
    }

    pub async fn exchange_code(&self, code: &str) -> Result<TokenResponse, OidcError> {
        let client = reqwest::Client::new();
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", self.redirect_uri.as_str()),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
        ];
        let resp = client
            .post(&self.token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| OidcError::Http(e.to_string()))?
            .error_for_status()
            .map_err(|e| OidcError::Http(e.to_string()))?;
        resp.json()
            .await
            .map_err(|e| OidcError::Decode(e.to_string()))
    }

    pub async fn userinfo(&self, access_token: &str) -> Result<UserInfo, OidcError> {
        let client = reqwest::Client::new();
        let resp = client
            .get(&self.userinfo_endpoint)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| OidcError::Http(e.to_string()))?
            .error_for_status()
            .map_err(|e| OidcError::Http(e.to_string()))?;
        resp.json()
            .await
            .map_err(|e| OidcError::Decode(e.to_string()))
    }
}

/// Minimal RFC3986 percent-encoding for query values.
fn enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> OidcConfig {
        OidcConfig {
            authorize_endpoint: "https://idp.example.com/authorize".into(),
            token_endpoint: "https://idp.example.com/token".into(),
            userinfo_endpoint: "https://idp.example.com/userinfo".into(),
            client_id: "asgard-app".into(),
            client_secret: "shh".into(),
            redirect_uri: "https://asgard.example.com/auth/callback".into(),
            scopes: vec!["openid".into(), "email".into(), "profile".into()],
        }
    }

    #[test]
    fn builds_authorization_url() {
        let url = cfg().authorization_url("st8", "nonce1");
        assert!(url.starts_with("https://idp.example.com/authorize?"));
        assert!(url.contains("client_id=asgard-app"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("state=st8"));
        assert!(url.contains("scope=openid%20email%20profile"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fasgard.example.com%2Fauth%2Fcallback"));
    }
}
