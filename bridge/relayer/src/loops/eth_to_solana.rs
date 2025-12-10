use std::sync::Arc;
use std::time::Duration;

use crate::BurnedToSolanaEvent;
use crate::abis::EVM_BRIDGE_ABI;
use anyhow::Result;
use ethers::core::types::Address as EvmAddress;
use ethers::providers::{Http, Middleware, Provider};
use ethers::{abi::Abi, contract::Contract};

pub async fn eth_to_solana_loop() -> Result<()> {
    let rpc_url = std::env::var("ETH_RPC_URL").expect("ETH_RPC_URL missing");
    let bridge_address: EvmAddress = std::env::var("ETH_BRIDGE_ADDRESS")?.parse()?;

    let provider = Arc::new(Provider::<Http>::try_from(rpc_url)?);

    let abi: Abi = serde_json::from_str(EVM_BRIDGE_ABI)?;
    let contract = Contract::new(bridge_address, abi, provider.clone());

    let mut last_block = provider.get_block_number().await?;

    loop {
        let events = contract
            .event::<BurnedToSolanaEvent>()
            .from_block(last_block)
            .query()
            .await?;

        for ev in events {
            println!("messageId = {:?}", ev.message_id);
            println!("nonce = {}", ev.nonce);
            println!("amount = {}", ev.amount);
        }

        last_block += 1.into();

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
