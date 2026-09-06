use anyhow::Context;
use env_logger::Env;
use fireplace::auth::FirebaseAuthClient;
use fireplace::auth::models::NewUser;
use ulid::Ulid;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .is_test(false)
        .init();

    let auth_client = FirebaseAuthClient::emulator("http://localhost:9099", None)?;

    let uid = Ulid::generate();
    let new_user = NewUser::builder()
        .username(format!("dessalines-{}", uid.to_string()))
        .email(format!("dessalines-{}@example.com", uid.to_string()))
        .password("hello123".to_string())
        .email_verified(true)
        .build();

    auth_client
        .create_user(new_user.clone())
        .await
        .context("Failed to create user")?;

    let users = auth_client
        .get_all_users()
        .await
        .context("Failed to get u sers")?;

    println!(" Here are the list of users {:#?}", users);
    let auth_claims = auth_client
        .login_with_password(new_user.email.as_str(), new_user.password.as_str(), true)
        .await
        .context("Failed to authenticate password credentials")?;

    println!("User logged in successfully {:?}", auth_claims);

    let api_key = "API_KEY";

    let token_refresh_claims = auth_client
        .refresh_token(api_key, auth_claims.refresh_token.as_str())
        .await
        .context("Failure refreshing token")?;

    println!("Token refreshing successfully {:?}", token_refresh_claims);

    auth_client
        .revoke_refresh_tokens(token_refresh_claims.user_id.as_str())
        .await
        .context("Failed to revoke refresh tokens")?;

    println!("Revoke Refresh tokens is successful");

    let oob_code = auth_client
        .generate_email_verification_link(new_user.email.as_str())
        .await
        .context("Failed to generate email verification link")?;

    println!("Email verification link successfully {:?}", oob_code);

    Ok(())
}
