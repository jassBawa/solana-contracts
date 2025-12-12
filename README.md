# Solana Contracts

A collection of Solana programs I built to learn how different types of contracts work on Solana. Each contract has tests so you can see how everything fits together.

## What's in here

### AMM Contract (`amm/`)

An automated market maker that lets you swap tokens, add liquidity, and remove it. Uses the constant product formula (x * y = k) to set prices.

- Create pools for token pairs
- Add liquidity and get LP tokens
- Swap tokens
- Remove liquidity

See the [AMM README](./amm/README.md) for details.

### Bridge Contract (`bridge/`)

A two-way bridge between Solana and EVM chains. Lock tokens on Solana to get wrapped tokens on EVM, or burn wrapped tokens on EVM to unlock the originals back on Solana.

- Solana program for locking/unlocking tokens
- EVM contract for minting/burning wrapped tokens
- Relayer service that watches both chains

See the [Bridge README](./bridge/README.md) for details.

## Getting started

Each contract is in its own folder with its own setup. Check the README in each folder for specific instructions.

General requirements:
- Rust (latest stable)
- Solana CLI
- Anchor framework
- Node.js and Yarn (for tests)

## Testing

All contracts include test suites. Run them from each contract's directory:

```bash
cd amm
anchor test

cd bridge/bridging-solana
anchor test
```

## Why I built these

These are practice projects to understand:
- How Solana programs work
- PDAs and account management
- Token operations with SPL
- Cross-chain interactions
- Testing Solana programs

Each contract has tests that show how to interact with it, which is helpful when learning.

