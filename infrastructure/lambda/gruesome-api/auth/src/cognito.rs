use aws_sdk_cognitoidentityprovider::{
    types::{AttributeType, AuthFlowType},
    Client,
};
use std::collections::HashMap;
use tracing::{error, info};

use crate::error::{AuthError, AuthResult};

pub struct CognitoService {
    client: Client,
    user_pool_id: String,
    client_id: String,
}

impl CognitoService {
    pub fn new(client: Client, user_pool_id: String, client_id: String) -> Self {
        Self {
            client,
            user_pool_id,
            client_id,
        }
    }

    /// Sign up a new user
    pub async fn sign_up(
        &self,
        email: &str,
        password: &str,
        username: &str,
    ) -> AuthResult<String> {
        info!("Signing up user: {}", email);

        let email_attr = AttributeType::builder()
            .name("email")
            .value(email)
            .build()
            .map_err(|e| AuthError::InternalError(format!("Failed to build email attribute: {}", e)))?;

        let result = self
            .client
            .sign_up()
            .client_id(&self.client_id)
            .username(username) // Use provided username (email is an alias)
            .password(password)
            .user_attributes(email_attr)
            .send()
            .await
            .map_err(|e| {
                error!("Cognito SignUp error: {:?}", e);
                match e.to_string() {
                    s if s.contains("UsernameExistsException") => AuthError::UserAlreadyExists,
                    s if s.contains("InvalidPasswordException") => {
                        AuthError::InvalidRequest("Password does not meet requirements".to_string())
                    }
                    s if s.contains("InvalidParameterException") => {
                        AuthError::InvalidRequest("Invalid parameter".to_string())
                    }
                    _ => AuthError::CognitoError(e.to_string()),
                }
            })?;

        let user_sub = result.user_sub();

        info!("User signed up successfully: {}", user_sub);
        Ok(user_sub.to_string())
    }

    /// Confirm user signup (auto-confirm for testing)
    pub async fn admin_confirm_sign_up(&self, username: &str) -> AuthResult<()> {
        info!("Admin confirming user: {}", username);

        self.client
            .admin_confirm_sign_up()
            .user_pool_id(&self.user_pool_id)
            .username(username)
            .send()
            .await
            .map_err(|e| {
                error!("Cognito AdminConfirmSignUp error: {:?}", e);
                AuthError::CognitoError(e.to_string())
            })?;

        info!("User confirmed successfully");
        Ok(())
    }

    /// Authenticate user and get tokens
    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> AuthResult<HashMap<String, String>> {
        info!("Authenticating user: {}", username);

        let mut auth_params = HashMap::new();
        auth_params.insert("USERNAME".to_string(), username.to_string());
        auth_params.insert("PASSWORD".to_string(), password.to_string());

        let result = self
            .client
            .initiate_auth()
            .auth_flow(AuthFlowType::UserPasswordAuth)
            .client_id(&self.client_id)
            .set_auth_parameters(Some(auth_params))
            .send()
            .await
            .map_err(|e| {
                error!("Cognito InitiateAuth error: {:?}", e);
                match e.to_string() {
                    s if s.contains("NotAuthorizedException") => AuthError::InvalidCredentials,
                    s if s.contains("UserNotFoundException") => AuthError::UserNotFound,
                    s if s.contains("UserNotConfirmedException") => AuthError::EmailNotVerified,
                    _ => AuthError::CognitoError(e.to_string()),
                }
            })?;

        let auth_result = result
            .authentication_result()
            .ok_or_else(|| AuthError::InternalError("No authentication result".to_string()))?;

        let mut tokens = HashMap::new();

        if let Some(access_token) = auth_result.access_token() {
            tokens.insert("access_token".to_string(), access_token.to_string());
        }

        if let Some(refresh_token) = auth_result.refresh_token() {
            tokens.insert("refresh_token".to_string(), refresh_token.to_string());
        }

        if let Some(id_token) = auth_result.id_token() {
            tokens.insert("id_token".to_string(), id_token.to_string());
        }

        let expires_in = auth_result.expires_in();
        tokens.insert("expires_in".to_string(), expires_in.to_string());

        info!("User authenticated successfully");
        Ok(tokens)
    }

    /// Refresh access token
    pub async fn refresh_token(&self, refresh_token: &str) -> AuthResult<HashMap<String, String>> {
        info!("Refreshing token");

        let mut auth_params = HashMap::new();
        auth_params.insert("REFRESH_TOKEN".to_string(), refresh_token.to_string());

        let result = self
            .client
            .initiate_auth()
            .auth_flow(AuthFlowType::RefreshTokenAuth)
            .client_id(&self.client_id)
            .set_auth_parameters(Some(auth_params))
            .send()
            .await
            .map_err(|e| {
                error!("Cognito RefreshToken error: {:?}", e);
                match e.to_string() {
                    s if s.contains("NotAuthorizedException") => AuthError::InvalidToken,
                    _ => AuthError::CognitoError(e.to_string()),
                }
            })?;

        let auth_result = result
            .authentication_result()
            .ok_or_else(|| AuthError::InternalError("No authentication result".to_string()))?;

        let mut tokens = HashMap::new();

        if let Some(access_token) = auth_result.access_token() {
            tokens.insert("access_token".to_string(), access_token.to_string());
        }

        if let Some(id_token) = auth_result.id_token() {
            tokens.insert("id_token".to_string(), id_token.to_string());
        }

        let expires_in = auth_result.expires_in();
        tokens.insert("expires_in".to_string(), expires_in.to_string());

        info!("Token refreshed successfully");
        Ok(tokens)
    }

    /// Get user by access token
    pub async fn get_user(&self, access_token: &str) -> AuthResult<HashMap<String, String>> {
        info!("Getting user from access token");

        let result = self
            .client
            .get_user()
            .access_token(access_token)
            .send()
            .await
            .map_err(|e| {
                error!("Cognito GetUser error: {:?}", e);
                match e.to_string() {
                    s if s.contains("NotAuthorizedException") => AuthError::InvalidToken,
                    _ => AuthError::CognitoError(e.to_string()),
                }
            })?;

        let mut user_data = HashMap::new();

        let username = result.username();
        user_data.insert("username".to_string(), username.to_string());

        for attr in result.user_attributes() {
            let name = attr.name();
            if let Some(value) = attr.value() {
                user_data.insert(name.to_string(), value.to_string());
            }
        }

        Ok(user_data)
    }
}
