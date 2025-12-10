use anyhow::Result;

use ethers::contract::EthEvent;
use ethers::types::H256;
use ethers::types::U256;

pub mod abis;
pub mod loops;

use crate::loops::eth_to_solana_loop;
use crate::loops::solana_to_eth_loop;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tokio::try_join!(solana_to_eth_loop(), eth_to_solana_loop())?;

    Ok(())
}

#[derive(Debug, Clone, EthEvent)]
#[ethevent(
    name = "BurnedToSolana",
    abi = "BurnedToSolana(bytes32,uint64,uint64,bytes32,uint64,uint256,bytes32)"
)]
pub struct BurnedToSolanaEvent {
    #[ethevent(indexed)]
    pub message_id: H256,

    pub src_chain_id: u64,
    pub dst_chain_id: u64,

    #[ethevent(indexed)]
    pub config: H256,

    pub nonce: u64,
    pub amount: U256,
    pub solana_recipient: H256,
}
