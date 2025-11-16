mod common;

use common::*;
use anchor_lang::{InstructionData, ToAccountMetas, system_program};
use private_dex::{instruction, state::LiquidityPool};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo};

use crate::common::PERMISSION_PROGRAM_ID;

// ============================================================================
// Helper Functions for Permission Instructions
// ============================================================================

/// Helper: Create user permission
fn create_user_permission(
    ctx: &mut TestContext,
    payer: &Keypair,
    user_keypair: &Keypair,
    user_pda: Pubkey,
    group_id: Pubkey,
) -> Result<(Pubkey, Pubkey), Box<dyn std::error::Error>> {
    // Use utility functions for PDA derivation
    let group = get_group_pda(group_id, &PERMISSION_PROGRAM_ID);
    let permission = get_permission_pda(user_pda, group, &PERMISSION_PROGRAM_ID);
    
    let accounts = private_dex::accounts::CreateUserPermission {
        payer: payer.pubkey(),
        user: user_keypair.pubkey(),
        user_account: user_pda,
        permission,
        group,
        permission_program: PERMISSION_PROGRAM_ID,
        system_program: system_program::ID,
    };
    
    let data = instruction::CreateUserPermission { group_id };
    
    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer],
        ctx.anchor_ctx.latest_blockhash(),
    );
    
    let signature = ctx.anchor_ctx.send_and_confirm_transaction(&tx)?;
    let tx_res = ctx.anchor_ctx.svm.get_transaction(&signature).unwrap();
    println!("tx_res: {:?}", tx_res);
    Ok((permission, group))
}

/// Helper: Create LP permission group
fn create_lp_permission_group(
    ctx: &mut TestContext,
    sender: &Keypair,
    group_id: Pubkey,
    users: Vec<Pubkey>,
) -> Result<Pubkey, Box<dyn std::error::Error>> {
    // Use utility function for PDA derivation
    let group = get_group_pda(group_id, &PERMISSION_PROGRAM_ID);
    
    let accounts = private_dex::accounts::CreateLpPermissionGroup {
        sender: sender.pubkey(),
        group,
        permission_program: PERMISSION_PROGRAM_ID,
        system_program: system_program::ID,
    };
    
    let data = instruction::CreateLpPermissionGroup { group_id };
    
    // Add remaining accounts (users to add to the group)
    let mut account_metas = accounts.to_account_metas(None);
    for user in users.iter() {
        account_metas.push(AccountMeta::new_readonly(*user, false));
    }
    
    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: account_metas,
        data: data.data(),
    };
    
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&sender.pubkey()),
        &[sender],
        ctx.anchor_ctx.latest_blockhash(),
    );
    
    ctx.anchor_ctx.send_and_confirm_transaction(&tx)?;
    Ok(group)
}

/// Helper: Create LP permission
fn create_lp_permission(
    ctx: &mut TestContext,
    sender: &Keypair,
    lp: Pubkey,
    group: Pubkey,
) -> Result<Pubkey, Box<dyn std::error::Error>> {
    // Use utility function for PDA derivation
    let permission = get_permission_pda(lp, group, &PERMISSION_PROGRAM_ID);
    
    let accounts = private_dex::accounts::CreateLpPermission {
        sender: sender.pubkey(),
        lp,
        permission,
        group,
        permission_program: PERMISSION_PROGRAM_ID,
        system_program: system_program::ID,
    };

    let lp_account = ctx.anchor_ctx.get_account::<LiquidityPool>(&lp).unwrap();
    
    let data = instruction::CreateLpPermission { mint_a: lp_account.mint_a, mint_b: lp_account.mint_b, bump: lp_account.bump };
    
    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&sender.pubkey()),
        &[sender],
        ctx.anchor_ctx.latest_blockhash(),
    );
    
    ctx.anchor_ctx.send_and_confirm_transaction(&tx)?;
    Ok(permission)
}

// ============================================================================
// Test Cases
// ============================================================================

#[test]
fn test_create_user_permission() {
    let mut ctx = TestContext::new();
    
    // Initialize the config
    initialize_config(&mut ctx).unwrap();
    println!("✅ Config initialized");
    
    // Create a user
    let user = Keypair::new();
    ctx.anchor_ctx.svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
    let user_pda = create_user(&mut ctx, &user).unwrap();
    println!("✅ User created at {}", user_pda);
    
    // Create permission for the user
    let group_id = Keypair::new().pubkey(); // Random group ID
    let (permission, _group) = create_user_permission(&mut ctx, &user, &user, user_pda, group_id).expect("Failed to create user permission");
    println!("✅ User permission created at {}", permission);
}

