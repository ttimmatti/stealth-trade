mod common;

use common::*;
use anchor_spl::token_interface::TokenAccount;
use private_dex::{constants::MAX_POSITIONS, state::{Config, LiquidityPool, LiquidityPoolStatus, User}};
use solana_sdk::{pubkey::Pubkey, signature::{Keypair, Signer}};
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo};
use spl_associated_token_account::get_associated_token_address;

// ============================================================================
// Test Cases
// ============================================================================

#[test]
fn test_initialize() {
    let mut ctx = TestContext::new();
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    
    // Verify the config was created correctly
    let config_account = ctx.anchor_ctx.get_account::<Config>(&ctx.config).expect("Failed to get config account");
    assert!(config_account.admin.eq(&ctx.admin.pubkey()), "Config account should have admin");
    assert!(!config_account.paused, "Config account should not be paused");
    assert_eq!(config_account.default_pool_fee_bps, 100, "Config account should have default pool fee bps");
    
    println!("✅ Config initialized successfully");
}

#[test]
fn test_update_config() {
    let mut ctx = TestContext::new();
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    
    // Update config to pause the protocol
    update_config(&mut ctx, Some(true), None).unwrap();

    let config_account = ctx.anchor_ctx.get_account::<Config>(&ctx.config).expect("Failed to get config account");
    assert!(config_account.paused, "Config account should be paused");

    println!("✅ Config updated successfully");
    
    // Update config to unpause and change fee
    update_config(&mut ctx, Some(false), Some(200)).unwrap();

    let config_account = ctx.anchor_ctx.get_account::<Config>(&ctx.config).expect("Failed to get config account");
    assert!(!config_account.paused, "Config account should not be paused");
    assert_eq!(config_account.default_pool_fee_bps, 200, "Config account should have default pool fee bps");
    
    println!("✅ Config updated again successfully");
}

#[test]
fn test_create_user() {
    let mut ctx = TestContext::new();
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    
    // Create a user
    let user = Keypair::new();
    ctx.anchor_ctx.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
    
    let user_pda = create_user(&mut ctx, &user).unwrap();
    
    // Verify the user account was created
    let user_account = ctx.anchor_ctx.get_account::<User>(&user_pda).expect("Failed to get user account");
    assert!(user_account.authority.eq(&user.pubkey()), "User account should have authority");
    assert_eq!(user_account.positions.len(), MAX_POSITIONS, "User account should have MAX_POSITIONS positions");
    assert_eq!(user_account.positions[0].mint, Pubkey::default(), "User account should have default position");
    assert_eq!(user_account.positions[0].amount, 0, "User account should have default position amount");
    
    println!("✅ User created successfully at {}", user_pda);
}

