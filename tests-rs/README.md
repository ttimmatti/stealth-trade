# Private DEX LiteSVM Tests

This directory contains comprehensive integration tests for the private-dex program using LiteSVM.

## Test Structure

The tests are organized in `private-dex-tests.rs` with the following structure:

### Helper Functions

Reusable functions for calling each program instruction:
- `call_initialize()` - Initialize the config account
- `call_create_user()` - Create a new user account
- `call_create_lp()` - Create a liquidity pool
- `call_deposit()` - Deposit tokens into user positions
- `call_withdraw()` - Withdraw tokens from user positions
- `call_add_liquidity()` - Add liquidity to a pool
- `call_remove_liquidity()` - Remove liquidity from a pool
- `call_swap()` - Swap tokens in a pool
- `call_transfer()` - Transfer tokens between users

### Individual Instruction Tests

Basic tests for each instruction:
1. `test_initialize()` - Tests config initialization
2. `test_create_user()` - Tests user account creation
3. `test_create_lp()` - Tests liquidity pool creation
4. `test_deposit_and_withdraw()` - Tests deposit and withdrawal flow
5. `test_transfer()` - Tests token transfers between users

### Integration Tests

Comprehensive tests combining multiple instructions:
1. `test_full_liquidity_flow()` - Tests the complete liquidity provision workflow:
   - Initialize config
   - Create user
   - Create mints
   - Create LP
   - Deposit tokens
   - Add liquidity
   - Remove liquidity

2. `test_swap_flow()` - Tests the swap workflow:
   - Initialize and setup
   - Create LP with liquidity
   - Execute swap (note: requires pool to be Active status)

3. `test_multi_user_flow()` - Tests multi-user interactions:
   - Create multiple users
   - Deposit, transfer, and withdraw tokens
   - Test cross-user token transfers

## Running the Tests

### Prerequisites

1. Build the program:
```bash
anchor build
```

2. Ensure the program keypair and binary exist:
   - `target/deploy/private_payments-keypair.json`
   - `target/deploy/private_payments.so`

### Run All Tests

```bash
cargo test --test private-dex-tests
```

### Run Specific Test

```bash
cargo test --test private-dex-tests test_initialize
cargo test --test private-dex-tests test_full_liquidity_flow
```

### Run with Output

```bash
cargo test --test private-dex-tests -- --nocapture
```

## Test Coverage

The tests cover:
- ✅ Config initialization
- ✅ User account creation
- ✅ Liquidity pool creation
- ✅ Token deposits and withdrawals
- ✅ Liquidity addition and removal
- ✅ Token transfers between users
- ✅ Swap operations (note: requires pool activation)

## Notes

1. **Pool Status**: The swap test may fail if the pool is in `Paused` status. The pool needs to be activated through an admin action or status change mechanism.

2. **Admin Account**: Tests use the program keypair as the admin account since the ADMIN constant in the program is set to the program ID.

3. **Virtual Reserves**: This program uses virtual reserves for liquidity management, meaning tokens are tracked internally without actual token transfers for LP operations.

4. **Error Handling**: Tests include basic error handling and will print informative messages about failures.

## Extending Tests

To add new tests:

1. Create a helper function for your instruction following the pattern
2. Add individual test cases
3. Consider adding integration tests that combine the new instruction with existing flows

Example:
```rust
#[test]
fn test_my_new_feature() {
    let mut svm = LiteSVM::new();
    // ... setup ...
    call_my_instruction(&mut svm, &program_id, &user, params).unwrap();
    println!("✓ My test passed");
}
```

