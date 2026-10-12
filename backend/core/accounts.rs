//! Connection lifecycle: login, saved accounts, account switching, logout.
//! Passwords live in the OS keyring keyed by (server, username).

use serde::Serialize;

use super::Core;
use crate::commands::{credentials, subsonic};
use crate::config::SavedAccount;
use crate::errors::UserError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthState {
    pub authed: bool,
    pub server: Option<String>,
    pub username: Option<String>,
    pub accounts: Vec<SavedAccount>,
}

fn normalize_server(s: &str) -> String {
    s.trim().trim_end_matches('/').to_string()
}

impl Core {
    pub fn auth_state(&self) -> AuthState {
        let conn = self.backend.app_state.connection.read();
        let authed = conn.server.is_some() && conn.username.is_some() && conn.password.is_some();
        AuthState {
            authed,
            server: conn.server.clone(),
            username: conn.username.clone(),
            accounts: self.config.lock().accounts.clone(),
        }
    }

    /// Validates credentials against the server. On success the account is
    /// remembered and (optionally) its password saved to the keyring.
    pub async fn connect(&self, server: String, username: String, password: String, save_password: bool) -> Result<AuthState, UserError> {
        let server = normalize_server(&server);
        let username = username.trim().to_string();
        if server.is_empty() || username.is_empty() {
            return Err(UserError::Unknown);
        }
        let state = self.backend.app_state.clone();
        // A failed attempt (wrong password, server down) must not log the user out of a working session.
        let previous = state.connection.read().clone();
        subsonic::set_connection(&state, Some(server.clone()), Some(username.clone()), Some(password.clone()));
        if let Err(e) = subsonic::validate_connection(state.clone()).await {
            *state.connection.write() = previous;
            return Err(e);
        }
        if save_password {
            let _ = credentials::save_password(Some(&server), &username, &password);
        }
        {
            let mut c = self.config.lock();
            c.server = Some(server.clone());
            c.username = Some(username.clone());
            let acct = SavedAccount { server, username };
            if !c.accounts.contains(&acct) {
                c.accounts.push(acct);
            }
        }
        self.save_config();
        Ok(self.auth_state())
    }

    /// Startup login with the last account's keyring password. `Ok(None)` when
    /// nothing is saved (UI shows the login screen / offline library).
    pub async fn auto_login(&self) -> Result<Option<AuthState>, UserError> {
        let (server, username) = {
            let c = self.config.lock();
            (c.server.clone(), c.username.clone())
        };
        let (Some(server), Some(username)) = (server, username) else { return Ok(None) };
        let Ok(password) = credentials::get_password(Some(&server), &username) else { return Ok(None) };
        self.connect(server, username, password, false).await.map(Some)
    }

    /// Switches to a saved account using its keyring password. Errors with
    /// `Auth` when the password is no longer stored (UI prompts for it).
    pub async fn switch_account(&self, server: String, username: String) -> Result<AuthState, UserError> {
        let password = credentials::get_password(Some(&server), &username).map_err(|_| UserError::Auth)?;
        self.connect(server, username, password, false).await
    }

    pub fn logout(&self) -> AuthState {
        subsonic::set_connection(&self.backend.app_state, None, None, None);
        self.auth_state()
    }

    /// Forgets a saved account and its keyring password.
    pub fn remove_account(&self, server: &str, username: &str) -> AuthState {
        let _ = credentials::delete_password(Some(server), username);
        self.config.lock().accounts.retain(|a| !(a.server == server && a.username == username));
        self.save_config();
        self.auth_state()
    }
}
