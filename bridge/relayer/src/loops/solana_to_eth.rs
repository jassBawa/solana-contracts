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

use crate::abis::EVM_BRIDGE_ABI;
use bridging_solana::accounts::{BridgeConfig, LockRecord};

declare_program!(bridging_solana);

pub async fn solana_to_eth_loop() -> Result<()> {
    let rpc_url = std::env::var("SOLANA_RPC_URL").expect("SOLANA_RPC_URL not set");
    let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let mut last_processed_nonce = 0;

    loop {
        if let Err(err) = process_new_locks(&rpc_client, &mut last_processed_nonce).await {
            eprintln!("[Sol→EVM] error: {}", err);
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
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
        let seeds = &[b"lock", config_pubkey.as_ref(), &nonce.to_le_bytes()];
        let (lock_pda, _) = Pubkey::find_program_address(seeds, &program_id);

        let lock_account = rpc.get_account(&lock_pda)?;
        let mut data: &[u8] = &lock_account.data;
        let lock = LockRecord::try_deserialize(&mut data)?;

        let msg = build_bridge_message(&config, &lock, config_pubkey);

        match submit_to_evm(&msg).await {
            Ok(_) => {
                *last_processed_nonce += 1;
            }
            Err(err) => {
                eprintln!(
                    "[Sol→EVM] failed at nonce {} — retrying later: {:?}",
                    nonce, err
                );
                break;
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

    // why not using things like this wallet.sign_transaction because
    // Signermiddleware will handle this thigns autoamaticallly
    let client = Arc::new(SignerMiddleware::new(provider, wallet));

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
