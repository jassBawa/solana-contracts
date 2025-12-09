pub const EVM_BRIDGE_ABI: &str = r#"[
  {
    "type": "function",
    "name": "mintFromSolana",
    "stateMutability": "nonpayable",
    "inputs": [
      { "name": "srcChainId", "type": "uint64" },
      { "name": "config", "type": "bytes32" },
      { "name": "nonce", "type": "uint64" },
      { "name": "tokenMint", "type": "bytes32" },
      { "name": "solanaUser", "type": "bytes32" },
      { "name": "amount", "type": "uint256" },
      { "name": "recipient", "type": "address" }
    ],
    "outputs": []
  }
]"#;