#[test]
fn test_deposit_tokens() {
    let mut ctx = TestContext::new();
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    
    // Create a user
    let user = Keypair::new();
    ctx.anchor_ctx.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
    let user_pda = create_user(&mut ctx, &user).unwrap();
    
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

    let ata_balance_before = ctx.anchor_ctx.get_account::<TokenAccount>(&user_ata).unwrap();
    assert_eq!(ata_balance_before.amount, 10_000_000, "User ATA should have 10_000_000 tokens");

    let vault = get_associated_token_address(&ctx.config, &mint);
    let vault_balance_before = ctx.anchor_ctx.get_account::<TokenAccount>(&vault);
    assert!(vault_balance_before.is_err(), "Vault should not be initialized");

    // Deposit tokens
    deposit(&mut ctx, &user, user_pda, mint, 5_000_000).unwrap();

    let ata_balance_after = ctx.anchor_ctx.get_account::<TokenAccount>(&user_ata).unwrap();
    assert_eq!(ata_balance_after.amount, 5_000_000, "User ATA should have 5_000_000 tokens");
    
    let user_account = ctx.anchor_ctx.get_account::<User>(&user_pda).expect("Failed to get user account");
    assert_eq!(user_account.positions[0].mint, mint, "Position 0 should have mint");
    assert_eq!(user_account.positions[0].amount, 5_000_000, "Position 0 should have 5_000_000 tokens");

    assert_eq!(user_account.positions[1].mint, Pubkey::default(), "Position 1 should have default mint");
    assert_eq!(user_account.positions[1].amount, 0, "Position 1 should have default amount");

    let vault_balance_after = ctx.anchor_ctx.get_account::<TokenAccount>(&vault).unwrap();
    assert_eq!(vault_balance_after.amount, 5_000_000, "Vault should have 5_000_000 tokens");

    // Create new token mint and mint tokens to the user
    let mint_2 = CreateMint::new(&mut ctx.anchor_ctx.svm, &user)
        .authority(&user.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();
    
    let user_ata_2 = CreateAssociatedTokenAccount::new(&mut ctx.anchor_ctx.svm, &user, &mint_2)
        .owner(&user.pubkey())
        .send()
        .unwrap();
    
    MintTo::new(&mut ctx.anchor_ctx.svm, &user, &mint_2, &user_ata_2, 10_000_000)
        .send()
        .unwrap();

    deposit(&mut ctx, &user, user_pda, mint_2, 6_500_000).unwrap();

    let user_account = ctx.anchor_ctx.get_account::<User>(&user_pda).expect("Failed to get user account");
    assert_eq!(user_account.positions[0].mint, mint, "Position 0 should be unchanged");
    assert_eq!(user_account.positions[0].amount, 5_000_000, "Position 0 should be unchanged");

    assert_eq!(user_account.positions[1].mint, mint_2, "Position 1 should have mint_2");
    assert_eq!(user_account.positions[1].amount, 6_500_000, "Position 1 should have 6_500_000 tokens");

    assert_eq!(user_account.positions[2].mint, Pubkey::default(), "Position 2 should have default mint");
    assert_eq!(user_account.positions[2].amount, 0, "Position 2 should have default amount");
    
    println!("✅ Tokens deposited successfully");
}

#[test]
fn test_deposit_and_withdraw() {
    let mut ctx = TestContext::new();
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    
    // Create a user
    let user = Keypair::new();
    ctx.anchor_ctx.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
    let user_pda = create_user(&mut ctx, &user).unwrap();
    
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
    
    MintTo::new(&mut ctx.anchor_ctx.svm, &user, &mint, &user_ata, 1_000_000)
        .send()
        .unwrap();
    
    // Deposit tokens
    deposit(&mut ctx, &user, user_pda, mint, 500_000).unwrap();
    println!("✅ Tokens deposited");
    
    // Withdraw tokens
    withdraw(&mut ctx, &user, user_pda, mint, 200_000).unwrap();
    println!("✅ Tokens withdrawn successfully");

    let user_account = ctx.anchor_ctx.get_account::<User>(&user_pda).expect("Failed to get user account");
    assert_eq!(user_account.positions[0].mint, mint, "Position 0 should have mint");
    assert_eq!(user_account.positions[0].amount, 300_000, "Position 0 should have 300_000 tokens");

    assert_eq!(user_account.positions[1].mint, Pubkey::default(), "Position 1 should have default mint");
    assert_eq!(user_account.positions[1].amount, 0, "Position 1 should have default amount");

    let ata_balance_after = ctx.anchor_ctx.get_account::<TokenAccount>(&user_ata).unwrap();
    assert_eq!(ata_balance_after.amount, 700_000, "User ATA should have 700_000 tokens");

    let vault = get_associated_token_address(&ctx.config, &mint);
    let vault_balance_after = ctx.anchor_ctx.get_account::<TokenAccount>(&vault).unwrap();
    assert_eq!(vault_balance_after.amount, 300_000, "Vault should have 300_000 tokens");
}

#[test]
fn test_withdraw() {
    let mut ctx = TestContext::new();
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    
    // Create a user
    let user = Keypair::new();
    ctx.anchor_ctx.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
    let user_pda = create_user(&mut ctx, &user).unwrap();
    
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
    
    MintTo::new(&mut ctx.anchor_ctx.svm, &user, &mint, &user_ata, 1_000_000)
        .send()
        .unwrap();
    
    // Deposit tokens
    deposit(&mut ctx, &user, user_pda, mint, 500_000).unwrap();
    println!("✅ Tokens deposited");
    
    // Try withdraw more tokens than deposited
    withdraw(&mut ctx, &user, user_pda, mint, 500_001).expect_err("Should return error");
    println!("✅ Error: Insufficient balance");

    withdraw(&mut ctx, &user, user_pda, mint, 500_000).unwrap();
    println!("✅ Max Tokens withdrawn successfully");

    let user_account = ctx.anchor_ctx.get_account::<User>(&user_pda).expect("Failed to get user account");
    assert_eq!(user_account.positions[0].mint, Pubkey::default(), "Position 0 should have default mint");
    assert_eq!(user_account.positions[0].amount, 0, "Position 0 should have 0 tokens");
}

#[test]
fn test_create_liquidity_pool() {
    let mut ctx = TestContext::new();
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    
    // Create a user to create the pool
    let creator = Keypair::new();
    ctx.anchor_ctx.airdrop(&creator.pubkey(), 10_000_000_000).unwrap();
    
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
    let (lp, mint_lp) = create_lp(&mut ctx, &creator, mint_a, mint_b).unwrap();
    
    // Verify the pool was created
    let lp_account = ctx.anchor_ctx.get_account::<LiquidityPool>(&lp).expect("Failed to get liquidity pool account");
    assert!(lp_account.authority.eq(&ctx.admin.pubkey()), "LP account should have authority");
    assert_eq!(lp_account.status, LiquidityPoolStatus::Paused, "LP account should be paused");
    assert_eq!(lp_account.mint_a, mint_a, "LP account should have mint_a");
    assert_eq!(lp_account.mint_b, mint_b, "LP account should have mint_b");
    assert_eq!(lp_account.vault_a, get_associated_token_address(&ctx.config, &mint_a), "LP account should have vault_a");
    assert_eq!(lp_account.vault_b, get_associated_token_address(&ctx.config, &mint_b), "LP account should have vault_b");
    assert_eq!(lp_account.virtual_reserve_a, 0, "LP account should have virtual_reserve_a");
    assert_eq!(lp_account.virtual_reserve_b, 0, "LP account should have virtual_reserve_b");
    assert_eq!(lp_account.lp_mint, mint_lp, "LP account should have lp_mint");
    assert_eq!(lp_account.lp_supply, 0, "LP account should have lp_supply");
    assert_eq!(lp_account.pool_fee_bps, 100, "LP account should have pool_fee_bps");
    
    println!("✅ Liquidity pool created at {}", lp);
    println!("✅ LP token mint created at {}", mint_lp);
}

#[test]
fn test_add_liquidity() {
    let mut ctx = TestContext::new();
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    
    // Create a user
    let user = Keypair::new();
    ctx.anchor_ctx.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
    let user_pda = create_user(&mut ctx, &user).unwrap();
    
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
    
    // Mint tokens to user and deposit
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
    
    // Deposit tokens into the protocol
    deposit(&mut ctx, &user, user_pda, mint_a, 5_000_000).unwrap();
    deposit(&mut ctx, &user, user_pda, mint_b, 5_000_000).unwrap();
    println!("✅ Tokens deposited");
    
    // Create liquidity pool
    let (lp, mint_lp) = create_lp(&mut ctx, &user, mint_a, mint_b).unwrap();
    println!("✅ LP created");
    
    // Add liquidity (first deposit, so we can set any ratio)
    add_liquidity(
        &mut ctx,
        &user,
        user_pda,
        mint_a,
        mint_b,
        lp,
        mint_lp,
        1_000_000, // LP tokens to mint
        2_000_000, // max token A
        2_000_000, // max token B
    )
    .unwrap();

    let user_account = ctx.anchor_ctx.get_account::<User>(&user_pda).expect("Failed to get user account");
    assert_eq!(user_account.positions[0].mint, mint_a, "Position 0 should have mint_a");
    assert_eq!(user_account.positions[0].amount, 3_000_000, "Position 0 should have 3_000_000 tokens");

    assert_eq!(user_account.positions[1].mint, mint_b, "Position 1 should have mint_b");
    assert_eq!(user_account.positions[1].amount, 3_000_000, "Position 1 should have 3_000_000 tokens");

    assert_eq!(user_account.positions[2].mint, mint_lp, "Position 2 should have mint_lp");
    assert_eq!(user_account.positions[2].amount, 1_000_000, "Position 2 should have 1_000_000 tokens");

    let lp_account = ctx.anchor_ctx.get_account::<LiquidityPool>(&lp).expect("Failed to get liquidity pool account");
    assert_eq!(lp_account.status, LiquidityPoolStatus::Active, "LP account should be active");
    assert_eq!(lp_account.virtual_reserve_a, 2_000_000, "LP account should have virtual_reserve_a");
    assert_eq!(lp_account.virtual_reserve_b, 2_000_000, "LP account should have virtual_reserve_b");
    assert_eq!(lp_account.lp_supply, 1_000_000, "LP account should have lp_supply");
    
    println!("✅ Liquidity added successfully");
}

#[test]
fn test_add_and_remove_liquidity() {
    let mut ctx = TestContext::new();
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    
    // Create a user
    let user = Keypair::new();
    ctx.anchor_ctx.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
    let user_pda = create_user(&mut ctx, &user).unwrap();
    
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
    
    // Create liquidity pool
    let (lp, mint_lp) = create_lp(&mut ctx, &user, mint_a, mint_b).unwrap();
    
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
    
    // Remove liquidity
    remove_liquidity(
        &mut ctx,
        &user,
        user_pda,
        mint_a,
        mint_b,
        lp,
        mint_lp,
        500_000, // LP tokens to burn
        0,       // min token A
        0,       // min token B
    )
    .unwrap();

    let user_account = ctx.anchor_ctx.get_account::<User>(&user_pda).expect("Failed to get user account");
    assert_eq!(user_account.positions[0].mint, mint_a, "Position 0 should have mint_a");
    assert_eq!(user_account.positions[0].amount, 4_000_000, "Position 0 should have 4_000_000 tokens");

    assert_eq!(user_account.positions[1].mint, mint_b, "Position 1 should have mint_b");
    assert_eq!(user_account.positions[1].amount, 4_000_000, "Position 1 should have 4_000_000 tokens");

    assert_eq!(user_account.positions[2].mint, mint_lp, "Position 2 should have mint_lp");
    assert_eq!(user_account.positions[2].amount, 500_000, "Position 2 should have 500_000 tokens");

    let lp_account = ctx.anchor_ctx.get_account::<LiquidityPool>(&lp).expect("Failed to get liquidity pool account");
    assert_eq!(lp_account.status, LiquidityPoolStatus::Active, "LP account should be active");
    assert_eq!(lp_account.virtual_reserve_a, 1_000_000, "LP account should have virtual_reserve_a");
    assert_eq!(lp_account.virtual_reserve_b, 1_000_000, "LP account should have virtual_reserve_b");
    assert_eq!(lp_account.lp_supply, 500_000, "LP account should have lp_supply");
    
    println!("✅ Liquidity removed successfully");
}

#[test]
fn test_swap() {
    let mut ctx = TestContext::new();
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    
    // Create a user
    let user = Keypair::new();
    ctx.anchor_ctx.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
    let user_pda = create_user(&mut ctx, &user).unwrap();
    
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
    
    deposit(&mut ctx, &user, user_pda, mint_a, 8_000_000).unwrap();
    deposit(&mut ctx, &user, user_pda, mint_b, 8_000_000).unwrap();
    
    // Create liquidity pool and add liquidity
    let (lp, mint_lp) = create_lp(&mut ctx, &user, mint_a, mint_b).unwrap();
    
    add_liquidity(
        &mut ctx,
        &user,
        user_pda,
        mint_a,
        mint_b,
        lp,
        mint_lp,
        1_000_000,
        5_000_000,
        5_000_000,
    )
    .unwrap();
    println!("✅ Liquidity added");

    let lp_data = ctx.anchor_ctx.get_account::<LiquidityPool>(&lp).expect("Failed to get liquidity pool account");
    assert_eq!(lp_data.lp_supply, 1_000_000);
    assert_eq!(lp_data.virtual_reserve_a, 5_000_000);
    assert_eq!(lp_data.virtual_reserve_b, 5_000_000);
    assert_eq!(lp_data.status, LiquidityPoolStatus::Active);
    
    // Try to swap
    let _ = swap(
        &mut ctx,
        &user,
        user_pda,
        mint_a,
        mint_b,
        lp,
        true,   // swap X for Y
        100_000, // amount
        0,      // min output
    );

    let lp_data_after = ctx.anchor_ctx.get_account::<LiquidityPool>(&lp).expect("Failed to get liquidity pool account");
    println!("LP state after: {:?}", lp_data_after);

    assert_eq!(lp_data_after.virtual_reserve_a, 5_100_000);
    assert!(lp_data_after.virtual_reserve_b - 4_900_000 < 5000);  // difference is commission and CP delta

    let user_account = ctx.anchor_ctx.get_account::<User>(&user_pda).expect("Failed to get user account");
    assert_eq!(user_account.positions[0].mint, mint_a, "Position 0 should have mint_a");
    assert_eq!(user_account.positions[0].amount, 2_900_000, "Position 0 should have 2_900_000 tokens");

    assert_eq!(user_account.positions[1].mint, mint_b, "Position 1 should have mint_b");
    assert!(3_100_000 - user_account.positions[1].amount < 5000, "Position 1 should be within 5000 tokens of 3_100_000 tokens with tolerance for pool fee and CP delta");
    
    println!("✅ Swap successful");
}

#[test]
fn test_transfer_between_users() {
    let mut ctx = TestContext::new();
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    
    // Create two users
    let user1 = Keypair::new();
    let user2 = Keypair::new();
    ctx.anchor_ctx.airdrop(&user1.pubkey(), 10_000_000_000).unwrap();
    ctx.anchor_ctx.airdrop(&user2.pubkey(), 10_000_000_000).unwrap();
    
    let user1_pda = create_user(&mut ctx, &user1).unwrap();
    let user2_pda = create_user(&mut ctx, &user2).unwrap();
    
    // Create a token mint
    let mint = CreateMint::new(&mut ctx.anchor_ctx.svm, &user1)
        .authority(&user1.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();
    
    // Mint tokens to user1 and deposit
    let user1_ata = CreateAssociatedTokenAccount::new(&mut ctx.anchor_ctx.svm, &user1, &mint)
        .owner(&user1.pubkey())
        .send()
        .unwrap();
    
    MintTo::new(&mut ctx.anchor_ctx.svm, &user1, &mint, &user1_ata, 1_000_000)
        .send()
        .unwrap();
    
    deposit(&mut ctx, &user1, user1_pda, mint, 500_000).unwrap();
    println!("✅ User1 deposited tokens");
    
    // Transfer from user1 to user2
    transfer(&mut ctx, &user1, user1_pda, user2_pda, mint, 200_000).unwrap();

    let user1_account = ctx.anchor_ctx.get_account::<User>(&user1_pda).expect("Failed to get user1 account");
    assert_eq!(user1_account.positions[0].mint, mint, "Position 0 should have mint");
    assert_eq!(user1_account.positions[0].amount, 300_000, "Position 0 should have 300_000 tokens");

    let user2_account = ctx.anchor_ctx.get_account::<User>(&user2_pda).expect("Failed to get user2 account");
    assert_eq!(user2_account.positions[0].mint, mint, "Position 0 should have mint");
    assert_eq!(user2_account.positions[0].amount, 200_000, "Position 0 should have 200_000 tokens");

    println!("✅ Transfer successful from user1 to user2");
}

#[test]
fn test_full_flow() {
    let mut ctx = TestContext::new();
    
    println!("=== Full Flow Test ===");
    
    // 1. Initialize
    initialize_config(&mut ctx).unwrap();
    println!("✅ 1. Config initialized");
    
    // 2. Create users
    let user1 = Keypair::new();
    let user2 = Keypair::new();
    ctx.anchor_ctx.airdrop(&user1.pubkey(), 10_000_000_000).unwrap();
    ctx.anchor_ctx.airdrop(&user2.pubkey(), 10_000_000_000).unwrap();
    
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
    
    // 8. Transfer some tokens to user2
    transfer(&mut ctx, &user1, user1_pda, user2_pda, mint_a, 500_000).unwrap();
    println!("✅ 8. Tokens transferred to user2");
    
    // 9. Withdraw some tokens
    withdraw(&mut ctx, &user1, user1_pda, mint_b, 500_000).unwrap();
    println!("✅ 9. Tokens withdrawn");
    
    println!("\n🎉 Full flow completed successfully!");
}

#[test]
fn test_multiple_deposits_same_mint() {
    let mut ctx = TestContext::new();
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    
    // Create a user
    let user = Keypair::new();
    ctx.anchor_ctx.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
    let user_pda = create_user(&mut ctx, &user).unwrap();
    
    // Create a token mint
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
    
    // Multiple deposits to the same mint
    deposit(&mut ctx, &user, user_pda, mint, 1_000_000).unwrap();
    println!("✅ First deposit");
    
    deposit(&mut ctx, &user, user_pda, mint, 2_000_000).unwrap();
    println!("✅ Second deposit");
    
    deposit(&mut ctx, &user, user_pda, mint, 1_500_000).unwrap();
    println!("✅ Third deposit");

    let user_account = ctx.anchor_ctx.get_account::<User>(&user_pda).expect("Failed to get user account");
    assert_eq!(user_account.positions[0].mint, mint, "Position 0 should have mint");
    assert_eq!(user_account.positions[0].amount, 4_500_000, "Position 0 should have 4_500_000 tokens");

    assert_eq!(user_account.positions[1].mint, Pubkey::default(), "Position 1 should have default mint");
    assert_eq!(user_account.positions[1].amount, 0, "Position 1 should have default amount");
    
    println!("✅ Multiple deposits to same mint successful");
}
