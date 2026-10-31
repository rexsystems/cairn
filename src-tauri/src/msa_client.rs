//! Which Microsoft OAuth client Cairn signs in with. Everything about the client ID lives here.
//!
//! Default: the official Minecraft launcher's Windows Live client, which needs no Azure app.
//! To use Cairn's own Azure app, set `CAIRN_MSA_CLIENT_ID` (runtime or build time) to its GUID.
//! That app needs the `https://login.microsoftonline.com/common/oauth2/nativeclient` redirect
//! and Mojang's approval for the Minecraft services API before sign-in works.

/// Client ID of the official Minecraft launcher on the Windows Live (login.live.com) endpoint.
pub const LIVE_DEFAULT_CLIENT_ID: &str = "00000000402b5328";

const LIVE_REDIRECT: &str = "https://login.live.com/oauth20_desktop.srf";
const MSAL_REDIRECT: &str = "https://login.microsoftonline.com/common/oauth2/nativeclient";
const LIVE_SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";
const MSAL_SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Clone, Debug, PartialEq)]
pub struct MsaClient {
    pub id: String,
    /// "live" (login.live.com) or "msal" (Azure AD v2).
    pub flow: &'static str,
    /// "environment", "build" or "default"; shown in Settings → About.
    pub source: &'static str,
}

impl MsaClient {
    pub fn resolve() -> Self {
        let runtime = std::env::var("CAIRN_MSA_CLIENT_ID").ok();
        Self::from_sources(runtime.as_deref(), option_env!("CAIRN_MSA_CLIENT_ID"))
    }

    fn from_sources(runtime: Option<&str>, build: Option<&str>) -> Self {
        let pick = |v: Option<&str>| v.map(str::trim).filter(|v| !v.is_empty()).map(String::from);
        let (id, source) = if let Some(id) = pick(runtime) {
            (id, "environment")
        } else if let Some(id) = pick(build) {
            (id, "build")
        } else {
            (LIVE_DEFAULT_CLIENT_ID.to_string(), "default")
        };
        MsaClient { flow: flow_for(&id), id, source }
    }

    /// Rebuild the client a saved account was created with, so refresh uses the same flow.
    pub fn for_account(id: &str, flow: &str) -> Self {
        let flow = if flow == "msal" { "msal" } else { flow_for(id) };
        MsaClient { id: id.to_string(), flow, source: "account" }
    }

    pub fn redirect(&self) -> &'static str {
        if self.flow == "live" { LIVE_REDIRECT } else { MSAL_REDIRECT }
    }

    pub fn scope(&self) -> &'static str {
        if self.flow == "live" { LIVE_SCOPE } else { MSAL_SCOPE }
    }

    pub fn token_url(&self) -> &'static str {
        token_url(self.flow)
    }

    pub fn device_code_url(&self) -> &'static str {
        if self.flow == "live" {
            "https://login.live.com/oauth20_connect.srf"
        } else {
            "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode"
        }
    }

    /// URL opened in the sign-in window.
    pub fn authorize_url(&self) -> String {
        let base = if self.flow == "live" {
            "https://login.live.com/oauth20_authorize.srf"
        } else {
            "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize"
        };
        format!(
            "{base}?client_id={}&response_type=code&redirect_uri={}&scope={}&prompt=select_account",
            urlencoding::encode(&self.id),
            urlencoding::encode(self.redirect()),
            urlencoding::encode(self.scope()),
        )
    }
}

/// Azure app registrations use GUID client IDs; Windows Live IDs are 16 hex digits.
pub fn flow_for(client_id: &str) -> &'static str {
    if client_id.len() == 36 && client_id.chars().filter(|c| *c == '-').count() == 4 { "msal" } else { "live" }
}

pub fn token_url(flow: &str) -> &'static str {
    if flow == "live" { "https://login.live.com/oauth20_token.srf" } else { "https://login.microsoftonline.com/consumers/oauth2/v2.0/token" }
}

pub fn scope_for(flow: &str) -> &'static str {
    if flow == "live" { LIVE_SCOPE } else { MSAL_SCOPE }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_the_live_client() {
        let c = MsaClient::from_sources(None, None);
        assert_eq!((c.id.as_str(), c.flow, c.source), (LIVE_DEFAULT_CLIENT_ID, "live", "default"));
        assert!(c.authorize_url().starts_with("https://login.live.com/oauth20_authorize.srf?client_id=00000000402b5328"));
    }

    #[test]
    fn runtime_beats_build_and_guid_uses_msal() {
        let guid = "11111111-2222-3333-4444-555555555555";
        let c = MsaClient::from_sources(Some(&format!(" {guid} ")), Some("0000000000000001"));
        assert_eq!((c.id.as_str(), c.flow, c.source), (guid, "msal", "environment"));
        assert!(c.authorize_url().contains("nativeclient"));
        let b = MsaClient::from_sources(Some("  "), Some(guid));
        assert_eq!(b.source, "build");
    }
}
