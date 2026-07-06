use crate::errors::{AppError, Result};
use keyring::Entry;

const SERVICE_NAME: &str = "com.momentum.simplefin";
const ACCOUNT_NAME: &str = "access_url";

pub struct Keychain;

impl Keychain {
    pub fn store_simplefin_access_url(access_url: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)
            .map_err(|e| AppError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        entry
            .set_password(access_url)
            .map_err(|e| AppError::Keychain(format!("Failed to store credentials in keychain: {}", e)))?;

        Ok(())
    }

    pub fn retrieve_simplefin_access_url() -> Result<String> {
        let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)
            .map_err(|e| AppError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        entry
            .get_password()
            .map_err(|e| AppError::Keychain(format!("Failed to retrieve credentials from keychain: {}", e)))
    }

    pub fn delete_simplefin_access_url() -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)
            .map_err(|e| AppError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        entry
            .delete_password()
            .map_err(|e| AppError::Keychain(format!("Failed to delete credentials from keychain: {}", e)))?;

        Ok(())
    }

    pub fn has_simplefin_access_url() -> Result<bool> {
        let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)
            .map_err(|e| AppError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        match entry.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::error::Error::NoEntry) => Ok(false),
            Err(e) => Err(AppError::Keychain(format!("Failed to check keychain: {}", e))),
        }
    }
}
