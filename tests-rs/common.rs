use anchor_litesvm::{AnchorContext, AnchorLiteSVM};
use private_dex::{instruction, state::LiquidityPool};
use anchor_lang::{InstructionData, ToAccountMetas, system_program};
use solana_sdk::{
    instruction::Instruction, 
    pubkey::Pubkey, 
    signature::{read_keypair_file, Keypair, Signer}, 
    transaction::Transaction
};
use litesvm_token::{
    CreateAssociatedTokenAccount, CreateMint, MintTo,
};
use spl_associated_token_account::get_associated_token_address;

// Re-export utility functions for PDA derivations
// pub mod utils;
// pub use utils::{get_group_pda, get_permission_pda};

// Constants from the program
pub const CONFIG_SEED: &[u8] = b"config";
pub const USER_SEED: &[u8] = b"user";
pub const LIQUIDITY_POOL_SEED: &[u8] = b"liquidity_pool";
pub const LP_MINT_SEED: &[u8] = b"lp_mint";
pub const LP_DECIMALS: u8 = 6;
pub const DECIMALS: u8 = 6;

// MagicBlock delegate program
pub const DELEGATE_PROGRAM_ID: Pubkey = Pubkey::from_str_const("DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh");
// localnet ER validator for tests
pub const ER_VALIDATOR_ID: Pubkey = Pubkey::from_str_const("mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev");
// MagicBlock permission program
pub const PERMISSION_PROGRAM_ID: Pubkey = Pubkey::from_str_const("BTWAqWNBmF2TboMh3fxMJfgR16xGHYD7Kgr2dPwbRPBi");

// ============================================================================
// Helper Types and Context
// ============================================================================

pub struct TestContext {
    pub anchor_ctx: AnchorContext,
    pub program_id: Pubkey,
    pub admin: Keypair,
    pub config: Pubkey,
}

impl TestContext {
    pub fn new() -> Self {        
        let program_keypair = read_keypair_file("../target/deploy/private_dex-keypair.json").unwrap();
        let program_id = program_keypair.pubkey();
        let program_bytes = include_bytes!("../target/deploy/private_dex.so");

        let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);

        // add permission program
        let permission_program_bytes = include_bytes!("fixtures/permission.so");
        let permission_program_id = PERMISSION_PROGRAM_ID;
        ctx.svm.add_program(permission_program_id, permission_program_bytes);

        // add delegate program
        let delegate_program_bytes = include_bytes!("fixtures/dlp.so");
        let delegate_program_id = DELEGATE_PROGRAM_ID;
        ctx.svm.add_program(delegate_program_id, delegate_program_bytes);
        
        let admin = read_keypair_file("../tmp/admin.json").unwrap();
        ctx.svm.airdrop(&admin.pubkey(), 100_000_000_000).unwrap(); // 100 SOL

        let balance = ctx.svm.get_balance(&admin.pubkey()).unwrap();
        println!("Balance: {:?}", balance);
        
        let (config, _) = Pubkey::find_program_address(&[CONFIG_SEED], &program_id);
        
        TestContext {
            anchor_ctx: ctx,
            program_id,
            admin,
            config,
        }
    }
}

// ============================================================================
// Helper Functions for Instructions
// ============================================================================

/// Helper: Initialize the config
pub fn initialize_config(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
    let accounts = private_dex::accounts::Initialize {
        sender: ctx.admin.pubkey(),
        config: ctx.config,
        delegate_program: DELEGATE_PROGRAM_ID,
        er_validator: ER_VALIDATOR_ID,
        permission_program: PERMISSION_PROGRAM_ID,
        system_program: system_program::ID,
    };
    
    let data = instruction::Initialize {};
    
    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&ctx.admin.pubkey()),
        &[&ctx.admin],
        ctx.anchor_ctx.latest_blockhash(),
    );
    
    ctx.anchor_ctx.send_and_confirm_transaction(&tx)?;
    Ok(())
}

