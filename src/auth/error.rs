use anyhow::anyhow;
use serde::Deserialize;

use crate::error::FirebaseError;

#[derive(Debug, Deserialize)]
pub(crate) struct AuthApiErrorResponse {
    error: AuthApiErrorInfo,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct AuthApiErrorInfo {
    pub message: String,
    pub errors: Vec<SpecificAuthApiErrorInfo>,
    pub code: u16,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct SpecificAuthApiErrorInfo {
    pub domain: Option<String>,
    pub message: String,
    pub reason: String,
}

impl From<AuthApiErrorResponse> for FirebaseError {
    fn from(err: AuthApiErrorResponse) -> Self {
        match err.error.message.as_ref() {
            "EMAIL_EXISTS" => FirebaseError::EmailAlreadyExists,
            "USER_NOT_FOUND" => FirebaseError::UserNotFound,
            _ => anyhow!("{:?}", err).into(),
        }
    }
}
