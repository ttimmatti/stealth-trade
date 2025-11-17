mod common;

use common::*;
use private_dex::state::{User, LiquidityPool};
use solana_sdk::signature::{Keypair, Signer};
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo};

// ============================================================================
// Test Cases for Delegate Instructions
// ============================================================================

#[test]
fn test_delegate_user() {
    let mut ctx = TestContext::new();
    
    println!("=== Delegate User Test ===");
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    println!("✅ Config initialized");
    
    // Create a user
    let user = Keypair::new();
    ctx.anchor_ctx.svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
    let user_pda = create_user(&mut ctx, &user).unwrap();
    println!("✅ User created at {}", user_pda);
    
    // Verify user account exists before delegation
    let user_account = ctx.anchor_ctx.get_account::<User>(&user_pda).expect("Failed to get user account");
    assert_eq!(user_account.authority, user.pubkey(), "User should have correct authority");
    println!("✅ User account verified before delegation");
    
    // Delegate the user account
    delegate_user(&mut ctx, &user, user.pubkey(), user_pda).expect("Failed to delegate user account");

    println!("✅ User account delegated successfully");
}

#[test]
fn test_delegate_user_with_deposits() {
    let mut ctx = TestContext::new();
    
    println!("=== Delegate User with Deposits Test ===");
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    println!("✅ Config initialized");
    
    // Create a user
    let user = Keypair::new();
    ctx.anchor_ctx.svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
    let user_pda = create_user(&mut ctx, &user).unwrap();
    println!("✅ User created");
    
    // Create a token mint and mint tokens to the user
    let mint = CreateMint::new(&mut ctx.anchor_ctx.svm, &user)
        .authority(&user.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();
    
    let user_ata = CreateAssociatedTokenAccount::new(&mut ctx.anchor_ctx.svm, &user, &mint)
        .owner(&user.pubkey())
        .send()
        .unwrap();
    
    MintTo::new(&mut ctx.anchor_ctx.svm, &user, &mint, &user_ata, 10_000_000)
        .send()
        .unwrap();
    println!("✅ Tokens minted");
    
    // Deposit tokens
    deposit(&mut ctx, &user, user_pda, mint, 5_000_000).unwrap();
    println!("✅ Tokens deposited: 5,000,000");
    
    // Verify balance before delegation
    let user_account_before = ctx.anchor_ctx.get_account::<User>(&user_pda).expect("Failed to get user account");
    assert_eq!(user_account_before.positions[0].mint, mint, "Position 0 should have mint");
    assert_eq!(user_account_before.positions[0].amount, 5_000_000, "Position 0 should have 5,000,000 tokens");
    
    // Delegate the user account
    delegate_user(&mut ctx, &user, user.pubkey(), user_pda).expect("Failed to delegate user account");

    println!("✅ User account with deposits delegated successfully");
}

#[test]
fn test_delegate_lp() {
    let mut ctx = TestContext::new();
    
    println!("=== Delegate LP Test ===");
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    println!("✅ Config initialized");
    
    // Create a user to create the pool
    let creator = Keypair::new();
    ctx.anchor_ctx.svm.airdrop(&creator.pubkey(), 10_000_000_000).unwrap();
    
    // Create two token mints
    let mint_a = CreateMint::new(&mut ctx.anchor_ctx.svm, &creator)
        .authority(&creator.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();
    
    let mint_b = CreateMint::new(&mut ctx.anchor_ctx.svm, &creator)
        .authority(&creator.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();
    
    // Create liquidity pool
    let (lp, _mint_lp) = create_lp(&mut ctx, &creator, mint_a, mint_b).unwrap();
    println!("✅ Liquidity pool created at {}", lp);
    
    // Verify LP account exists before delegation
    let lp_account = ctx.anchor_ctx.get_account::<LiquidityPool>(&lp).expect("Failed to get LP account");
    assert_eq!(lp_account.mint_a, mint_a, "LP should have correct mint_a");
    assert_eq!(lp_account.mint_b, mint_b, "LP should have correct mint_b");
    println!("✅ LP account verified before delegation");
    
    // Delegate the LP account
    delegate_lp(&mut ctx, &creator, lp).expect("Failed to delegate LP account");
    
    println!("✅ LP account delegated successfully");
}

#[test]
fn test_delegate_lp_with_liquidity() {
    let mut ctx = TestContext::new();
    
    println!("=== Delegate LP with Liquidity Test ===");
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    println!("✅ Config initialized");
    
    // Create a user
    let user = Keypair::new();
    ctx.anchor_ctx.svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
    let user_pda = create_user(&mut ctx, &user).unwrap();
    println!("✅ User created");
    
    // Create two token mints
    let mint_a = CreateMint::new(&mut ctx.anchor_ctx.svm, &user)
        .authority(&user.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();
    
    let mint_b = CreateMint::new(&mut ctx.anchor_ctx.svm, &user)
        .authority(&user.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();
    
    // Mint and deposit tokens
    let user_ata_a = CreateAssociatedTokenAccount::new(&mut ctx.anchor_ctx.svm, &user, &mint_a)
        .owner(&user.pubkey())
        .send()
        .unwrap();
    let user_ata_b = CreateAssociatedTokenAccount::new(&mut ctx.anchor_ctx.svm, &user, &mint_b)
        .owner(&user.pubkey())
        .send()
        .unwrap();
    
    MintTo::new(&mut ctx.anchor_ctx.svm, &user, &mint_a, &user_ata_a, 10_000_000)
        .send()
        .unwrap();
    MintTo::new(&mut ctx.anchor_ctx.svm, &user, &mint_b, &user_ata_b, 10_000_000)
        .send()
        .unwrap();
    
    deposit(&mut ctx, &user, user_pda, mint_a, 5_000_000).unwrap();
    deposit(&mut ctx, &user, user_pda, mint_b, 5_000_000).unwrap();
    println!("✅ Tokens deposited");
    
    // Create liquidity pool
    let (lp, mint_lp) = create_lp(&mut ctx, &user, mint_a, mint_b).unwrap();
    println!("✅ LP created");
    
    // Add liquidity
    add_liquidity(
        &mut ctx,
        &user,
        user_pda,
        mint_a,
        mint_b,
        lp,
        mint_lp,
        1_000_000,
        2_000_000,
        2_000_000,
    )
    .unwrap();
    println!("✅ Liquidity added");
    
    // Verify LP state before delegation
    let lp_account_before = ctx.anchor_ctx.get_account::<LiquidityPool>(&lp).expect("Failed to get LP account");
    assert_eq!(lp_account_before.virtual_reserve_a, 2_000_000, "LP should have reserve_a");
    assert_eq!(lp_account_before.virtual_reserve_b, 2_000_000, "LP should have reserve_b");
    assert_eq!(lp_account_before.lp_supply, 1_000_000, "LP should have supply");
    
    // Delegate the LP account
    delegate_lp(&mut ctx, &user, lp).expect("Failed to delegate LP account");
    
    println!("✅ LP account delegated successfully");
}

#[test]
fn test_delegate_multiple_users() {
    let mut ctx = TestContext::new();
    
    println!("=== Delegate Multiple Users Test ===");
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    println!("✅ Config initialized");
    
    // Create multiple users
    let user1 = Keypair::new();
    let user2 = Keypair::new();
    let user3 = Keypair::new();
    ctx.anchor_ctx.svm.airdrop(&user1.pubkey(), 10_000_000_000).unwrap();
    ctx.anchor_ctx.svm.airdrop(&user2.pubkey(), 10_000_000_000).unwrap();
    ctx.anchor_ctx.svm.airdrop(&user3.pubkey(), 10_000_000_000).unwrap();
    
    let user1_pda = create_user(&mut ctx, &user1).unwrap();
    let user2_pda = create_user(&mut ctx, &user2).unwrap();
    let user3_pda = create_user(&mut ctx, &user3).unwrap();
    println!("✅ 3 users created");
    
    // Delegate all users
    delegate_user(&mut ctx, &user1, user1.pubkey(), user1_pda).expect("Failed to delegate user 1 account");
    delegate_user(&mut ctx, &user2, user2.pubkey(), user2_pda).expect("Failed to delegate user 2 account");
    delegate_user(&mut ctx, &user3, user3.pubkey(), user3_pda).expect("Failed to delegate user 3 account");

    println!("✅ All 3 users delegated successfully");
}

#[test]
fn test_full_delegate_flow() {
    let mut ctx = TestContext::new();
    
    println!("=== Full Delegate Flow Test ===");
    
    // 1. Initialize
    initialize_config(&mut ctx).unwrap();
    println!("✅ 1. Config initialized");
    
    // 2. Create users
    let user1 = Keypair::new();
    let user2 = Keypair::new();
    ctx.anchor_ctx.svm.airdrop(&user1.pubkey(), 10_000_000_000).unwrap();
    ctx.anchor_ctx.svm.airdrop(&user2.pubkey(), 10_000_000_000).unwrap();
    
    let user1_pda = create_user(&mut ctx, &user1).unwrap();
    let user2_pda = create_user(&mut ctx, &user2).unwrap();
    println!("✅ 2. Users created");
    
    // 3. Create tokens
    let mint_a = CreateMint::new(&mut ctx.anchor_ctx.svm, &user1)
        .authority(&user1.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();
    
    let mint_b = CreateMint::new(&mut ctx.anchor_ctx.svm, &user1)
        .authority(&user1.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();
    println!("✅ 3. Token mints created");
    
    // 4. Mint tokens to users
    let user1_ata_a = CreateAssociatedTokenAccount::new(&mut ctx.anchor_ctx.svm, &user1, &mint_a)
        .owner(&user1.pubkey())
        .send()
        .unwrap();
    let user1_ata_b = CreateAssociatedTokenAccount::new(&mut ctx.anchor_ctx.svm, &user1, &mint_b)
        .owner(&user1.pubkey())
        .send()
        .unwrap();
    
    MintTo::new(&mut ctx.anchor_ctx.svm, &user1, &mint_a, &user1_ata_a, 10_000_000)
        .send()
        .unwrap();
    MintTo::new(&mut ctx.anchor_ctx.svm, &user1, &mint_b, &user1_ata_b, 10_000_000)
        .send()
        .unwrap();
    println!("✅ 4. Tokens minted");
    
    // 5. Deposit tokens
    deposit(&mut ctx, &user1, user1_pda, mint_a, 5_000_000).unwrap();
    deposit(&mut ctx, &user1, user1_pda, mint_b, 5_000_000).unwrap();
    println!("✅ 5. Tokens deposited");
    
    // 6. Create LP
    let (lp, mint_lp) = create_lp(&mut ctx, &user1, mint_a, mint_b).unwrap();
    println!("✅ 6. Liquidity pool created");
    
    // 7. Add liquidity
    add_liquidity(
        &mut ctx,
        &user1,
        user1_pda,
        mint_a,
        mint_b,
        lp,
        mint_lp,
        1_000_000,
        3_000_000,
        3_000_000,
    )
    .unwrap();
    println!("✅ 7. Liquidity added");
    
    // 8. Delegate user accounts
    delegate_user(&mut ctx, &user1, user1.pubkey(), user1_pda).expect("Failed to delegate user 1 account");
    delegate_user(&mut ctx, &user2, user2.pubkey(), user2_pda).expect("Failed to delegate user 2 account");

    println!("✅ 8. User accounts delegated");
    
    // 9. Delegate LP account
    delegate_lp(&mut ctx, &user1, lp).expect("Failed to delegate LP account");
    
    println!("✅ 9. LP account delegated");
}
