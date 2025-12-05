# Automated Market Maker (AMM) on Solana

A decentralized exchange (DEX) implementation built on Solana using Anchor framework. This AMM enables token swaps, liquidity provision, and liquidity removal through a constant product formula (x * y = k).

## Overview

This AMM contract implements a Uniswap V2-style automated market maker on Solana. Users can:
- Create liquidity pools for token pairs
- Add liquidity to pools and receive LP (Liquidity Provider) tokens
- Swap tokens at prices determined by the constant product formula
- Remove liquidity by burning LP tokens

## Architecture

### Program Structure

The program is organized into several modules:

- **State**: Defines the on-chain data structures (`Amm`, `Pool`)
- **Instructions**: Contains all program instructions
- **Constants**: Shared constants (seeds, minimum liquidity)
- **Errors**: Custom error types

### Key Design Decisions

#### PDA-Based Architecture

The contract uses Program Derived Addresses (PDAs) for deterministic account generation:

- **AMM Account**: Stores AMM configuration (admin, fee)
- **Pool Account**: Stores pool state (token pair, AMM reference)
- **Pool Authority PDA**: Separate PDA used as authority for token accounts and LP mint
- **Liquidity Mint PDA**: The LP token mint for each pool

#### Why Separate Pool Authority?

The `pool` account is a data account that stores state, while `pool_authority` is a PDA used exclusively for signing transactions. This separation provides:

1. **Clear separation of concerns**: Data storage vs. signing authority
2. **Flexibility**: Pool data can be read without requiring mutability for signing
3. **Security**: Different PDAs for different purposes reduces attack surface
4. **Best practices**: Follows Solana's recommended patterns for program design

Both are PDAs, but serve different roles:
- `pool`: PDA that stores data (amm, mint_a, mint_b)
- `pool_authority`: PDA used only for signing (no data storage)

## Features

### 1. Create AMM

Initializes a new AMM instance with an admin and trading fee.

```rust
create_amm(id: Pubkey, fee: u16)
```

- `id`: Unique identifier for the AMM
- `fee`: Trading fee in basis points (e.g., 500 = 5%)

### 2. Create Pool

Creates a new liquidity pool for a token pair.

```rust
create_pool()
```

This instruction:
- Initializes the pool account with token pair information
- Creates the LP token mint with pool authority
- Sets up token accounts for pool reserves
- Derives all necessary PDAs

### 3. Deposit Liquidity

Add liquidity to a pool and receive LP tokens.

```rust
deposit_liquidity(amount_a: u64, amount_b: u64)
```

**Initial Deposit:**
- Uses geometric mean formula: `liquidity = sqrt(amount_a * amount_b) - MINIMUM_LIQUIDITY`
- Minimum liquidity is locked forever to prevent complete pool drainage

**Subsequent Deposits:**
- Calculates liquidity based on proportional share: `min(liquidity_from_a, liquidity_from_b)`
- Automatically adjusts amounts to maintain pool ratio
- Mints LP tokens to the depositor

### 4. Swap Tokens

Swap tokens using the constant product formula.

```rust
swap_tokens(swap_a: bool, input_amount: u64, min_output_amount: u64)
```

**Formula:** `(x + Δx) * (y - Δy) = x * y`

- Applies trading fee to input amount
- Calculates output using constant product formula
- Validates minimum output amount (slippage protection)
- Transfers tokens between user and pool

### 5. Withdraw Liquidity

Remove liquidity by burning LP tokens and receiving underlying tokens.

```rust
withdraw_liquidity(amount: u64)
```

- Calculates proportional share of pool reserves
- Transfers tokens from pool to user
- Burns LP tokens
- Validates that withdrawal amounts are non-zero

## Technologies

### Core Stack

- **Anchor Framework** (v0.32.1): Solana program framework
- **Rust**: Program language
- **TypeScript**: Test suite
- **Solana SPL Token**: Token program integration

### Key Libraries

- `anchor-lang`: Core Anchor macros and types
- `anchor-spl`: SPL token program integration
- `fixed`: Fixed-point arithmetic for precise calculations
- `@coral-xyz/anchor`: Anchor TypeScript client
- `@solana/spl-token`: SPL token utilities

### Testing

- **Mocha**: Test framework
- **Chai**: Assertion library
- **ts-mocha**: TypeScript support for Mocha

## State Structure

### Amm Account

```rust
pub struct Amm {
    pub id: Pubkey,      // Unique AMM identifier
    pub admin: Pubkey,   // Admin address
    pub fee: u16,        // Trading fee in basis points
}
```

### Pool Account

```rust
pub struct Pool {
    pub amm: Pubkey,     // Reference to AMM
    pub mint_a: Pubkey,  // First token mint
    pub mint_b: Pubkey,  // Second token mint
}
```

## Constants

- `MINIMUM_LIQUIDITY`: 100 (locked forever on first deposit)
- `AUTHORITY_SEED`: "authority" (for pool authority PDA)
- `LIQUIDITY_SEED`: "liquidity" (for LP mint PDA)

## Error Handling

The program defines custom errors for better debugging:

- `InvalidFee`: Fee value is invalid
- `InvalidMint`: Invalid mint for the pool
- `DepositTooSmall`: Deposit amount too small
- `OutputTooSmall`: Swap output below minimum
- `InvariantViolated`: Constant product invariant violated
- `InsufficientLiquidity`: Not enough LP tokens
- `ZeroWithdrawal`: Withdrawal would result in zero tokens

## Setup

### Prerequisites

- Rust (latest stable)
- Solana CLI (v1.18+)
- Anchor CLI (v0.32.1+)
- Node.js and Yarn

### Installation

```bash
# Install dependencies
yarn install

# Build the program
anchor build

# Run tests
anchor test
```

## Testing

The test suite covers all major functionality:

- AMM creation and validation
- Pool creation with invalid mint checks
- Liquidity deposits (equal amounts, zero amount validation)
- Token swaps with slippage protection
- Liquidity withdrawal

Run tests with:

```bash
anchor test
```

## Security Considerations

1. **Minimum Liquidity**: Prevents complete pool drainage
2. **Slippage Protection**: Users can specify minimum output amounts
3. **Ratio Validation**: Deposits automatically adjust to maintain pool ratio
4. **PDA Verification**: All PDAs are verified using seeds and bumps
5. **Access Control**: Only authorized accounts can perform operations

## Mathematical Formulas

### Constant Product Formula

For swaps: `(x + Δx) * (y - Δy) = x * y`

Where:
- `x`, `y`: Current pool reserves
- `Δx`: Input amount (after fee)
- `Δy`: Output amount

### Liquidity Calculation

**Initial deposit:**
```
liquidity = sqrt(amount_a * amount_b) - MINIMUM_LIQUIDITY
```

**Subsequent deposits:**
```
liquidity_from_a = (amount_a * lp_supply) / reserve_a
liquidity_from_b = (amount_b * lp_supply) / reserve_b
liquidity = min(liquidity_from_a, liquidity_from_b)
```

**Withdrawal:**
```
amount_a = (lp_amount * reserve_a) / (lp_supply + MINIMUM_LIQUIDITY)
amount_b = (lp_amount * reserve_b) / (lp_supply + MINIMUM_LIQUIDITY)
```

## License

ISC