/// Helper: Update config
pub fn update_config(
    ctx: &mut TestContext,
    is_paused: Option<bool>,
    default_pool_fee_bps: Option<u16>,
) -> Result<(), Box<dyn std::error::Error>> {
    let accounts = private_dex::accounts::UpdateConfig {
        sender: ctx.admin.pubkey(),
        config: ctx.config,
        er_validator: None,
        system_program: system_program::ID,
    };
    
    let data = instruction::UpdateConfig {
        is_paused,
        default_pool_fee_bps,
    };
    
    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&ctx.admin.pubkey()),
        &[&ctx.admin],
        ctx.anchor_ctx.latest_blockhash(),
    );
    
    ctx.anchor_ctx.send_and_confirm_transaction(&tx)?;
    Ok(())
}

/// Helper: Create user
pub fn create_user(
    ctx: &mut TestContext,
    user_keypair: &Keypair,
) -> Result<Pubkey, Box<dyn std::error::Error>> {
    let (user_pda, _) = Pubkey::find_program_address(
        &[USER_SEED, user_keypair.pubkey().as_ref()],
        &ctx.program_id,
    );
    
    let accounts = private_dex::accounts::CreateUser {
        sender: user_keypair.pubkey(),
        user: user_pda,
        config: ctx.config,
        system_program: system_program::ID,
    };
    
    let data = instruction::CreateUser {};
    
    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&user_keypair.pubkey()),
        &[user_keypair],
        ctx.anchor_ctx.latest_blockhash(),
    );
    
    ctx.anchor_ctx.send_and_confirm_transaction(&tx)?;
    Ok(user_pda)
}

/// Helper: Deposit tokens
pub fn deposit(
    ctx: &mut TestContext,
    user_keypair: &Keypair,
    user_pda: Pubkey,
    mint: Pubkey,
    amount: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let sender_ata = get_associated_token_address(&user_keypair.pubkey(), &mint);
    let vault = get_associated_token_address(&ctx.config, &mint);
    
    let accounts = private_dex::accounts::Deposit {
        sender: user_keypair.pubkey(),
        user: user_pda,
        config: ctx.config,
        sender_ata,
        vault,
        mint,
        associated_token_program: anchor_spl::associated_token::ID,
        token_program: anchor_spl::token::ID,
        system_program: system_program::ID,
    };
    
    let data = instruction::Deposit { amount };
    
    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&user_keypair.pubkey()),
        &[user_keypair],
        ctx.anchor_ctx.latest_blockhash(),
    );
    
    ctx.anchor_ctx.send_and_confirm_transaction(&tx)?;
    Ok(())
}

/// Helper: Withdraw tokens
pub fn withdraw(
    ctx: &mut TestContext,
    user_keypair: &Keypair,
    user_pda: Pubkey,
    mint: Pubkey,
    amount: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let sender_ata = get_associated_token_address(&user_keypair.pubkey(), &mint);
    let vault = get_associated_token_address(&ctx.config, &mint);
    
    let accounts = private_dex::accounts::Withdraw {
        sender: user_keypair.pubkey(),
        user: user_pda,
        config: ctx.config,
        sender_ata,
        vault,
        mint,
        associated_token_program: anchor_spl::associated_token::ID,
        token_program: anchor_spl::token::ID,
        system_program: system_program::ID,
    };
    
    let data = instruction::Withdraw { amount };
    
    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&user_keypair.pubkey()),
        &[user_keypair],
        ctx.anchor_ctx.latest_blockhash(),
    );
    
    ctx.anchor_ctx.send_and_confirm_transaction(&tx)?;
    Ok(())
}

/// Helper: Create liquidity pool
pub fn create_lp(
    ctx: &mut TestContext,
    creator: &Keypair,
    mint_a: Pubkey,
    mint_b: Pubkey,
) -> Result<(Pubkey, Pubkey), Box<dyn std::error::Error>> {
    let (lp, _) = Pubkey::find_program_address(
        &[LIQUIDITY_POOL_SEED, mint_a.as_ref(), mint_b.as_ref()],
        &ctx.program_id,
    );
    
    let (mint_lp, _) = Pubkey::find_program_address(
        &[LP_MINT_SEED, lp.as_ref()],
        &ctx.program_id,
    );
    
    let vault_a = get_associated_token_address(&ctx.config, &mint_a);
    let vault_b = get_associated_token_address(&ctx.config, &mint_b);
    let vault_lp = get_associated_token_address(&ctx.config, &mint_lp);
    
    let accounts = private_dex::accounts::CreateLp {
        sender: creator.pubkey(),
        lp,
        config: ctx.config,
        mint_a,
        mint_b,
        mint_lp,
        vault_a,
        vault_b,
        vault_lp,
        associated_token_program: anchor_spl::associated_token::ID,
        token_program: anchor_spl::token::ID,
        system_program: system_program::ID,
    };
    
    let data = instruction::CreateLp {};
    
    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&creator.pubkey()),
        &[creator],
        ctx.anchor_ctx.latest_blockhash(),
    );
    
    ctx.anchor_ctx.send_and_confirm_transaction(&tx)?;
    Ok((lp, mint_lp))
}