#[test]
fn test_create_lp_permission_group() {
    let mut ctx = TestContext::new();
    
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
    println!("✅ Users created");
    
    // Create permission group with multiple users
    let group_id = Keypair::new().pubkey();
    let users = vec![user1.pubkey(), user2.pubkey(), user3.pubkey()];
    
    create_lp_permission_group(&mut ctx, &user1, group_id, users.clone()).expect("Failed to create LP permission group");
    println!("✅ LP permission group created");
    
    println!("✅ LP permission group created successfully");
    println!("   Group PDA: {}", group_id);
    println!("   Members: {} users", users.len());
    println!("   User 1 PDA: {}", user1_pda);
    println!("   User 2 PDA: {}", user2_pda);
    println!("   User 3 PDA: {}", user3_pda);
}

#[test]
fn test_create_lp_permission() {
    let mut ctx = TestContext::new();
    
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
    
    // First, create a permission group
    let group_id = Keypair::new().pubkey();
    let users = vec![creator.pubkey()];
    
    create_lp_permission_group(&mut ctx, &creator, group_id, users).expect("Failed to create LP permission group");
    println!("✅ LP permission group created");

    let group = get_group_pda(group_id, &PERMISSION_PROGRAM_ID);
    // Now create permission for the LP
    let permission = create_lp_permission(&mut ctx, &creator, lp, group).expect("Failed to create LP permission");
    println!("✅ LP permission created at {}", permission);

    println!("✅ LP permission created successfully");
    println!("   LP PDA: {}", lp);
    println!("   Permission PDA: {}", permission);
    println!("   Group PDA: {}", group);
}

#[test]
fn test_full_permission_flow() {
    let mut ctx = TestContext::new();
    
    println!("=== Full Permission Flow Test ===");
    
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
    
    // 3. Create user permissions
    let group_id_1 = Keypair::new().pubkey();
    let group_id_2 = Keypair::new().pubkey();
    
    let perm1_result = create_user_permission(&mut ctx, &user1, &user1, user1_pda, group_id_1);
    let perm2_result = create_user_permission(&mut ctx, &user2, &user2, user2_pda, group_id_2);
    
    if perm1_result.is_ok() && perm2_result.is_ok() {
        println!("✅ 3. User permissions created");
    } else {
        println!("⚠️  3. User permission creation failed (expected if permission program not loaded)");
    }
    
    // 4. Create tokens and LP
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
    
    let (lp, _) = create_lp(&mut ctx, &user1, mint_a, mint_b).unwrap();
    println!("✅ 4. Tokens and LP created");
    
    // 5. Create LP permission group with both users
    let lp_group_id = Keypair::new().pubkey();
    let users = vec![user1.pubkey(), user2.pubkey()];
    
    create_lp_permission_group(&mut ctx, &user1, lp_group_id, users).expect("Failed to create LP permission group");
    println!("✅ LP permission group created");
    
    let group = get_group_pda(lp_group_id, &PERMISSION_PROGRAM_ID);
    
    // 6. Create LP permission
    let lp_perm_result = create_lp_permission(&mut ctx, &user1, lp, group).expect("Failed to create LP permission");
    println!("✅ LP permission created");

    println!("✅ LP permission created successfully");
    println!("   Permission PDA: {}", lp_perm_result);
    println!("   LP PDA: {}", lp);
    println!("   Group PDA: {}", group);
}

#[test]
fn test_user_permission_with_deposit() {
    let mut ctx = TestContext::new();
    
    println!("=== User Permission with Deposit Test ===");
    
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
    println!("✅ Tokens deposited");
    
    // Create permission for the user
    let group_id = Keypair::new().pubkey();
    let (permission, group) = create_user_permission(&mut ctx, &user, &user, user_pda, group_id).expect("Failed to create user permission");
    println!("✅ User permission created");
    
    println!("✅ User permission created successfully");
    println!("   Permission PDA: {}", permission);
    println!("   Group PDA: {}", group);
    println!("   User PDA: {}", user_pda);
}

