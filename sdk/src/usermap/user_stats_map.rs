use std::{collections::HashMap, time::Duration};

use drift::{state::events::OrderRecord, ID as PROGRAM_ID};
use solana_sdk::pubkey::Pubkey;

use crate::{
    accounts::{BulkAccountLoader, UserStatsAccountSubscriber},
    addresses::pda::get_user_stats_account_pubkey,
    events::types::{EventMap, WrappedEvent},
    types::{SdkResult, UserStatsAccount},
    user_stats::UserStats,
    user_stats_config::{UserStatsConfig, UserStatsSubscriptionConfig},
    AccountProvider, DriftClient,
};

use super::UserMap;

pub struct UserStatsMap<'a, T, U>
where
    T: AccountProvider,
{
    user_stats_map: HashMap<Pubkey, UserStats<'a, T, U>>,
    drift_client: &'a DriftClient<T, U>,
    bulk_account_loader: BulkAccountLoader,
}

impl<'a, T, U> UserStatsMap<'a, T, U>
where
    T: AccountProvider,
{
    pub fn new(
        drift_client: &'a DriftClient<T, U>,
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

    pub async fn subscribe(&mut self, authorities: &[Pubkey]) -> SdkResult<()> {
        if self.user_stats_map.is_empty() {
            return Ok(());
        }

        self.drift_client.subscribe().await?;
        self.sync(authorities).await?;

        Ok(())
    }

    pub async fn add_user_stat(
        &mut self,
        authority: Pubkey,
        user_stats_account: Option<UserStatsAccount>,
        skip_fetch: Option<bool>,
    ) -> SdkResult<()> {
        let mut user_stat = UserStats::new(UserStatsConfig {
            account_subscription: Some(UserStatsSubscriptionConfig::Polling {
                account_loader: self.bulk_account_loader.clone(),
            }),
            drift_client: self.drift_client,
            user_stats_account_public_key: get_user_stats_account_pubkey(&PROGRAM_ID, authority),
        })?;

        if let Some(true) = skip_fetch {
            if let UserStatsAccountSubscriber::Polling(ref mut polling) =
                user_stat.account_subscriber
            {
                polling.add_to_account_loader().await;
            }
        } else {
            user_stat.subscribe(user_stats_account).await?;
        }

        self.user_stats_map.insert(authority, user_stat);

        Ok(())
    }

    pub async fn update_with_other_record(
        &mut self,
        record: OrderRecord,
        user_map: UserMap,
    ) -> SdkResult<()> {
        let user = user_map.must_get(&record.user.to_string()).await?;
        let authority = user.authority;
        if !self.has(&authority) {
            self.add_user_stat(authority, None, Some(false)).await?;
        }

        Ok(())
    }

    pub async fn update_with_event_record<E>(
        &mut self,
        record: WrappedEvent<E>,
        user_map: Option<UserMap>,
    ) -> SdkResult<()> {
        match record.event_type {
            EventMap::DepositRecord(record) => {
                self.must_get(&record.data.user_authority).await?;
            }
            EventMap::FundingPaymentRecord(record) => {
                self.must_get(&record.data.user_authority).await?;
            }
            EventMap::LiquidationRecord(record) => {
                if let Some(user_map) = user_map {
                    let user = user_map.must_get(&record.data.user.to_string()).await?;
                    self.must_get(&user.authority).await?;

                    let liquidatator_user = user_map
                        .must_get(&record.data.liquidator.to_string())
                        .await?;
                    self.must_get(&liquidatator_user.authority).await?;
                }
            }
            EventMap::OrderRecord(record) => {
                if let Some(mut user_map) = user_map {
                    user_map.update_with_order_record(record.data).await?;
                }
            }
            EventMap::OrderActionRecord(record) => {
                if let Some(taker) = record.data.taker {
                    self.must_get(&taker).await?;
                }

                if let Some(maker) = record.data.maker {
                    self.must_get(&maker).await?;
                }
            }
            EventMap::SettlePnlRecord(record) => {
                if let Some(user_map) = user_map {
                    let user = user_map.must_get(&record.data.user.to_string()).await?;
                    self.must_get(&user.authority).await?;
                }
            }
            EventMap::NewUserRecord(record) => {
                self.must_get(&record.data.user_authority).await?;
            }
            EventMap::LPRecord(record) => {
                if let Some(user_map) = user_map {
                    let user = user_map.must_get(&record.data.user.to_string()).await?;
                    self.must_get(&user.authority).await?;
                }
            }
            EventMap::InsuranceFundStakeRecord(record) => {
                self.must_get(&record.data.user_authority).await?;
            }
            _ => {}
        }

        Ok(())
    }

    pub fn has(&self, authority: &Pubkey) -> bool {
        self.user_stats_map.contains_key(authority)
    }

    pub fn get(&self, authority_pubkey: &Pubkey) -> Option<&UserStats<T, U>> {
        self.user_stats_map.get(authority_pubkey)
    }

    /// Enforce that a UserStats will exist for the given authority_pubkey
    pub async fn must_get(&mut self, authority: &Pubkey) -> SdkResult<Option<&UserStats<T, U>>> {
        if !self.has(authority) {
            self.add_user_stat(*authority, None, Some(false)).await?;
        }

        Ok(self.get(authority))
    }

    pub fn size(&self) -> usize {
        self.user_stats_map.len()
    }

    pub async fn sync(&mut self, authorities: &[Pubkey]) -> SdkResult<()> {
        // let mut futures = Vec::new();

        // TODO: use join_all
        for &authority in authorities {
            self.add_user_stat(authority, None, Some(true)).await?;
        }
        // let futures: Vec<_> = authorities
        //     .iter()
        //     .map(|&authority| {
        //         let stat_future = self.add_user_stat(authority, None, Some(true));
        //         async move {
        //             stat_future.await
        //         }
        //     })
        //     .collect();

        // let results = futures_util::future::join_all(futures).await;

        // for result in results {
        //     result?;
        // }

        self.bulk_account_loader.load().await;

        Ok(())
    }

    pub async fn unsubscribe(&mut self) {
        let keys: Vec<Pubkey> = self.user_stats_map.keys().cloned().collect();

        for key in keys {
            if let Some(user_stats) = self.user_stats_map.get_mut(&key) {
                user_stats.unsubscribe().await;
            }
        }
        self.user_stats_map.clear();
    }
}
