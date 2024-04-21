use dotenv::dotenv;
use drift_sdk::{get_market_accounts, types::Context, DriftClient, RpcAccountProvider, Wallet};
use solana_sdk::signature::read_keypair_file;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let rpc = RpcAccountProvider::new("https://api.devnet.solana.com");
    let keypair_path = std::env::var("KEYPAIR_FILE").unwrap();
    let keypair = read_keypair_file(keypair_path).unwrap();
    let client = DriftClient::new(Context::DevNet, rpc, Wallet::new(keypair))
        .await
        .unwrap();

    let (spots, _perps) = get_market_accounts(client.inner()).await.unwrap();

    for spot in spots {
        let name = String::from_utf8(spot.name.to_vec()).unwrap();
        if name == "sol" {
            break;
        }
    }

}
