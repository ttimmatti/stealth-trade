# Stealth Trade – Private DEX on Private Ephemeral Rollups (PER)

A Solana Fast Private Trading Platform that works on Solana with Private Ephemeral Rollup (PER).

## Summary

- [Private Payments](#private-payments)
  - [Summary](#summary)
  - [How it works](#how-it-works)
    - [Private Ephemeral Rollup](#private-ephemeral-rollup)
    - [How do private transfers work on PER?](#how-do-private-transfers-work-on-PER?)
  - [Running the demo](#running-the-demo)
    - [Installation](#installation)
    - [Run the app](#run-the-app)
    - [Usage guide](#usage-guide)
    - [Test the program](#test-the-program)

## How it works

### Private Ephemeral Rollup

A Private Ephemeral Rollup (PER) is a variant of MagicBlock [Ephemeral Rollup](https://docs.magicblock.gg/pages/get-started/introduction/why-magicblock) (ER), that runs the validator inside a Trusted Execution Environment (TEE), specifically Intel Trust Domain Extension (TDX), to provide privacy and auditability.

PER operators can configure a middleware that enables user-specific READ/WRITE permissions on Solana account level:
- **Fine-grained Privacy Control** on individual Solana accounts and account groups
- **Simple Authentication via Session Keys** after signing challenge with private key
- **Customizable Permissions** are stored, enforced, modifiable through on-chain permission program and metadata accounts.


### How does private DEX work on PER?

For operators:
1. **Configure READ permission settings** through on-chain Permission Program
2. **Manage groups** for role-based access control
3. **Apply permissions** to specific program instructions and on individual accounts

For users:
1. **Deposit** tokens into User sub-account on Solana
3. **Delegate** sub-account to private ephemeral rollup
4. **Make a private transfer** to any address on private ephemeral rollup
5. **Make a private swap** with any of the available Liquidity Pools on private ephemeral rollup
6. **Undelegate and withdraw** tokens from User sub-account on Solana

User Authentication Flow:
1. User signs a challenge using their private key for authentication.
2. Middleware verifies the signature and then generates a session key.
3. PER authorize subsequent READ requests based on validity of the session key.

A **Private Ephemeral Rollup** ensures only permissioned users can view an account’s balances, while still allowing anyone to credit it.

A typical interaction will go as follows (illustrated below):

For operators:
1. DEX provider configures on-chain READ permission settings for users.
2. Payment provider enables READ permissions on delegated accounts for specific roles, while users retain READ/WRITE permission on their own delegated accounts.

For users:
1. Bob deposits 100 USDC into his sub-account on Solana
2. Bob conceals his sub-account by delegating to PER
3. Bob trades privately on PER, avoiding MEV and preserving full anonimity for his funds and PNLs
4. Bob sends some of his profits to Alice on PER
5. Alice can continue trading and sending transfers privately on PER until she decides to undelegate some of her tokens and reveal the balances, she can still keep some of her balances private leaving them on her other delegated sub-accounts.


### Test the program

You must have a MagicBlock validator and a test validator running locally: 
1. In MagicBlock validator's repo: `cargo run -- --remote-url http://localhost:8899 --remote-ws-url ws://localhost:8900 --rpc-addr 0.0.0.0 --rpc-port 7799`
2. `solana-test-validator`

```bash
anchor test --skip-local-validator
```