/// Helper: Add liquidity
pub fn add_liquidity(
    ctx: &mut TestContext,
    user_keypair: &Keypair,
    user_pda: Pubkey,
    mint_a: Pubkey,
    mint_b: Pubkey,
    lp: Pubkey,
    mint_lp: Pubkey,
    amount: u64,
    max_x: u64,
    max_y: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let accounts = private_dex::accounts::AddLiquidity {
        payer: user_keypair.pubkey(),
        user: user_keypair.pubkey(),
        user_account: user_pda,
        lp,
        config: ctx.config,
        mint_a,
        mint_b,
        mint_lp,
        session_token: None,
        system_program: system_program::ID,
    };
    
    let data = instruction::AddLiquidity {
        amount,
        max_x,
        max_y,
    };
    
    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&user_keypair.pubkey()),
        &[user_keypair],
        ctx.anchor_ctx.latest_blockhash(),
    );
    
    ctx.anchor_ctx.send_and_confirm_transaction(&tx)?;
    Ok(())
}

/// Helper: Remove liquidity
pub fn remove_liquidity(
    ctx: &mut TestContext,
    user_keypair: &Keypair,
    user_pda: Pubkey,
    mint_a: Pubkey,
    mint_b: Pubkey,
    lp: Pubkey,
    mint_lp: Pubkey,
    amount: u64,
    min_x: u64,
    min_y: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let accounts = private_dex::accounts::RemoveLiquidity {
        payer: user_keypair.pubkey(),
        user: user_keypair.pubkey(),
        user_account: user_pda,
        lp,
        config: ctx.config,
        mint_a,
        mint_b,
        mint_lp,
        session_token: None,
        system_program: system_program::ID,
    };
    
    let data = instruction::RemoveLiquidity {
        amount,
        min_x,
        min_y,
    };
    
    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&user_keypair.pubkey()),
        &[user_keypair],
        ctx.anchor_ctx.latest_blockhash(),
    );
    
    ctx.anchor_ctx.send_and_confirm_transaction(&tx)?;
    Ok(())
}

/// Helper: Swap tokens
pub fn swap(
    ctx: &mut TestContext,
    user_keypair: &Keypair,
    user_pda: Pubkey,
    mint_a: Pubkey,
    mint_b: Pubkey,
    lp: Pubkey,
    is_x: bool,
    amount: u64,
    min: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let accounts = private_dex::accounts::Swap {
        payer: user_keypair.pubkey(),
        user: user_keypair.pubkey(),
        user_account: user_pda,
        lp,
        config: ctx.config,
        mint_a,
        mint_b,
        session_token: None,
        system_program: system_program::ID,
    };
    
    let data = instruction::Swap { is_x, amount, min };
    
    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&user_keypair.pubkey()),
        &[user_keypair],
        ctx.anchor_ctx.latest_blockhash(),
    );
    
    ctx.anchor_ctx.send_and_confirm_transaction(&tx)?;
    Ok(())
}

/// Helper: Transfer tokens between users
pub fn transfer(
    ctx: &mut TestContext,
    sender_keypair: &Keypair,
    sender_pda: Pubkey,
    destination_pda: Pubkey,
    mint: Pubkey,
    amount: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let accounts = private_dex::accounts::Transfer {
        payer: sender_keypair.pubkey(),
        user: sender_keypair.pubkey(),
        user_account: sender_pda,
        config: ctx.config,
        destination_user_account: destination_pda,
        mint,
        session_token: None,
        system_program: system_program::ID,
    };
    
    let data = instruction::Transfer { amount };
    
    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&sender_keypair.pubkey()),
        &[sender_keypair],
        ctx.anchor_ctx.latest_blockhash(),
    );
    
    ctx.anchor_ctx.send_and_confirm_transaction(&tx)?;
    Ok(())
}


