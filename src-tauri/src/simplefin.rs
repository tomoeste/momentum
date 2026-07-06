use crate::errors::Result;
use crate::models::{Account, RawTransaction};

pub struct SimpleFin {
    access_url: String,
}

impl SimpleFin {
    pub fn new(access_url: String) -> Self {
        SimpleFin { access_url }
    }

    pub async fn fetch_accounts(&self) -> Result<Vec<Account>> {
        // TODO: implement SimpleFIN /accounts endpoint
        Ok(Vec::new())
    }

    pub async fn fetch_transactions(&self, days_back: u32) -> Result<Vec<RawTransaction>> {
        // TODO: implement SimpleFIN /transactions endpoint with date filtering
        Ok(Vec::new())
    }

    pub fn validate_access_url(access_url: &str) -> Result<()> {
        // TODO: validate that access_url is properly formatted
        Ok(())
    }
}
