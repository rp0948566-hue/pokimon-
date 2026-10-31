//! OS keyring storage for the cloud refresh token (Linux Secret Service / platform keychain).
//!
//! There is deliberately no file fallback: when no keyring is available the session is not remembered
//! and the user signs in again next launch. The token is never logged or put in an error text.

use zeroize::Zeroizing;

const USER: &str = "supabase-refresh-token";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    /// No usable OS keyring (not installed, locked, or access denied).
    Unavailable,
}

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "No OS keyring is available, so the sign-in cannot be remembered."
        )
    }
}

fn service() -> String {
    // Tests use their own service name so they never touch a real session.
    std::env::var("RETINACARE_KEYRING_SERVICE")
        .ok()
        .filter(|s| !s.is_empty() && s.len() < 64)
        .unwrap_or_else(|| format!("RetinaCare/{}", super::supabase::PROJECT_REF))
}

fn entry() -> Result<keyring::Entry, KeyError> {
    keyring::Entry::new(&service(), USER).map_err(|_| KeyError::Unavailable)
}

pub fn save_refresh_token(token: &str) -> Result<(), KeyError> {
    entry()?
        .set_password(token)
        .map_err(|_| KeyError::Unavailable)
}

/// `Ok(None)` when nothing is stored; `Err` when the keyring itself cannot be used.
pub fn load_refresh_token() -> Result<Option<Zeroizing<String>>, KeyError> {
    match entry()?.get_password() {
        Ok(token) => Ok(Some(Zeroizing::new(token))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err(KeyError::Unavailable),
    }
}

pub fn delete_refresh_token() {
    if let Ok(entry) = entry() {
        let _ = entry.delete_credential();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a running, unlocked OS secret service (true on the development machine, not in CI).
    #[test]
    #[ignore]
    fn token_round_trips_through_the_os_keyring_and_is_removed() {
        std::env::set_var("RETINACARE_KEYRING_SERVICE", "RetinaCare-test-keyring");
        delete_refresh_token();
        assert_eq!(load_refresh_token().unwrap(), None);
        save_refresh_token("synthetic-test-token").unwrap();
        assert_eq!(
            load_refresh_token().unwrap().as_deref().map(String::as_str),
            Some("synthetic-test-token")
        );
        delete_refresh_token();
        assert_eq!(load_refresh_token().unwrap(), None);
    }
}