fn buffer_account(
    account_pda: Pubkey,
    program_id: Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(&[b"buffer", account_pda.as_ref()], &program_id).0
}

fn delegation_record_account(
    account_pda: Pubkey,
    delegation_program_id: Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(&[b"delegation", account_pda.as_ref()], &delegation_program_id).0
}

fn delegation_metadata_account(
    account_pda: Pubkey,
    delegation_program_id: Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(&[b"delegation-metadata", account_pda.as_ref()], &delegation_program_id).0
}

/// Helper: Delegate user account to ephemeral rollups
pub fn delegate_user(
    ctx: &mut TestContext,
    payer: &Keypair,
    user: Pubkey,
    user_pda: Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {    
    let data = instruction::DelegateUser { user };
    
    // Manually construct account metas for the delegate instruction
    let accounts = private_dex::accounts::DelegateUser {
        payer: payer.pubkey(),
        config: ctx.config,
        validator: ER_VALIDATOR_ID,
        user_account: user_pda,
        buffer_user_account: buffer_account(user_pda, ctx.program_id),
        delegation_record_user_account: delegation_record_account(user_pda, DELEGATE_PROGRAM_ID),
        delegation_metadata_user_account: delegation_metadata_account(user_pda, DELEGATE_PROGRAM_ID),
        owner_program: ctx.program_id,
        delegation_program: DELEGATE_PROGRAM_ID,
        system_program: system_program::ID,
    };
    
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
    
    ctx.anchor_ctx.send_and_confirm_transaction(&tx)?;
    Ok(())
}

/// Helper: Delegate LP account to ephemeral rollups
pub fn delegate_lp(
    ctx: &mut TestContext,
    payer: &Keypair,
    lp: Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {
    let lp_account = ctx.anchor_ctx.get_account::<LiquidityPool>(&lp).unwrap();
    
    let data = instruction::DelegateLp {
        mint_a: lp_account.mint_a,
        mint_b: lp_account.mint_b,
    };
    
    // Manually construct account metas for the delegate instruction
    let accounts = private_dex::accounts::DelegateLp {
        payer: payer.pubkey(),
        config: ctx.config,
        validator: ER_VALIDATOR_ID,
        lp_account: lp,
        buffer_lp_account: buffer_account(lp, ctx.program_id),
        delegation_record_lp_account: delegation_record_account(lp, DELEGATE_PROGRAM_ID),
        delegation_metadata_lp_account: delegation_metadata_account(lp, DELEGATE_PROGRAM_ID),
        owner_program: ctx.program_id,
        delegation_program: DELEGATE_PROGRAM_ID,
        system_program: system_program::ID,
    };
    
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
    
    ctx.anchor_ctx.send_and_confirm_transaction(&tx)?;
    Ok(())
}

// ============================================================================
// Utility Functions for PDA Derivations
// ============================================================================

/// Derives the Group PDA for a given group_id
pub fn get_group_pda(group_id: Pubkey, permission_program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"group:", group_id.as_ref()],
        permission_program_id
    ).0
}

/// Derives the Permission PDA for a given delegated account and group
pub fn get_permission_pda(delegated_account: Pubkey, group: Pubkey, permission_program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"permission:", delegated_account.as_ref()],
        permission_program_id
    ).0
}

// ============================================================================
// Helper Functions for Test Setup
// ============================================================================

pub fn setup_user_with_tokens(
    ctx: &mut TestContext,
) -> Result<(Keypair, Pubkey), Box<dyn std::error::Error>> {
    let user = Keypair::new();
    ctx.anchor_ctx.svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
    let user_pda = create_user(ctx, &user)?;
    Ok((user, user_pda))
}

pub fn create_and_mint_token(
    ctx: &mut TestContext,
    authority: &Keypair,
    amount: u64,
) -> Result<(Pubkey, Pubkey), Box<dyn std::error::Error>> {
    let mint = CreateMint::new(&mut ctx.anchor_ctx.svm, authority)
        .authority(&authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();
    
    let ata = CreateAssociatedTokenAccount::new(&mut ctx.anchor_ctx.svm, authority, &mint)
        .owner(&authority.pubkey())
        .send()
        .unwrap();
    
    MintTo::new(&mut ctx.anchor_ctx.svm, authority, &mint, &ata, amount)
        .send()
        .unwrap();
    
    Ok((mint, ata))
}

