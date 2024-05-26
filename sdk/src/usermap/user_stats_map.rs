use std::{collections::HashMap, time::Duration};

use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    pubkey::Pubkey,
};

use crate::{
    accounts::BulkAccountLoader,
    types::{SdkResult, UserStatsAccount},
    AccountProvider, DriftClient,
};

pub struct UserStatsMap<T, U>
where
    T: AccountProvider,
{
    user_stats_map: HashMap<String, UserStatsAccount>,
    drift_client: DriftClient<T, U>,
    bulk_account_loader: BulkAccountLoader,
}

impl<T, U> UserStatsMap<T, U>
where
    T: AccountProvider,
{
    pub fn new(
        drift_client: DriftClient<T, U>,
        bulk_account_loader: Option<BulkAccountLoader>,
    ) -> Self {
        let bulk_account_loader = match bulk_account_loader {
            Some(loader) => loader,
            None => BulkAccountLoader::new(
                drift_client.backend.rpc_client.clone(),
                drift_client.backend.account_provider.commitment_config(),
                Duration::from_secs(0),
            ),
        };

        Self {
            user_stats_map: HashMap::new(),
            drift_client,
            bulk_account_loader,
        }
    }

    pub async fn subscribe(&self, authorities: &[Pubkey]) -> SdkResult<()> {
        if self.user_stats_map.is_empty() {
            return Ok(());
        }

        self.drift_client.subscribe().await?;
        self.sync(authorities).await;

        Ok(())
    }

    pub async fn add_user_stat(
        authorities: &[Pubkey],
        user_stats_account: Option<UserStatsAccount>,
        skip_fetch: Option<bool>,
    ) {
    }

    pub fn size(&self) -> usize {
        self.user_stats_map.len()
    }

    pub async fn sync(&self, authorities: &[Pubkey]) {}
}
