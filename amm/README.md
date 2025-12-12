# AMM Contract

A simple automated market maker built on Solana. Lets you swap tokens, add liquidity to pools, and remove it when you want. Uses the constant product formula (x * y = k) to set prices.

## What it does

This contract lets you:
- Create pools for any two tokens
- Add liquidity and get LP tokens back
- Swap one token for another
- Remove liquidity by burning your LP tokens

The price for swaps comes from the pool's reserves. More of one token in the pool means it's worth less, so you get more of the other token when swapping.

## How it works

### Creating an AMM

First, you create an AMM instance with an admin and a fee. The fee is in basis points (500 = 5%).

### Creating a pool

Once you have an AMM, you can create pools for token pairs. Each pool gets its own LP token mint. The pool stores the two token mints it's for.

### Adding liquidity

When you add liquidity, you deposit both tokens. On the first deposit, the contract uses `sqrt(amount_a * amount_b)` to figure out how many LP tokens to mint. After that, it uses the ratio of your deposit to the existing reserves.

A small amount of liquidity (100 tokens) is locked forever on the first deposit. This prevents someone from draining the pool completely.

### Swapping

To swap, you send one token to the pool and get the other back. The contract:
1. Takes a fee from your input
2. Calculates output using the constant product formula
3. Checks you're getting at least the minimum you asked for (slippage protection)
4. Transfers the tokens

The formula: `(reserve_a + input) * (reserve_b - output) = reserve_a * reserve_b`

### Removing liquidity

Burn your LP tokens to get back your share of both tokens in the pool. The amounts are proportional to how much of the LP supply you're burning.

## Project structure

```
programs/amm/src/
├── lib.rs              # Main program entry point
├── state.rs            # Amm and Pool account structs
├── constants.rs        # Constants like MINIMUM_LIQUIDITY
├── errors.rs           # Custom error types
└── instructions/       # Instruction handlers
    ├── create_amm.rs
    ├── create_pool.rs
    ├── deposit_liquidity.rs
    ├── swap_tokens.rs
    └── withdraw_liquidity.rs
```

## Setup

Make sure you have:
- Rust (latest stable)
- Solana CLI
- Anchor CLI
- Node.js and Yarn

Then:

```bash
yarn install
anchor build
anchor test
```

## Testing

Tests cover creating AMMs, pools, adding/removing liquidity, and swapping. Run them with:

```bash
anchor test
```

## Technical details

### PDAs

The contract uses Program Derived Addresses (PDAs) for:
- The AMM account (stores admin and fee)
- The pool account (stores token pair info)
- The pool authority (signs for token transfers)
- The LP token mint

### State

**Amm account:**
- `id`: Unique identifier
- `admin`: Admin address
- `fee`: Trading fee in basis points

**Pool account:**
- `amm`: Reference to the AMM
- `mint_a`: First token mint
- `mint_b`: Second token mint

### Errors

- `InvalidFee`: Fee is 10000 or higher (100%)
- `InvalidMint`: Wrong mint for the pool
- `DepositTooSmall`: Deposit amount too small
- `OutputTooSmall`: Swap output below minimum
- `InvariantViolated`: Constant product check failed
- `InsufficientLiquidity`: Not enough LP tokens
- `ZeroWithdrawal`: Withdrawal would give zero tokens

## Notes

- The contract uses fixed-point math for precise calculations
- All PDAs are verified using seeds and bumps
- Minimum liquidity prevents complete pool drainage
- Slippage protection lets users set minimum output amounts

ISC License
