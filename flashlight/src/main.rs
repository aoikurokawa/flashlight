use std::{env, sync::Arc, time::Duration};

use clap::{Parser, Subcommand};
use dotenv::dotenv;
use flashlight::{
    config::{BaseBotConfig, FillerConfig, GlobalConfig},
    filler::FillerBot,
    funding_rate_updater::FundingRateUpdaterBot,
    metrics::RuntimeSpec,
    trigger::TriggerBot,
};
use log::info;
use sdk::{
    blockhash_subscriber::BlockhashSubscriber,
    drift_client::DriftClient,
    priority_fee::{
        priority_fee_subscriber::PriorityFeeSubscriber, types::PriorityFeeSubscriberConfig,
    },
    slot_subscriber::SlotSubscriber,
    types::Context,
    usermap::UserMap,
    utils::load_keypair_multi_format,
    RpcAccountProvider, Wallet,
};
use solana_sdk::commitment_config::CommitmentConfig;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize user
    InitUser {},

    /// Just In Time Auction Bot
    Jit {},

    /// Order Matching Bot
    Filler {},

    /// Enable Funding Rate updater bot
    FundingRateUpdater {},

    /// Enable Triggering bot
    Trigger {},
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    dotenv().ok();
    env_logger::init();

    let endpoint = env::var("RPC_URL").expect("RPC_URL must be set");
    let private_key = env::var("PRIVATE_KEY").expect("SECRET_KEY must be set");
    let websocket_url = env::var("WEBSOCKET_URL").expect("WEBSOCKET_URL must be set");
    let wallet = Wallet::new(load_keypair_multi_format(&private_key).expect("valid keypair"));
    let account_provider = RpcAccountProvider::new(&endpoint);

    let mut drift_client: DriftClient<RpcAccountProvider> =
        DriftClient::new(Context::DevNet, account_provider, &wallet)
            .await
            .expect("fail to construct drift client");
    drift_client.add_user(0).await.expect("add user");
    drift_client
        .subscribe()
        .await
        .expect("drift client subscribing");

    let mut slot_subscriber = SlotSubscriber::new(&websocket_url);
    slot_subscriber.subscribe().await.expect("subscribing slot");

    let lamports_balance = drift_client
        .backend
        .rpc_client
        .get_balance(&wallet.authority())
        .await
        .expect("get balance");

    info!("Wallet pubkey: {}", &wallet.authority());
    info!("SOL balance: {}", lamports_balance / 10 * 9);

    match cli.command {
        Commands::InitUser {} => {
            // let keypair = read_keypair_file(&private_key).expect("read keypair");
            // let secret_key_slices = keypair.secret().to_bytes();
            // let key = bs58::encode(keypair.secret()).into_string();
            // let key = String::from_utf8(secret_key_slices.to_vec()).unwrap();

            // println!("{:?}", wallet.signer());
        }
        Commands::Jit {} => {}
        Commands::Filler {} => {
            let drift_client = Arc::new(drift_client);
            let base_config = BaseBotConfig {
                bot_id: "filler".to_string(),
                dry_run: true,
                metrics_port: Some(9465),
                run_once: Some(true),
            };

            let mut user_map = UserMap::new(CommitmentConfig::confirmed(), &endpoint, true, None);
            user_map.subscribe().await.expect("subscribing usermap");
            let priority_fee_subscriber =
                PriorityFeeSubscriber::new(PriorityFeeSubscriberConfig::new(drift_client.clone()))
                    .expect("construct PriorityFeeSubscriber");

            let mut bot = FillerBot::new(
                &websocket_url,
                slot_subscriber,
                None,
                drift_client.clone(),
                user_map,
                RuntimeSpec::default(),
                GlobalConfig::default(),
                FillerConfig {
                    base_config,
                    ..FillerConfig::default()
                },
                priority_fee_subscriber,
                BlockhashSubscriber::new(60, endpoint),
                None,
            )
            .await;

            bot.init().await;

            bot.start_interval_loop().await;
        }
        Commands::FundingRateUpdater {} => {
            let config = BaseBotConfig {
                bot_id: "funding_rate_updater".to_string(),
                dry_run: true,
                metrics_port: Some(9465),
                run_once: Some(true),
            };

            let mut bot: FundingRateUpdaterBot<RpcAccountProvider> =
                FundingRateUpdaterBot::new(drift_client, config);
            if let Err(e) = bot.init().await {
                println!("{e}");
            }

            if let Err(e) = bot
                .start_interval_loop(Duration::from_secs(2).as_millis() as u64)
                .await
            {
                println!("{e}");
            }
        }
        Commands::Trigger {} => {
            let config = BaseBotConfig {
                bot_id: "trigger".to_string(),
                dry_run: true,
                metrics_port: Some(9465),
                run_once: Some(true),
            };

            let mut user_map = UserMap::new(CommitmentConfig::confirmed(), &endpoint, true, None);
            user_map.subscribe().await.expect("subscribing usermap");

            let mut bot: TriggerBot =
                TriggerBot::new(Arc::new(drift_client), slot_subscriber, user_map, config);
            if let Err(e) = bot.init().await {
                println!("{e}");
            }

            bot.start_interval_loop().await;
        }
    }
}
