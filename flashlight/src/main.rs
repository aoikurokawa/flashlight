use std::{env, time::Duration};

use clap::{Parser, Subcommand};
use dotenv::dotenv;
use flashlight::{config::BaseBotConfig, funding_rate_updater::FundingRateUpdaterBot};
use sdk::{
    types::Context, utils::load_keypair_multi_format, DriftClient, RpcAccountProvider, Wallet,
};

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
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    dotenv().ok();
    env_logger::init();

    let endpoint = env::var("RPC_URL").expect("RPC_URL must be set");
    let private_key = env::var("PRIVATE_KEY").expect("SECRET_KEY must be set");
    let wallet = Wallet::new(load_keypair_multi_format(&private_key).expect("valid keypair"));
    let account_provider = RpcAccountProvider::new(&endpoint);

    let drift_client: DriftClient<RpcAccountProvider, u16> =
        DriftClient::new(Context::DevNet, account_provider, wallet)
            .await
            .expect("fail to construct drift client");

    match cli.command {
        Commands::InitUser {} => {
            // let keypair = read_keypair_file(&private_key).expect("read keypair");
            // let secret_key_slices = keypair.secret().to_bytes();
            // let key = bs58::encode(keypair.secret()).into_string();
            // let key = String::from_utf8(secret_key_slices.to_vec()).unwrap();

            // println!("{:?}", wallet.signer());
        }
        Commands::Jit {} => {}
        Commands::Filler {} => {}
        Commands::FundingRateUpdater {} => {
            let config = BaseBotConfig {
                bot_id: "funding_rate_updater".to_string(),
                dry_run: true,
                metrics_port: Some(9465),
                run_once: Some(true),
            };

            let mut bot: FundingRateUpdaterBot<RpcAccountProvider, _> =
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
    }
}
