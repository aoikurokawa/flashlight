use std::env;

use clap::{Parser, Subcommand};
use dotenv::dotenv;
use sdk::utils::load_keypair_multi_format;
use solana_sdk::commitment_config::CommitmentConfig;
use tokio::sync::Mutex;

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
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    dotenv().ok();
    env_logger::init();

    let endpoint = env::var("RPC_URL").expect("RPC_URL must be set");
    let private_key = env::var("PRIVATE_KEY").expect("SECRET_KEY must be set");
    let wallet = sdk::Wallet::new(load_keypair_multi_format(&private_key).expect("valid keypair"));

    match cli.command {
        Commands::InitUser {} => {
            // let keypair = read_keypair_file(&private_key).expect("read keypair");
            // let secret_key_slices = keypair.secret().to_bytes();
            // let key = bs58::encode(keypair.secret()).into_string();
            // let key = String::from_utf8(secret_key_slices.to_vec()).unwrap();

            println!("{:?}", wallet.signer());
        }
        Commands::Jit {} => {}
        Commands::Filler {} => {}
    }
}
