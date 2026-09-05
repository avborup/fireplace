//! # Firebase Auth
//!
//! This module provides access to Firebase Authentication for user management and token operations.
//! For the complete API reference, see the [`FirebaseAuthClient`] documentation.
//!
//! ## Overview
//!
//! Firebase Auth enables you to:
//!
//! - **Manage users**: Create, retrieve, update, and list user accounts
//! - **Handle authentication**: Generate custom tokens and validate ID tokens
//! - **Control access**: Set custom claims for role-based access control
//!
//! ## Contents
//!
//! - [Initializing the client](#initializing-the-client)
//! - [User management](#user-management)
//!   * [Creating users](#creating-users)
//!   * [Retrieving users](#retrieving-users)
//!   * [Updating users](#updating-users)
//!   * [Listing all users](#listing-all-users)
//! - [Token operations](#token-operations)
//!   * [Creating custom tokens](#creating-custom-tokens)
//!   * [Verifying ID tokens](#verifying-id-tokens)
//! - [Custom claims](#custom-claims)
//! - [Username Password Login](#username-password-login)
//!
//! ## Initializing the client
//!
//! Create a [`FirebaseAuthClient`] using a Firebase service account:
//!
//! ```no_run
//! use fireplace::{ServiceAccount, auth::FirebaseAuthClient};
//!
//! let service_account = ServiceAccount::from_file("./service-account.json").unwrap();
//! let auth_client = FirebaseAuthClient::new(service_account).unwrap();
//! ```
//!
//! ## User management
//!
//! ### Creating users
//!
//! Create new users with email and password authentication:
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() -> Result<(), fireplace::error::FirebaseError> {
//! # let auth_client = fireplace::auth::test_helpers::initialise()?;
//! use fireplace::auth::models::NewUser;
//!
//!  let new_user = NewUser::builder()
//!     .display_name("John Doe".to_string())
//!     .username("john_doe".to_string())
//!     .email("user@example.com".to_string())
//!     .password("hello123".to_string())
//!     .email_verified(true)
//!     .build();
//!
//! let user_id = auth_client.create_user(new_user).await?;
//!
//! println!("Created user: {}", user_id);
//! # Ok(())
//! # }
//! ```
//!
//! See [`FirebaseAuthClient::create_user`] for more details.
//!
//! ### Retrieving users
//!
//! Fetch user information by their user ID:
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() -> Result<(), fireplace::error::FirebaseError> {
//! # let auth_client = fireplace::auth::test_helpers::initialise()?;
//! if let Some(user) = auth_client.get_user("user_id_here").await? {
//!     println!("User email: {:?}", user.email);
//!     println!("Display name: {:?}", user.display_name);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! See [`FirebaseAuthClient::get_user`] for more details.
//!
//! ### Updating users
//!
//! Update user attributes like email, display name, or password:
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() -> Result<(), fireplace::error::FirebaseError> {
//! # let auth_client = fireplace::auth::test_helpers::initialise()?;
//! use fireplace::auth::models::UpdateUserValues;
//!
//! auth_client.update_user(
//!     "user_id_here",
//!     UpdateUserValues::builder()
//!         .display_name(Some("Jane Doe".to_string()))
//!         .disabled(Some(false))
//!         .build()
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! See [`FirebaseAuthClient::update_user`] for more details.
//!
//! ### Listing all users
//!
//! Retrieve all users in your Firebase project:
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() -> Result<(), fireplace::error::FirebaseError> {
//! # let auth_client = fireplace::auth::test_helpers::initialise()?;
//! let all_users = auth_client.get_all_users().await?;
//! println!("Total users: {}", all_users.len());
//! # Ok(())
//! # }
//! ```
//!
//! For better performance with large user bases, use concurrent fetching:
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() -> Result<(), fireplace::error::FirebaseError> {
//! # let auth_client = fireplace::auth::test_helpers::initialise()?;
//! // Use 4 concurrent workers
//! let all_users = auth_client.get_all_users_concurrently(4).await?;
//! # Ok(())
//! # }
//! ```
//!
//! See [`FirebaseAuthClient::get_all_users`] and [`FirebaseAuthClient::get_all_users_concurrently`].
//!
//! ## Token operations
//!
//! ### Creating custom tokens
//!
//! Generate custom tokens that users can exchange for ID tokens:
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() -> Result<(), fireplace::error::FirebaseError> {
//! # let auth_client = fireplace::auth::test_helpers::initialise()?;
//! let custom_token = auth_client.create_custom_token("user_id_here").await?;
//! // Send this token to the client to sign in
//! # Ok(())
//! # }
//! ```
//!
//! See [`FirebaseAuthClient::create_custom_token`] for more details.
//!
//! ### Verifying ID tokens
//!
//! Validate ID tokens from authenticated users:
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() -> Result<(), fireplace::error::FirebaseError> {
//! # let auth_client = fireplace::auth::test_helpers::initialise()?;
//! let claims = auth_client
//!     .decode_id_token::<serde_json::Value>("id_token_here")
//!     .await?;
//!
//! let user_id = claims["user_id"].as_str().unwrap();
//! # Ok(())
//! # }
//! ```
//!
//! See [`FirebaseAuthClient::decode_id_token`] for more details.
//!
//! ## Custom claims
//!
//! Set custom attributes on users for role-based access control:
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() -> Result<(), fireplace::error::FirebaseError> {
//! # let auth_client = fireplace::auth::test_helpers::initialise()?;
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct CustomClaims {
//!     role: String,
//!     admin: bool,
//! }
//!
//! auth_client.set_custom_user_claims(
//!     "user_id_here",
//!     CustomClaims {
//!         role: "moderator".to_string(),
//!         admin: false,
//!     }
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! These claims will appear in the user's ID token after they re-authenticate.
//! See [`FirebaseAuthClient::set_custom_user_claims`] for more details.
//!
//! ## Username Password Login
//!
//!Use username and password credentials to authenticate user in order to retrive access and refresh tokens
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() -> Result<(), fireplace::error::FirebaseError> {
//! # let auth_client = fireplace::auth::test_helpers::initialise()?;
//!
//! let auth_claims = auth_client.login_with_password("joe","password", true).await?;
//! print!("User authenticated successfully : {:?}", auth_claims);
//! # Ok(())
//! # }
//! ```
//!
//! See [`FirebaseAuthClient::login_with_password`] for more details.

use std::collections::HashMap;

use self::{
    credential::{ApiAuthTokenManager, UserTokenManager},
    models::{GetAccountInfoResponse, NewUser, User},
};
use crate::auth::models::{AuthClaims, RefreshTokenClaims};
use crate::{
    ServiceAccount,
    auth::{
        error::AuthApiErrorResponse,
        models::{BatchGetResponse, UpdateUserBody, UpdateUserValues},
    },
    error::FirebaseError,
};
use anyhow::Context;
use chrono::Utc;
use reqwest::{Client, ClientBuilder, Response};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use url::Url;

mod credential;
mod error;
pub mod models;
pub mod test_helpers;

pub struct FirebaseAuthClient {
    client: Client,
    api_url: String,
    user_token_manager: UserTokenManager,
    api_auth_token_manager: ApiAuthTokenManager,
    project_id: String,
    emulated: bool,
}

impl FirebaseAuthClient {
    pub fn new(service_account: ServiceAccount) -> Result<Self, FirebaseError> {
        let client_builder = Self::http_client_builder(None)?;
        let client = client_builder
            .https_only(true)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self::create_firebase_auth_client(
            client,
            "https://identitytoolkit.googleapis.com/v1",
            service_account,
            false,
        ))
    }

    fn create_firebase_auth_client(
        client: Client,
        firebase_auth_api_url: &str,
        service_account: ServiceAccount,
        emulated: bool,
    ) -> Self {
        let credential_manager = ApiAuthTokenManager::new(service_account.clone());
        let project_id = service_account.project_id.clone();
        let token_handler = UserTokenManager::new(service_account, client.clone());

        Self {
            user_token_manager: token_handler,
            client,
            api_url: firebase_auth_api_url.to_string(),
            api_auth_token_manager: credential_manager,
            project_id,
            emulated,
        }
    }

    pub fn new_with_proxy(
        service_account: ServiceAccount,
        proxy_url: &str,
        accept_invalid_certificate: bool,
    ) -> Result<Self, FirebaseError> {
        let client_builder = Self::http_client_builder(Some(proxy_url))?;
        let client = client_builder
            .https_only(true)
            .danger_accept_invalid_certs(accept_invalid_certificate)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self::create_firebase_auth_client(
            client,
            "https://identitytoolkit.googleapis.com/v1",
            service_account,
            false,
        ))
    }

    const EMULATOR_DUMMY_KEY: &'static str = "-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC+0fXCo5tN9xdW
ri8CaEmHOp0ok3f4AGrz56fL0tZut0zK6NxFtsyuIe5zmvgAtUSRuQB5kSiC+m6D
EI+vE1PK+V5cKaoLdSGnR3e/TwOaag62z9LHVtl7pafnbR292m7v6J6lUftbME5O
ptJToI0rFjFxHRPhBe+DejqTF4Maa/Vjr8l4XWYUjKDnpgWhPB+cMWFoaS0Iv5eC
LZ6nARcmAqOv4q29qtoG/Fl2GAeTQNWXRn7YPz3P6bvjde7OIGJ+e3UDFSykZoJc
V8sX6jsvcfIDxLpDbB8JF+jjEJFP2tQ91Bhk7Jm4z4xvyz1LGzWMrTCPiWusU2j0
3VGMOLv1AgMBAAECggEAPrg6RCiMcnrl4+rKrsriaB1e7KhUBWL1xId/AqKhMrDy
/7q8M/namt2yR7NHEsyB5jfdrReGLk7sJg46e/wpTYzbdAWJy8xLLHV0KaBp+cYk
Yfg7OqQURez0HEAkR3PVhOt7wbEJojZldrqkZTJfVCsuQxSgpRTTXe698/0FbZg/
BIN0cC3KfbIXM+IWAZ02TWUgaqyYUKqLTyt5NhPYiKGxOH1pQiGzn+qR16QiFx5q
v2U9L4dIM1MjN9snqZdQPFsdQdZsx5ZG55ItDNA6Ld1zv39t83P1z/frIkbG6WGx
yKl7pDahi9BMXS7sRra3aIx6MiH6BA0y2PbReT0+3QKBgQDpYpSVqs3+cp7pj8gB
sKgCnU4+TANjlq0PM8PGpaLCA+8V1cwVYNpn92vOwHBa0BeYIcX5v3ZUOoBtuqa2
6p3KBADyDC+OnL2Wn0NZ3UAhh+vNTIb020aBk+rdr342yNqX15QwSn2slk6Ugb2T
aGj9oGtR8JtkRzAhd2XQHQ2zLwKBgQDRT4BfxHQ9vxX1Km4aJKo4Fa0gDBGD7l9C
A5ScYxevV1zq8Ie8oIUH1NqSdKXoQ5zUGEsj+yC9oNQCE3wLtLK3nogKX/hN0pJr
Am+NTM76Mg1L4xehB8QmDo7fzDDsfRULHGW4C/R5I3rq0NacKdj+2evRfkgTFsIa
OIems+kKGwKBgQCgG6ETjDoFdVzPnnP3tNmN8Zzb4AnzKEtRQpHslXUy5MAAmsuH
Xwp2iKbND/gEVP3awFCxJUhoQDobDRGSKyJUlKqRYwq9K4pk6p2p6L82QajjDr0G
/edAvdlCfEJ6ExDjWmGvP2s4G2WNO/RyTA668HKvpAD2ql+6lc6jF0tuwQKBgGFz
o3BzNi+9dQq1m5eedIYySW2/ULRdWoqQwjNhy5g+k6trG7eOAQIZbz57Ave16yGw
weNZu++uHqdszPdiRUdKj0pNTn1lSfh6pNdj6IYCocAwVkMKK6AQbSLznd+tiQuo
SA+9uTBhfYwweYsxzK8zZSEfq4z/rpWotje+UgT7AoGBAKDSIUjRSP1eSsbRRd0Y
92MvGJQ8SBP2LSNYsCmn9m4aHCKTEhJHmxMMSj2B6heG1k9Ih+LmyvddbcL5gxuJ
SRI5IBgi6eBij9lM8FaLjlE3tpdblx/Xij3/4/WDGu1c3cKAHNVbNktPmnZ2fE34
lPTlzALOoknxQtKOWgLsu7XF
-----END PRIVATE KEY-----
";

    pub fn emulator(emulator_url: &str, proxy_url: Option<&str>) -> Result<Self, FirebaseError> {
        let client_builder = Self::http_client_builder(proxy_url)?;
        let client = client_builder
            .https_only(false)
            .danger_accept_invalid_certs(true)
            .build()
            .context("Failed to create HTTP client for emulator")?;

        let dummy_service_account = ServiceAccount {
            project_id: "demo-firebase-project".to_string(),
            private_key: Self::EMULATOR_DUMMY_KEY.to_string(),
            private_key_id: "mock_private_key_id_123456789".to_string(),
            client_email: "firebase-adminsdk-mock@://gserviceaccount.com".to_string(),
            client_id: "://googleusercontent.com".to_string(),
        };

        let emulator_auth_url = format!("{emulator_url}/identitytoolkit.googleapis.com/v1");
        Ok(Self::create_firebase_auth_client(
            client,
            emulator_auth_url.as_str(),
            dummy_service_account,
            true,
        ))
    }

    fn http_client_builder(proxy_uri: Option<&str>) -> Result<ClientBuilder, FirebaseError> {
        let client_builder = Client::builder();
        match proxy_uri {
            Some(proxy_url) => {
                let proxy = reqwest::Proxy::all(proxy_url)
                    .context(format!("Failed to connect proxy {}", proxy_url))?;

                Ok(client_builder.proxy(proxy))
            }
            None => Ok(client_builder),
        }
    }

    fn url(&self, path: impl AsRef<str>) -> String {
        format!("{}{}", self.api_url, path.as_ref())
    }

    fn project_url(&self, path: impl AsRef<str>) -> String {
        format!(
            "{}/projects/{}{}",
            self.api_url,
            &self.project_id,
            path.as_ref()
        )
    }

    async fn get_access_token(&self) -> Result<String, FirebaseError> {
        let access_token = match self.emulated {
            true => "owner".to_string(),
            false => self
                .api_auth_token_manager
                .get_access_token()
                .await
                .map_err(|e| {
                    tracing::error!("Failed to get access token: {e}");
                    e
                })?,
        };

        Ok(access_token)
    }

    /// Creates a new `POST` request builder with the `Authorization` header set
    /// to an authorized admin access token.
    async fn auth_post(
        &self,
        url: impl AsRef<str>,
    ) -> Result<reqwest::RequestBuilder, FirebaseError> {
        let access_token = self.get_access_token().await?;

        let builder = self
            .post(url.as_ref())
            .await?
            .header("Authorization", format!("Bearer {access_token}"));

        Ok(builder)
    }

    /// Creates a new `POST` request builder without `Authorization` header set
    async fn post(&self, url: impl AsRef<str>) -> Result<reqwest::RequestBuilder, FirebaseError> {
        let builder = self.client.post(url.as_ref());

        Ok(builder)
    }

    /// Creates a new `GET` request builder with the `Authorization` header set
    /// to an authorized admin access token.
    async fn auth_get(
        &self,
        url: impl AsRef<str>,
    ) -> Result<reqwest::RequestBuilder, FirebaseError> {
        let access_token = self.get_access_token().await?;

        let builder = self
            .client
            .get(url.as_ref())
            .header("Authorization", format!("Bearer {access_token}"));

        Ok(builder)
    }

    /// Decodes an ID token and returns its claims. Only succeeds if the token
    /// is valid. The token is valid if it:
    ///
    /// - Is not expired
    /// - Is issued for this Firebase project
    /// - Has a valid digital signature from Google
    ///
    /// The [Firebase API docs] list further requirements.
    ///
    /// # Generic parameters
    ///
    /// The generic type parameter `C` is the format of the decoded JWT claims
    /// that will be used for deserialization. See the examples below.
    ///
    /// # Examples
    ///
    /// A valid token:
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fireplace::error::FirebaseError> {
    /// # use ulid::Ulid;
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
    /// use fireplace::auth::models::NewUser;
    ///
    /// // Create some user so we can get a valid ID token
    /// let user_id = auth_client
    ///     .create_user(
    ///         NewUser::builder()
    ///             .display_name("Mario".to_string())
    ///             .username("mario".to_string())
    ///             .email(format!("{}@example.com", Ulid::generate()))
    ///             .password( Ulid::generate().to_string())
    ///             .email_verified(false)
    ///             .build()
    ///     )
    ///     .await?;
    ///
    /// // Generate custom token, which the "user" can use to sign into Firebase
    /// let custom_token = auth_client.create_custom_token(&user_id).await?;
    ///
    /// // Sign into Firebase to obtain an ID token
    /// let id_token = auth_client.sign_in_with_custom_token(&custom_token).await?;
    ///
    /// // Decode the ID token. If we get Ok back, we know it's valid and the
    /// // user is authenticated.
    /// let decoded_token = auth_client
    ///     .decode_id_token::<serde_json::Value>(&id_token)
    ///     .await?;
    ///
    /// assert_eq!(user_id, decoded_token["user_id"].as_str().unwrap());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// An invalid token will result in an error:
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fireplace::error::FirebaseError> {
    /// # use ulid::Ulid;
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
    /// // Some invalid ID token. It is expired, and it might be issued for a
    /// // different Firebase project.
    /// let id_token = "eyJhbGciOiJSUzI1NiIsImtpZCI6IjU4NWI5MGI1OWM2YjM2ZDNjOTBkZjBlOTEwNDQ1M2U2MmY4ODdmNzciLCJ0eXAiOiJKV1QifQ.eyJpc3MiOiJodHRwczovL3NlY3VyZXRva2VuLmdvb2dsZS5jb20vcnVzdC1hZG1pbi1zZGstdGVzdCIsImF1ZCI6InJ1c3QtYWRtaW4tc2RrLXRlc3QiLCJhdXRoX3RpbWUiOjE2NjQ5OTUwNjcsInVzZXJfaWQiOiJIRnRxZ0NQc0hTTTF5SngwUnVaY0ZXbVQ5TEMzIiwic3ViIjoiSEZ0cWdDUHNIU00xeUp4MFJ1WmNGV21UOUxDMyIsImlhdCI6MTY2NDk5NTA2NywiZXhwIjoxNjY0OTk4NjY3LCJlbWFpbCI6ImZzYWZhQHRlc3RwLmFwcCIsImVtYWlsX3ZlcmlmaWVkIjpmYWxzZSwiZmlyZWJhc2UiOnsiaWRlbnRpdGllcyI6eyJlbWFpbCI6WyJmc2FmYUB0ZXN0cC5hcHAiXX0sInNpZ25faW5fcHJvdmlkZXIiOiJwYXNzd29yZCJ9fQ.ImphBsbuXJOMKyZF21YIK0PQ4ZFwPDDfJ56wW1cJkKBUhGUICW9zNv2WgCuZ03XdfexYcGabUjetruOQBx9c9eSJsPZQdAblNYk9vcBbmpaxya55HNkSbp2ZfX5S_ReUSekjiGsd53qfRLOTHxu4m-LGddE2_lfz6Mx2IAf9ij6JjU-uc5w5klmT3OAUkxUBpPyAcocwHU0WqXuOYDBo-WRL8hC2CTgQ8o0Mo-wHBsIZ_IU_SkIHG7xl2oq91Gm7q227KX7j5LnNaOgM3GuCOajPzzyKzTKAcX2pCKlkyR1bQHuefzuyPF_RME0jroOuHZm031uW_v4rnMWO3HtmDw";
    /// let decode_result = auth_client
    ///     .decode_id_token::<serde_json::Value>(id_token)
    ///     .await;
    ///
    /// assert!(decode_result.is_err());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Deserializing to your own format:
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fireplace::error::FirebaseError> {
    /// # use ulid::Ulid;
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
    /// # use fireplace::auth::models::NewUser;
    /// # use serde::Deserialize;
    /// # let user_id = auth_client
    /// #     .create_user(
    /// #        NewUser::builder()
    /// #            .display_name("Mario".to_string())
    /// #            .username("mario".to_string())
    /// #            .email(format!("{}@example.com", Ulid::generate()))
    /// #            .password( Ulid::generate().to_string())
    /// #            .email_verified(false)
    /// #            .build()
    /// #        )
    /// #     .await?;
    /// # let custom_token = auth_client.create_custom_token(&user_id).await?;
    /// # let id_token = auth_client.sign_in_with_custom_token(&custom_token).await?;
    /// #
    /// #[derive(Debug, Deserialize)]
    /// struct Claims {
    ///     user_id: String,
    ///     email: String,
    ///     firebase: FirebaseClaims,
    /// }
    ///
    /// #[derive(Debug, Deserialize)]
    /// struct FirebaseClaims {
    ///     sign_in_provider: String,
    /// }
    ///
    /// // We can make our own claims type and deserialize into that
    /// let claims = auth_client.decode_id_token::<Claims>(&id_token).await?;
    ///
    /// // Or we can just use serde_json::Value:
    /// let claims_json = auth_client
    ///     .decode_id_token::<serde_json::Value>(&id_token)
    ///     .await?;
    ///
    /// assert_eq!(claims.user_id, claims_json["user_id"].as_str().unwrap());
    /// assert_eq!(claims.email, claims_json["email"].as_str().unwrap());
    /// assert_eq!(
    ///     claims.firebase.sign_in_provider,
    ///     claims_json["firebase"]["sign_in_provider"]
    ///         .as_str()
    ///         .unwrap()
    /// );
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [Firebase API docs]: https://firebase.google.com/docs/auth/admin/verify-id-tokens#verify_id_tokens_using_a_third-party_jwt_library
    #[tracing::instrument(name = "Decode ID token", skip(self, token))]
    pub async fn decode_id_token<C: DeserializeOwned>(
        &self,
        token: &str,
    ) -> Result<C, FirebaseError> {
        let id_token_claims = self
            .user_token_manager
            .decode_id_token(token)
            .await
            .map_err(FirebaseError::ValidateTokenError)?;

        Ok(id_token_claims)
    }

    /// Create a custom token for a user, which can then be used to sign into
    /// Firebase.
    ///
    /// # Examples
    ///
    /// See the first example for [`decode_id_token`](Self::decode_id_token).
    #[tracing::instrument(name = "Create custom token", skip(self, user_id))]
    pub async fn create_custom_token(
        &self,
        user_id: impl AsRef<str>,
    ) -> Result<String, FirebaseError> {
        let user_id = user_id.as_ref();

        tracing::debug!("Creating custom token for user '{}'", user_id);

        let id_token_claims = self.user_token_manager.create_custom_token(user_id).await?;

        Ok(id_token_claims)
    }

    /// Retrieve info about a user by their user ID. Returns `None` if the user
    /// does not exist.
    ///
    /// You will also get back any custom claims that have been set on the user.
    /// See the examples in [`set_custom_user_claims`](Self::set_custom_user_claims).
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fireplace::error::FirebaseError> {
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
    /// use fireplace::auth::models::NewUser;
    /// use ulid::Ulid;
    ///
    /// // Create a user we can fetch afterwards
    /// let email = format!("{}@example.com", Ulid::generate());
    /// let user = auth_client
    ///     .create_user(
    ///         NewUser::builder()
    ///             .display_name("Mario".to_string())
    ///             .username("mario".to_string())
    ///             .email(format!("{}@example.com", Ulid::generate()))
    ///             .password( Ulid::generate().to_string())
    ///             .email_verified(false)
    ///             .build()
    ///     )
    ///     .await?;
    ///
    /// let user = auth_client.get_user(&user).await?.unwrap();
    ///
    /// assert_eq!(user.display_name, Some("Mario".to_string()));
    ///
    /// // A noteworthy thing to mention is that Firebase will turn the email
    /// // address into lowercase:
    /// assert_eq!(user.email, Some(email.to_lowercase()));
    ///
    /// // ... and there are many more fields to explore
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// If you try to fetch a user that doesn't exist, you'll get `None`:
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fireplace::error::FirebaseError> {
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
    /// assert!(auth_client.get_user("does-not-exist").await?.is_none());
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(name = "Get user", skip(self, user_id))]
    pub async fn get_user(&self, user_id: impl AsRef<str>) -> Result<Option<User>, FirebaseError> {
        let user_id = user_id.as_ref();

        let body = serde_json::json!({
            "localId": [user_id],
        });

        tracing::debug!("Retrieving user with ID '{}'", user_id);

        let res = self
            .auth_post(self.url("/accounts:lookup"))
            .await?
            .body(body.to_string())
            .send()
            .await
            .context("Failed to send get user request")?;

        if !res.status().is_success() {
            return Err(response_error("Failed to get user", res).await);
        }

        let res_body: GetAccountInfoResponse =
            res.json().await.context("Failed to read response JSON")?;
        let user = res_body.users.and_then(|mut users| users.pop());

        Ok(user)
    }

    /// Retrieve all users in Firebase Auth. By default, this will send multiple batch requests to
    /// the Firebase Auth API sequentially until all users have been retrieved. If you have
    /// thousands of users, this can take a while. See
    /// [`get_all_users_concurrently`](Self::get_all_users_concurrently) if you want these requests
    /// to be sent concurrently by `N` workers.
    #[tracing::instrument(name = "Get all users", skip_all)]
    pub async fn get_all_users(&self) -> Result<Vec<User>, FirebaseError> {
        self.get_all_users_concurrently(1).await
    }

    /// Retrieve all users in Firebase Auth using `num_workers` concurrent workers. See
    /// [`get_all_users`](Self::get_all_users) for more info.
    ///
    /// The number of workers should be between 1 and 62. If you pass a number outside
    /// this range, it will be clamped.
    ///
    /// Keep in mind that, at the time of writing, the Identity Toolkit API limit is 500
    /// requests/second per service account. This method will fire off multiple requests
    /// simultaneously. See the up-to-date limits in the [official docs](https://firebase.google.com/docs/auth/limits).
    #[tracing::instrument(name = "Get all users concurrently", skip_all)]
    pub async fn get_all_users_concurrently(
        &self,
        num_workers: u8,
    ) -> Result<Vec<User>, FirebaseError> {
        let base_url = self.project_url("/accounts:batchGet");

        fn make_pagination_url(
            base_url: &str,
            max_results: usize,
            next_page_token: Option<&str>,
        ) -> String {
            format!(
                "{base_url}?maxResults={max_results}{}",
                next_page_token
                    .map(|token| format!("&nextPageToken={token}"))
                    .unwrap_or_else(|| { "".to_string() })
            )
        }

        let legal = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
            .chars()
            .collect::<Vec<_>>();

        let num_workers = (num_workers as usize).clamp(1, legal.len());

        let chunk_size = legal.len() / num_workers;
        let mut ranges = Vec::new();

        for i in 0..num_workers {
            let min = legal[i * chunk_size];
            let max = if i < num_workers - 1 {
                legal[(i + 1) * chunk_size]
            } else {
                (*legal.last().unwrap() as u8 + 1) as char
            };

            ranges.push((min, max));
        }

        async fn run_worker(
            client: &FirebaseAuthClient,
            range: (char, char),
            base_url: &str,
        ) -> Result<Vec<User>, FirebaseError> {
            const MAX_RESULTS: usize = 1000;
            let (min, max) = (range.0.to_string(), range.1.to_string());

            let mut all_users = Vec::new();
            let mut next_page_token = Some(min);
            loop {
                let url = make_pagination_url(base_url, MAX_RESULTS, next_page_token.as_deref());

                let res = client
                    .auth_get(url)
                    .await?
                    .header("Content-Type", "application/json")
                    .send()
                    .await
                    .context("Failed to send get all users request")?;

                if !res.status().is_success() {
                    return Err(response_error("Failed to get all users", res).await);
                }

                let res_body: BatchGetResponse =
                    res.json().await.context("Failed to read response JSON")?;

                if let Some(mut users) = res_body.users {
                    all_users.append(&mut users);
                }

                next_page_token = res_body.next_page_token.map(|t| t.to_string());

                if next_page_token.is_none() || next_page_token.as_deref() >= Some(&max) {
                    break;
                }
            }

            Ok(all_users)
        }

        let futures = ranges
            .into_iter()
            .map(|range| run_worker(self, range, &base_url));

        let worker_results = futures::future::try_join_all(futures).await?;

        let mut deduped = HashMap::new();
        for users in worker_results {
            for user in users {
                deduped.insert(user.uid.clone(), user);
            }
        }

        let all_users = deduped.into_values().collect();

        Ok(all_users)
    }

    /// Creates a new user in Firebase Auth using the email/password provider.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fireplace::error::FirebaseError> {
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
    /// use fireplace::{auth::models::NewUser, error::FirebaseError};
    /// use ulid::Ulid;
    ///
    /// let new_user = NewUser::builder()
    ///     .display_name("Mario".to_string())
    ///     .username("mario".to_string())
    ///     .email(format!("{}@example.com", Ulid::generate()))
    ///     .password( Ulid::generate().to_string())
    ///     .email_verified(false)
    ///     .build();
    ///
    /// // When we create the user, we get back their unique user ID
    /// let user_id = auth_client.create_user(new_user.clone()).await?;
    ///
    /// println!("Created user with ID '{}'", user_id);
    ///
    /// // If we attempt to create another user with the same email, Firebase
    /// // will complain
    /// let create_again_result = auth_client.create_user(new_user).await;
    ///
    /// assert!(matches!(
    ///     create_again_result,
    ///     Err(FirebaseError::EmailAlreadyExists)
    /// ));
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(name = "Create user", skip(self, new_user))]
    pub async fn create_user(&self, new_user: NewUser) -> Result<String, FirebaseError> {
        let body = serde_json::to_string(&new_user).context("Failed to serialize new user")?;

        let res = self
            .auth_post(self.url("/accounts:signUp"))
            .await?
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .context("Failed to send create user request")?;

        if !res.status().is_success() {
            let err = res
                .json::<AuthApiErrorResponse>()
                .await
                .context("Failed to read error response JSON")?
                .into();

            tracing::error!("Failed to create user: {}", &err);

            return Err(err);
        }

        #[derive(Deserialize)]
        struct SignUpResponse {
            #[serde(rename = "localId")]
            uid: String,
        }

        let res_body: SignUpResponse = res.json().await.context("Failed to read response JSON")?;

        tracing::info!("Created user with id '{}'", &res_body.uid);

        Ok(res_body.uid)
    }

    /// Updates a user's attributes in Firebase Auth, such as email or display name.
    ///
    /// This function allows you to update specific fields of a user. Passing `None` for a field
    /// will remove it. Only the provided fields will be modified; others remain unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fireplace::error::FirebaseError> {
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
    /// use fireplace::auth::models::{NewUser, UpdateUserValues};
    /// use ulid::Ulid;
    ///
    /// let username = format!("julius-{}", Ulid::generate());
    /// let new_user = NewUser::builder()
    ///     .display_name("Julius Caesar".to_string())
    ///     .username(username.to_string())
    ///     .email(format!("{}@example.com", username.to_string()))
    ///     .password(Ulid::generate().to_string())
    ///     .email_verified(false)
    ///     .build();
    ///
    /// let user_id = auth_client
    ///     .create_user(new_user)
    ///     .await?;
    ///
    /// // Give a new value for the email
    /// let new_email = format!("caesar@deceased{}.it", Ulid::generate());
    ///
    /// // Pass `None` to delete a field
    /// let new_display_name: Option<String> = None;
    ///
    /// let res = auth_client
    ///     .update_user(
    ///         &user_id,
    ///          UpdateUserValues::builder()
    ///             .display_name(new_display_name)
    ///             .email(Some(new_email.clone()))
    ///             .build(),
    ///     )
    ///     .await?;
    ///
    /// assert_eq!(res.email, Some(new_email.to_lowercase()));
    /// assert_eq!(res.display_name, None);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Disable a user:
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fireplace::error::FirebaseError> {
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
    /// use fireplace::auth::models::{NewUser, UpdateUserValues};
    /// use ulid::Ulid;
    ///
    /// let username = format!("julius-{}", Ulid::generate());
    /// let new_user = NewUser::builder()
    ///     .display_name("Julius Caesar".to_string())
    ///     .email(format!("{}@example.com", username))
    ///     .password(Ulid::generate().to_string())
    ///     .email_verified(false)
    ///     .build();
    ///
    /// let user_id = auth_client
    ///     .create_user(new_user)
    ///     .await?;
    ///
    /// // Disable the user
    /// auth_client
    ///     .update_user(&user_id, UpdateUserValues::builder().disabled(Some(true)).build())
    ///     .await?;
    ///
    /// // Verify the user is disabled
    /// let user = auth_client.get_user(&user_id).await?.unwrap();
    /// assert_eq!(user.disabled, Some(true));
    ///
    /// // Re-enable the user
    /// auth_client
    ///     .update_user(&user_id, UpdateUserValues::builder().disabled(Some(false)).build())
    ///     .await?;
    ///
    /// // Verify the user is enabled again
    /// let user = auth_client.get_user(&user_id).await?.unwrap();
    /// assert_eq!(user.disabled, Some(false));
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(name = "Update user", skip_all, fields(user_id = %user_id.as_ref()))]
    pub async fn update_user(
        &self,
        user_id: impl AsRef<str>,
        updated_values: UpdateUserValues,
    ) -> Result<User, FirebaseError> {
        let body_values = UpdateUserBody::from_values(user_id.as_ref(), updated_values);
        let body =
            serde_json::to_string(&body_values).context("Failed to serialize updated values")?;

        let res = self
            .auth_post(self.url("/accounts:update"))
            .await?
            .body(body)
            .send()
            .await
            .context("Failed to send update user request")?;

        if !res.status().is_success() {
            let err = res
                .json::<AuthApiErrorResponse>()
                .await
                .context("Failed to read error response JSON")?
                .into();

            tracing::error!("Failed to update user: {err}");

            return Err(err);
        }

        let res_body: User = res.json().await.context("Failed to read response JSON")?;

        tracing::info!("Updated user with id '{}'", &res_body.uid);

        Ok(res_body)
    }

    /// Signs into Firebase with a custom generated token, which you can get
    /// from [`create_custom_token`](Self::create_custom_token). Returns an ID
    /// token for Firebase.
    ///
    /// # Examples
    ///
    /// See the first example for [`decode_id_token`](Self::decode_id_token).
    #[tracing::instrument(name = "Sign in with custom token", skip(self, custom_token))]
    pub async fn sign_in_with_custom_token(
        &self,
        custom_token: impl AsRef<str>,
    ) -> Result<String, FirebaseError> {
        tracing::debug!("Signing in with custom token");

        let body = serde_json::json!({
            "token": custom_token.as_ref(),
            "returnSecureToken": true,
        });

        let res = self
            .auth_post(self.url("/accounts:signInWithCustomToken"))
            .await?
            .body(body.to_string())
            .send()
            .await
            .context("Failed to send sign-in request")?;

        if !res.status().is_success() {
            return Err(response_error("Failed to get user", res).await);
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SignInResponse {
            id_token: String,
        }

        let res_body: SignInResponse = res.json().await.context("Failed to read response JSON")?;

        Ok(res_body.id_token)
    }

    /// Set custom attributes on a user. The attributes can be anything JSON-
    /// serializable. This will overwrite any existing attributes competely.
    ///
    /// The fields that you set as custom claims will show up in the ID token
    /// claims. This can, for example, be useful for access-control. Note that
    /// users need to re-authenticate for the custom claims to appear in the ID
    /// token.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), anyhow::Error> {
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
    ///
    /// use fireplace::auth::models::NewUser;
    /// use serde::{Deserialize, Serialize};
    /// use ulid::Ulid;
    ///
    /// // Create a user we can set some claims on
    /// let username = format!("julius-{}", Ulid::generate());
    /// let new_user = NewUser::builder()
    ///     .display_name("Julius Caesar".to_string())
    ///     .email(format!("{}@example.com", username))
    ///     .password(Ulid::generate().to_string())
    ///     .email_verified(false)
    ///     .build();
    ///
    /// let user_id = auth_client
    ///     .create_user(new_user)
    ///     .await?;
    ///
    /// // Initially, the user will have no claims
    /// let user = auth_client.get_user(&user_id).await?.unwrap();
    /// assert_eq!(user.custom_claims, serde_json::Value::Null);
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct CustomClaims {
    ///     #[serde(default)]
    ///     roles: Vec<String>,
    /// }
    ///
    /// // Set some custom claims
    /// auth_client
    ///     .set_custom_user_claims(
    ///         &user_id,
    ///         CustomClaims {
    ///             roles: vec!["superhero".to_string()],
    ///         },
    ///     )
    ///     .await?;
    ///
    /// // Now, the user should have those claims as a JSON value
    /// let user = auth_client.get_user(&user_id).await?.unwrap();
    /// let custom_claims: CustomClaims = serde_json::from_value(user.custom_claims)?;
    ///
    /// assert_eq!(custom_claims.roles, vec!["superhero"]);
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(name = "Set custom user claims", skip(self, user_id, new_claims))]
    pub async fn set_custom_user_claims<C: Serialize>(
        &self,
        user_id: &str,
        new_claims: C,
    ) -> Result<(), FirebaseError> {
        let custom_claims =
            serde_json::to_string(&new_claims).context("Failed to serialize claims")?;

        let body = serde_json::json!({
            "localId": user_id,
            "customAttributes": custom_claims,
        });

        let res = self
            .auth_post(self.url("/accounts:update"))
            .await?
            .body(body.to_string())
            .send()
            .await
            .context("Failed to send custom claims request")?;

        if !res.status().is_success() {
            return Err(response_error("Failed to set custom user claims", res).await);
        }

        tracing::debug!("Set custom claims for user '{}'", user_id);

        Ok(())
    }

    /// Authenticates a user with the Firebase Identity Toolkit using an email and password.
    ///
    /// Sends an HTTP `POST` request to the `/accounts:signInWithPassword` endpoint to exchange
    /// user credentials for authentication tokens and claims.
    ///
    /// # Arguments
    ///
    /// * `email` - The user's registered email address.
    /// * `password` - The raw password associated with the account.
    /// * `secure_token` - Whether to return an ID token and refresh token in the response (`true` recommended).
    ///
    /// # Returns
    ///
    /// * `Ok(AuthClaims)` - Upon successful authentication, returns claims containing token details and account metadata.
    /// * `Err(FirebaseError)` - If the credentials are invalid, the account is disabled, or a network/parsing error occurs.
    ///
    /// # Errors
    ///
    /// This function will return [`FirebaseError`](FirebaseError) if:
    ///
    /// * The Firebase API returns an error response (e.g., `EMAIL_NOT_FOUND`, `INVALID_PASSWORD`, `USER_DISABLED`).
    /// * Sending the HTTP request to the Firebase Identity Toolkit fails.
    /// * Parsing the response or error body JSON fails.
    ///
    /// # Tracing & Telemetry
    ///
    /// This function is instrumented with `tracing::instrument`. The `password` argument is **skipped**
    /// from the generated trace span to prevent sensitive credentials from leaking into logs.
    ///
    /// # Example
    ///
    /// ```
    /// let auth_claims = auth_client
    ///     .login_with_password("joe", "password", true)
    ///     .await.expect("Authentication Failure for user login with password");
    /// println!("User logged in successfully : {:?}", auth_claims);
    /// ```
    #[tracing::instrument(name = "login with password", skip(self, password))]
    pub async fn login_with_password(
        &self,
        email: &str,
        password: &str,
        secure_token: bool,
    ) -> Result<AuthClaims, FirebaseError> {
        let body = serde_json::json!({
            "email": email,
            "password": password,
            "returnSecureToken": secure_token,
        });

        let res = self
            .auth_post(self.url("/accounts:signInWithPassword"))
            .await?
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .context("Failed to send password login request")?;

        if !res.status().is_success() {
            let err = res
                .json::<AuthApiErrorResponse>()
                .await
                .context("Failed to read error response JSON")?
                .into();

            tracing::error!("Failed to authenticate password credentials: {err}");

            return Err(err);
        }

        let claims: AuthClaims = res.json().await.context("Failed to read response JSON")?;

        tracing::debug!("User logged in successfully {:?}", claims);

        Ok(claims)
    }

    #[tracing::instrument(name = "refresh token", skip(self, api_key, refresh_token))]
    pub async fn refresh_token(
        &self,
        api_key: &str,
        refresh_token: &str,
    ) -> Result<RefreshTokenClaims, FirebaseError> {
        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", refresh_token);

        let refresh_token_url = match self.emulated {
            true => {
                let base_emulator_url = Url::parse(self.api_url.as_ref())
                    .context("Failure to parse url for emulator")?
                    .origin()
                    .ascii_serialization();
                format!("{base_emulator_url}/securetoken.googleapis.com/v1/token?key={api_key}")
            }
            false => self.url(format!("/token?key={api_key}")),
        };
        let res = self
            .post(refresh_token_url)
            .await?
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await
            .context("Failed to send refresh token request")?;

        if !res.status().is_success() {
            let err = res
                .json::<AuthApiErrorResponse>()
                .await
                .context("Failed to deserialize JSON Response")?
                .into();

            tracing::error!("Failed to refresh user token: {err}");

            return Err(err);
        }

        let refresh_claims: RefreshTokenClaims =
            res.json().await.context("Failed to read response JSON")?;

        tracing::debug!("Token refresh is successful {:?}", refresh_claims);

        Ok(refresh_claims)
    }

    #[tracing::instrument(name = "revoke refresh tokens", skip(self))]
    pub async fn revoke_refresh_tokens(&self, user_id: &str) -> Result<(), FirebaseError> {
        let now = Utc::now().timestamp();

        let body = serde_json::json!({
            "localId": user_id,
            "validSince": now,
        });

        let res = self
            .auth_post(self.url("/accounts:update"))
            .await?
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .context("Failed to send account update to revoke refresh tokens")?;

        if !res.status().is_success() {
            let err = res
                .json::<AuthApiErrorResponse>()
                .await
                .context("Failed to read error response JSON")?
                .into();

            tracing::error!("Failed to revoke user refresh tokens: {err}");

            return Err(err);
        }

        let updated_user_body: String = res.text().await.context("Failed to read response Text")?;

        tracing::debug!(
            "Refresh Tokens has been revoked successfully: {}",
            updated_user_body
        );

        Ok(())
    }
}

async fn response_error(msg: &'static str, res: Response) -> FirebaseError {
    let status = res.status();
    let body = res.text().await.unwrap_or_default();

    let err = anyhow::anyhow!("{} (status: {}): {}", msg, status, body).into();

    tracing::error!("{:?}'", &err);

    err
}
