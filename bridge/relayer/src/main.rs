use anchor_lang::AccountDeserialize;
use anchor_lang::declare_program;

use anyhow::{Result, anyhow};

use ethers::{
    abi::Abi,
    contract::Contract,
    core::types::Address as EvmAddress,
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::U256,
};

use solana_client::{rpc_client::RpcClient, rpc_config::CommitmentConfig};
use solana_sdk::pubkey::Pubkey;

use std::str::FromStr;
use std::sync::Arc;

pub mod abis;

declare_program!(bridging_solana);

use bridging_solana::accounts::BridgeConfig;

use crate::abis::EVM_BRIDGE_ABI;
use crate::bridging_solana::accounts::LockRecord;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let rpc_url = std::env::var("SOLANA_RPC_URL").expect("SOLANA_RPC_URL not set");
    let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Track last processed nonce in memory for now
    //TODO: we should have a redis or db service to store this for backup.
    let mut last_processed_nonce: u64 = 0;

    loop {
        print!("Hello from process_new_locks, {}", last_processed_nonce);
        match process_new_locks(&rpc_client, &mut last_processed_nonce).await {
            Ok(_) => tokio::time::sleep(std::time::Duration::from_secs(10)).await,
            Err(err) => {
                eprintln!("Some error came: {}", err);
            }
        }
    }
}

async fn process_new_locks(rpc: &RpcClient, last_processed_nonce: &mut u64) -> Result<()> {
    let config_pubkey = Pubkey::from_str(
        &std::env::var("BRIDGE_CONFIG_PUBKEY").expect("BRIDGE_CONFIG_PUBKEY not set"),
    )?;
    let config_account = rpc.get_account(&config_pubkey)?;
    let mut config_account_data: &[u8] = &config_account.data;

    let config = BridgeConfig::try_deserialize(&mut config_account_data)
        .map_err(|e| anyhow!("Failed to deserialize BridgeConfig: {:?}", e))?;

    let current_nonce = config.nonce;

    if current_nonce == *last_processed_nonce {
        println!("No new nonce found");
        return Ok(());
    }

    println!(
        "last processed nonce --> {}  current_nonce --> {}",
        last_processed_nonce, current_nonce
    );

    let program_id = Pubkey::new_from_array(bridging_solana::ID.to_bytes());

    for nonce in *last_processed_nonce..current_nonce {
        // fetching the lockrecord
        let seeds = &[b"lock", config_pubkey.as_ref(), &nonce.to_le_bytes()];
        let (lock_pda, _bump) = Pubkey::find_program_address(seeds, &program_id);

        match rpc.get_account(&lock_pda) {
            Ok(lock_account) => {
                let mut lock_account_slice: &[u8] = &lock_account.data;
                let lock_record = LockRecord::try_deserialize(&mut lock_account_slice)
                    .map_err(|e| anyhow!("Failed to deserialize LockRecord: {:?}", e))?;

                println!("  lock_pda: {}", lock_pda);
                println!("  nonce: {}", lock_record.nonce);
                println!("  user: {}", lock_record.user);
                println!("  amount: {}", lock_record.amount);

                let msg = build_bridge_message(&config, &lock_record, config_pubkey);

                if let Err(err) = submit_to_evm(&msg).await {
                    eprintln!("Error submitting to EVM: {:#}", err);
                }

                *last_processed_nonce = nonce + 1;
            }
            Err(err) => {
                eprintln!(
                    " Could not fetch LockRecord for nonce {} at {}: {:?}",
                    nonce, lock_pda, err
                );
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
struct BridgeMessage {
    pub src_chain_id: u64,
    pub config: [u8; 32],
    pub nonce: u64,
    pub token_mint: [u8; 32],
    pub user: [u8; 32],
    pub amount: u64,
    pub destination_address: [u8; 20],
}

fn build_bridge_message(
    config: &BridgeConfig,
    lock: &LockRecord,
    config_pubkey: Pubkey,
) -> BridgeMessage {
    // this source_chain_id should come from env
    let source_chain_id: u64 = std::env::var("SRC_CHAIN_ID")
        .expect("SRC_CHAIN_ID not set")
        .parse()
        .expect("SRC_CHAIN_ID must be a u64");

    BridgeMessage {
        src_chain_id: source_chain_id,
        config: config_pubkey.to_bytes(),
        nonce: lock.nonce,
        token_mint: config.token_mint.to_bytes(),
        user: lock.user.to_bytes(),
        amount: lock.amount,
        destination_address: config.destination_bridge,
    }
}

async fn submit_to_evm(msg: &BridgeMessage) -> Result<()> {
    println!("Submitting to EVM: {msg:#?}");

    let rpc_url = std::env::var("EVM_RPC_URL")?;
    let private_key = std::env::var("EVM_PRIVATE_KEY")?;
    let bridge_address = std::env::var("EVM_BRIDGE_ADDRESS")?;
    let chain_id: u64 = std::env::var("EVM_CHAIN_ID")?.parse()?;

    let provider = Provider::<Http>::try_from(rpc_url)?;
    let wallet: LocalWallet = private_key.parse()?;
    let wallet = wallet.with_chain_id(chain_id);

    // Signermiddleware will handle this thigns autoamaticallly
    // 1. Build correct calldata
    // 2. Choose correct nonce
    // 3. Estimate gas
    // 4. Set chain ID
    // 5.Sign tx
    // 6. Submit tx
    // 7. Track confirmation
    // 8. Retry safely
    // wallet.sign_transaction() does only 5(sign)
    let client = Arc::new(SignerMiddleware::new(provider, wallet));

    // why not using things like this wallet.sign_transaction

    let bridge_address: EvmAddress = bridge_address.parse()?;
    let abi: Abi = serde_json::from_str(EVM_BRIDGE_ABI)?;
    let bridge = Contract::new(bridge_address, abi, client);

    let tx = bridge.method::<_, ()>(
        "mintFromSolana",
        (
            msg.src_chain_id,
            msg.config,
            msg.nonce,
            msg.token_mint,
            msg.user,
            U256::from(msg.amount),
            EvmAddress::from(msg.destination_address),
        ),
    )?;

    // this only send the tx onchain
    let pending_tx = tx.send().await?;

    // await until the transaction is not failed or processed onchain
    let receipt = pending_tx.await?;

    println!("Etherem tx confirmed: {:?}", receipt);
    Ok(())
}
