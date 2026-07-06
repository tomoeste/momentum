use crate::errors::{AppError, Result};
use keyring::Entry;

const SIMPLEFIN_SERVICE: &str = "com.momentum.simplefin";
const SIMPLEFIN_ACCOUNT: &str = "access_url";
const LLM_SERVICE: &str = "com.momentum.llm";
const LLM_API_KEY_ACCOUNT: &str = "api_key";

pub struct Keychain;

impl Keychain {
    // SimpleFIN methods
    pub fn store_simplefin_access_url(access_url: &str) -> Result<()> {
        let entry = Entry::new(SIMPLEFIN_SERVICE, SIMPLEFIN_ACCOUNT)
            .map_err(|e| AppError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        entry
            .set_password(access_url)
            .map_err(|e| AppError::Keychain(format!("Failed to store credentials in keychain: {}", e)))?;

        Ok(())
    }

    pub fn retrieve_simplefin_access_url() -> Result<String> {
        let entry = Entry::new(SIMPLEFIN_SERVICE, SIMPLEFIN_ACCOUNT)
            .map_err(|e| AppError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        entry
            .get_password()
            .map_err(|e| AppError::Keychain(format!("Failed to retrieve credentials from keychain: {}", e)))
    }

    pub fn delete_simplefin_access_url() -> Result<()> {
        let entry = Entry::new(SIMPLEFIN_SERVICE, SIMPLEFIN_ACCOUNT)
            .map_err(|e| AppError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        entry
            .delete_password()
            .map_err(|e| AppError::Keychain(format!("Failed to delete credentials from keychain: {}", e)))?;

        Ok(())
    }

    pub fn has_simplefin_access_url() -> Result<bool> {
        let entry = Entry::new(SIMPLEFIN_SERVICE, SIMPLEFIN_ACCOUNT)
            .map_err(|e| AppError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        match entry.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::error::Error::NoEntry) => Ok(false),
            Err(e) => Err(AppError::Keychain(format!("Failed to check keychain: {}", e))),
        }
    }

    // LLM API Key methods
    pub fn store_llm_api_key(api_key: &str) -> Result<()> {
        let entry = Entry::new(LLM_SERVICE, LLM_API_KEY_ACCOUNT)
            .map_err(|e| AppError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        entry
            .set_password(api_key)
            .map_err(|e| AppError::Keychain(format!("Failed to store LLM API key in keychain: {}", e)))?;

        Ok(())
    }

    pub fn retrieve_llm_api_key() -> Result<String> {
        let entry = Entry::new(LLM_SERVICE, LLM_API_KEY_ACCOUNT)
            .map_err(|e| AppError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        entry
            .get_password()
            .map_err(|e| AppError::Keychain(format!("Failed to retrieve LLM API key from keychain: {}", e)))
    }

    pub fn delete_llm_api_key() -> Result<()> {
        let entry = Entry::new(LLM_SERVICE, LLM_API_KEY_ACCOUNT)
            .map_err(|e| AppError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        entry
            .delete_password()
            .map_err(|e| AppError::Keychain(format!("Failed to delete LLM API key from keychain: {}", e)))?;

        Ok(())
    }

    pub fn has_llm_api_key() -> Result<bool> {
        let entry = Entry::new(LLM_SERVICE, LLM_API_KEY_ACCOUNT)
            .map_err(|e| AppError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        match entry.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::error::Error::NoEntry) => Ok(false),
            Err(e) => Err(AppError::Keychain(format!("Failed to check keychain: {}", e))),
        }
    }
}
