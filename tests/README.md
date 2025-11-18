# Permission and Delegate Tests (TypeScript)

This directory contains TypeScript versions of the litesvm tests from the `tests-rs/` folder.

## Structure

```
tests/
├── test-utils.ts          # Shared utilities and PDA derivation functions
├── permission-tests.ts    # Permission system tests (TypeScript version of tests-rs/permission-tests.rs)
├── delegate-tests.ts      # Delegation tests (TypeScript version of tests-rs/delegate-tests.rs)
└── README_NEW_TESTS.md    # This file
```

## Files

### `test-utils.ts`
Common utilities for all tests:
- **Constants**: Program IDs, seeds, decimals
- **PDA Functions**:
  - `getGroupPda()` - Derives Group PDA for permission system
  - `getPermissionPda()` - Derives Permission PDA
  - `getUserPda()` - Derives User account PDA
  - `getConfigPda()` - Derives Config PDA
  - `getLiquidityPoolPda()` - Derives LP PDA
  - `getLpMintPda()` - Derives LP Mint PDA

### `permission-tests.ts`
Tests for permission-related instructions:
1. **Creates user permission** - Tests creating individual user permissions
2. **Creates LP permission group** - Tests creating shared permission groups
3. **Creates LP permission** - Tests linking LP to permission groups
4. **Full permission flow** - Integration test covering complete workflow

### `delegate-tests.ts`
Tests for delegation instructions:
1. **Delegates user account** - Tests user account delegation to ER
2. **Delegates user account with deposits** - Tests delegation preserves user balances
3. **Delegates LP account** - Tests LP delegation
4. **Delegates multiple users** - Tests delegating multiple user accounts
5. **Full delegate flow** - Integration test with deposits, LP creation, and delegation

## Running Tests

### Prerequisites
Make sure you have:
1. A local validator running on `localhost:8899`
2. Admin keypair at `tmp/admin.json`
3. Built the program: `anchor build`
4. Deployed the program

### Run Permission Tests Only
```bash
yarn test:permission
# or with bun
bun test:permission
```

### Run Delegate Tests Only
```bash
yarn test:delegate
# or with bun
bun test:delegate
```

### Run Individual Test
```bash
# Permission tests - specific test
yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/permission-tests.ts --grep "Creates user permission"

# Delegate tests - specific test
yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/delegate-tests.ts --grep "Delegates user account"
```

## Test Behavior

### Expected Behavior
- ✅ Tests will create all necessary accounts (users, LPs, etc.)
- ✅ Tests verify state before and after operations
- ⚠️ Permission/delegation operations may fail if the external programs are not loaded
- ✅ Tests handle failures gracefully and report expected failures

### Test Output
Tests will show:
- ✅ Success messages for completed operations
- ⚠️ Warning messages for expected failures (e.g., when permission program not loaded)
- Detailed information about created PDAs and accounts

## Differences from Rust Tests

1. **Language**: TypeScript vs Rust
2. **Framework**: Anchor TypeScript client vs litesvm
3. **Connection**: Uses actual Solana RPC vs in-memory simulation
4. **Structure**: Uses Mocha/Chai vs native Rust test framework
5. **Account Handling**: Uses Anchor's client helpers vs manual instruction building

## Similarities to Rust Tests

1. **Test Coverage**: Same test cases and scenarios
2. **Test Flow**: Identical step-by-step execution
3. **Assertions**: Same validations and checks
4. **PDAs**: Same derivation logic (in test-utils.ts)
5. **Expected Failures**: Both handle missing programs gracefully

## Integration with Existing Tests

These tests are **separate** from the existing tests in the `tests/` directory:
- `private-payments.ts` - Original payment tests
- `session-private-transfers.ts` - Session-based transfer tests
- `private-payments-tee-devnet.ts` - TEE devnet tests

The new tests follow the same structure but focus specifically on:
- Permission system functionality
- Delegation to Ephemeral Rollups

## Troubleshooting

### Tests Fail to Connect
- Ensure local validator is running: `solana-test-validator`
- Check RPC endpoint in the test files (default: `http://localhost:8899`)

### Account Not Found Errors
- Make sure to initialize config before running tests
- The `before()` hook handles this automatically

### Permission/Delegation Failures
- These are **expected** if permission/delegate programs aren't loaded
- Tests will show ⚠️ warnings and continue
- To fully test these features, load the required programs into your validator

### Program ID Mismatch
- Ensure your program is built and deployed
- Check `declare_id!` in your program matches the deployed address
- Verify `tmp/admin.json` exists

## Notes

- Tests use the same admin keypair as Rust tests (`tmp/admin.json`)
- Tests create fresh accounts for each test case
- All accounts are created with sufficient SOL for rent
- Token mints use 6 decimals (configurable in test-utils.ts)

## Future Enhancements

Potential improvements:
- Add more edge case tests
- Add session token integration tests
- Add undelegation tests
- Add permission revocation tests
- Add multi-signature permission tests

