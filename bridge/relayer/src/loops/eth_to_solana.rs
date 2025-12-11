use anchor_lang::{AccountDeserialize, InstructionData, declare_program};
use anyhow::{Result, anyhow};
use ethers::core::types::{Address as EvmAddress, U64};
use ethers::providers::{Http, Middleware, Provider};
use ethers::{abi::Abi, contract::Contract};
use solana_client::{rpc_client::RpcClient, rpc_config::CommitmentConfig};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey as SolanaPubkey,
    signature::{Keypair, read_keypair_file},
    signer::Signer,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::ID as TOKEN_PROGRAM_ID;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use crate::BurnedToSolanaEvent;
use crate::abis::EVM_BRIDGE_ABI;

declare_program!(bridging_solana);

use bridging_solana::accounts::BridgeConfig;
use bridging_solana::client::args::UnlockFromEvm as UnlockFromEvmArgs;

pub async fn eth_to_solana_loop() -> Result<()> {
    let evm_rpc_url = std::env::var("EVM_RPC_URL").expect("EVM_RPC_URL missing");
    let bridge_address: EvmAddress = std::env::var("EVM_BRIDGE_ADDRESS")?.parse()?;

    let provider = Arc::new(Provider::<Http>::try_from(evm_rpc_url)?);

    let abi: Abi = serde_json::from_str(EVM_BRIDGE_ABI)?;
    let contract = Contract::new(bridge_address, abi, provider.clone());

    // solana configuration
    let sol_rpc_url = std::env::var("SOLANA_RPC_URL").expect("SOLANA_RPC_URL missing");
    let sol_client = RpcClient::new_with_commitment(sol_rpc_url, CommitmentConfig::confirmed());

    let kp_path = std::env::var("SOLANA_RELAYER_KEYPAIR").expect("SOLANA_RELAYER_KEYPAIR not set");
    let payer: Keypair = read_keypair_file(kp_path).expect("Failed to read relayer keypair");

    let config_pubkey = SolanaPubkey::from_str(
        &std::env::var("BRIDGE_CONFIG_PUBKEY").expect("BRIDGE_CONFIG_PUBKEY not set"),
    )?;

    // Initialize from_block from environment or use current block - 100 as fallback
    let mut from_block = match std::env::var("EVM_START_BLOCK") {
        Ok(block_str) => {
            let block_num: u64 = block_str
                .parse()
                .map_err(|_| anyhow!("EVM_START_BLOCK must be a valid u64"))?;
            U64::from(block_num)
        }
        Err(_) => {
            let current = provider.get_block_number().await?;
            // Use last 100 blocks as fallback to avoid missing events
            let fallback = current.saturating_sub(U64::from(100));
            println!(
                " No EVM_START_BLOCK set, using current block - 100: {}",
                fallback
            );
            fallback
        }
    };

    loop {
        match process_events(
            &contract,
            &sol_client,
            &payer,
            config_pubkey,
            &mut from_block,
        )
        .await
        {
            Ok(()) => {}
            Err(err) => {
                eprintln!("Error processing events: {:?}", err);
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn process_events<M: Middleware>(
    contract: &Contract<M>,
    sol_client: &RpcClient,
    payer: &Keypair,
    config_pubkey: SolanaPubkey,
    from_block: &mut U64,
) -> Result<()>
where
    M: 'static,
{
    let events = contract
        .event::<BurnedToSolanaEvent>()
        .from_block(*from_block)
        .query()
        .await?;

    if !events.is_empty() {
        println!("\n[EVM→Sol] found {} BurnedToSolana events", events.len());
    }

    for (idx, ev) in events.iter().enumerate() {
        println!(
            " Processing event {}/{}: message_id={:?}, nonce={}, amount={}",
            idx + 1,
            events.len(),
            ev.message_id,
            ev.nonce,
            ev.amount
        );

        match submit_unlock_to_solana(sol_client, payer, config_pubkey, ev).await {
            Ok(_) => {
                println!(
                    " Successfully processed event (nonce: {}, amount: {})",
                    ev.nonce, ev.amount
                );
            }
            Err(err) => {
                eprintln!("Failed to process event (nonce: {}): {:?}", ev.nonce, err);
            }
        }
    }

    if !events.is_empty() {
        println!(
            "Processed {} events, updating from_block to {}",
            events.len(),
            from_block
        );
    }

    *from_block = from_block.saturating_add(U64::from(1));

    Ok(())
}

pub async fn submit_unlock_to_solana(
    client: &RpcClient,
    payer: &Keypair,
    config_pubkey: SolanaPubkey,
    ev: &BurnedToSolanaEvent,
) -> Result<()> {
    let config_account = client.get_account(&config_pubkey)?;
    let mut config_data: &[u8] = &config_account.data;
    let config = BridgeConfig::try_deserialize(&mut config_data)
        .map_err(|e| anyhow!("Failed to deserialize BridgeConfig: {:?}", e))?;

    let expected_dst_chain_id: u64 = std::env::var("SOLANA_CHAIN_ID")
        .unwrap_or_else(|_| "0".to_string())
        .parse()
        .unwrap_or(0);

    if ev.dst_chain_id != expected_dst_chain_id {
        return Err(anyhow!(
            "Destination chain ID mismatch: event={}, expected={}",
            ev.dst_chain_id,
            expected_dst_chain_id
        ));
    }

    let event_config_bytes: [u8; 32] = ev.config.to_fixed_bytes();
    let config_pubkey_bytes: [u8; 32] = config_pubkey.to_bytes();
    if event_config_bytes != config_pubkey_bytes {
        return Err(anyhow!(
            "Config mismatch: event config doesn't match bridge config pubkey"
        ));
    }

    let amount_u64: u64 = ev
        .amount
        .try_into()
        .map_err(|_| anyhow!("Amount {} too large for u64", ev.amount))?;

    let sol_recipient = SolanaPubkey::new_from_array(ev.solana_recipient.0);

    // Validate recipient is not zero
    if sol_recipient == SolanaPubkey::default() {
        return Err(anyhow!("Invalid zero recipient address"));
    }

    let program_id = SolanaPubkey::new_from_array(bridging_solana::ID.to_bytes());

    let (vault_authority, _vault_bump) =
        SolanaPubkey::find_program_address(&[b"vault", config_pubkey.as_ref()], &program_id);

    let processed_seeds = &[
        b"processed",
        &ev.src_chain_id.to_le_bytes()[..],
        &ev.nonce.to_le_bytes()[..],
    ];
    let (processed_message_pda, _bump) =
        SolanaPubkey::find_program_address(processed_seeds, &program_id);

    let token_mint_solana = SolanaPubkey::new_from_array(config.token_mint.to_bytes());

    let recipient_token_account = get_associated_token_address(&sol_recipient, &token_mint_solana);
    let token_vault = get_associated_token_address(&vault_authority, &token_mint_solana);

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(config_pubkey, false),
        AccountMeta::new(processed_message_pda, false),
        AccountMeta::new_readonly(vault_authority, false),
        AccountMeta::new(token_vault, false),
        AccountMeta::new(recipient_token_account, false),
        AccountMeta::new_readonly(sol_recipient, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(
            SolanaPubkey::from_str("11111111111111111111111111111111")
                .map_err(|_| anyhow!("Invalid system program address"))?,
            false,
        ),
    ];

    let instruction_data = UnlockFromEvmArgs {
        src_chain_id: ev.src_chain_id,
        nonce: ev.nonce,
        amount: amount_u64,
    };
    let data = instruction_data.data();

    let instruction = Instruction {
        program_id,
        accounts,
        data,
    };

    let recent_blockhash = client.get_latest_blockhash()?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[payer], recent_blockhash);

    let signature = client.send_and_confirm_transaction(&transaction)?;

    println!(
        "Successfully unlocked {} tokens to {} on Solana (tx: {})",
        amount_u64, sol_recipient, signature
    );

    Ok(())
}
