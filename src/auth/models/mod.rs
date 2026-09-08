use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};
use serde_with::skip_serializing_none;
use typed_builder_macro::TypedBuilder;

mod update_user;

pub use update_user::*;

#[derive(Debug, Deserialize)]
pub(crate) struct GetAccountInfoResponse {
    pub users: Option<Vec<User>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BatchGetResponse {
    pub users: Option<Vec<User>>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(rename = "localId")]
    pub uid: String,
    pub password_hash: Option<String>,
    pub password_updated_at: Option<u64>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub phone_number: Option<String>,
    pub display_name: Option<String>,
    pub photo_url: Option<String>,
    pub disabled: Option<bool>,
    pub salt: Option<String>,
    #[serde(
        default,
        rename = "customAttributes",
        deserialize_with = "deserialize_custom_attributes"
    )]
    pub custom_claims: serde_json::Value,
    pub valid_since: Option<String>,
    pub tenant_id: Option<String>,
    // pub provider_user_info: Option<Vec<ProviderUserInfo>>,
    // pub mfaInfo: Option<Vec<MultiFactorInfo>>,
    pub created_at: Option<String>,
    pub last_login_at: Option<String>,
    pub last_refresh_at: Option<String>,
    #[serde(flatten)]
    pub other: serde_json::Value,
}

fn deserialize_custom_attributes<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr + Default,
    T::Err: std::fmt::Display,
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    let t = s
        .map(|s| T::from_str(&s).map_err(serde::de::Error::custom))
        .transpose()?
        .unwrap_or_default();
    Ok(t)
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct NewUser {
    #[builder(default = None, setter(into))]
    pub display_name: Option<String>,
    pub email: String,
    pub password: String,
    #[builder(default = None, setter(into))]
    #[serde(rename = "localId")]
    pub username: Option<String>,
    #[builder(default = false)]
    pub email_verified: bool,
    #[builder(default = None, setter(into))]
    pub phone_number: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, TypedBuilder)]
pub struct AuthClaims {
    pub kind: String,
    pub registered: bool,
    #[serde(rename = "localId")]
    pub local_id: String,
    pub email: String,
    #[serde(rename = "idToken")]
    pub id_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(rename = "expiresIn", deserialize_with = "deserialize_from_str")]
    pub expires_in: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize, TypedBuilder)]
pub struct RefreshTokenClaims {
    pub access_token: String,
    pub id_token: String,
    pub refresh_token: String,
    pub token_type: String,
    #[serde(deserialize_with = "deserialize_from_str")]
    pub expires_in: u16,
    pub user_id: String,
    pub project_id: String,
}

fn deserialize_from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: std::fmt::Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct OobCode {
    pub email: String,
    pub oob_code: String,
    pub oob_link: String,
}
