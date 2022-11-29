use std::env;

use crate::ServiceAccount;

use super::{AuthClientOptions, FirebaseAuthClient};

pub fn initialise() -> Result<FirebaseAuthClient, anyhow::Error> {
    let service_account = ServiceAccount {
        project_id: env::var("FIREBASE_PROJECT_ID")?,
        client_id: env::var("FIREBASE_CLIENT_ID")?,
        client_email: env::var("FIREBASE_CLIENT_EMAIL")?,
        private_key_id: env::var("FIREBASE_PRIVATE_KEY_ID")?,
        private_key: env::var("FIREBASE_PRIVATE_KEY")?.replace(r"\n", "\n"),
    };

    let auth_options = AuthClientOptions::default();
    let auth_client = FirebaseAuthClient::new(service_account, auth_options)?;

    Ok(auth_client)
}
