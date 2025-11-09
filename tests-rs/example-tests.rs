// use anchor_escrow_q4_25::{accounts, instruction};
// use anchor_lang::{system_program, InstructionData, ToAccountMetas};
// use litesvm::LiteSVM;
// use solana_sdk::{
//     instruction::Instruction, pubkey::Pubkey, signature::{read_keypair_file, Keypair, Signer}, transaction::Transaction
// };
// use litesvm_token::{
//     spl_token::{self, native_mint::DECIMALS},
//     CreateAssociatedTokenAccount, CreateMint, MintTo,
// };
// use spl_associated_token_account::get_associated_token_address;

// #[test]
// fn test_make_and_refund() {
//     // ============================================================================
//     // Test Env Setup: Initialize environment and deploy escrow program
//     // ============================================================================
//     let mut svm = LiteSVM::new();

//     let program_keypair = read_keypair_file("target/deploy/anchor_escrow_q4_25-keypair.json").unwrap();
//     let program_id = program_keypair.pubkey();
//     let program_bytes = include_bytes!("../target/deploy/anchor_escrow_q4_25.so");

//     svm.add_program(program_id, program_bytes);

//     // ============================================================================
//     // Create and fund test accounts
//     // ============================================================================
//     let maker = Keypair::new();
//     let taker = Keypair::new();
//     svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap(); // 10 SOL
//     svm.airdrop(&taker.pubkey(), 10_000_000_000).unwrap(); // 10 SOL

//     // ============================================================================
//     // Token Setup: Create mints and token accounts
//     // Token swap flow: Maker offers mint_a tokens, wants mint_b tokens in return
//     // ============================================================================

//     // Create two token mints
//     let mint_a = CreateMint::new(&mut svm, &maker)
//         .authority(&maker.pubkey())
//         .decimals(DECIMALS)
//         .send()
//         .unwrap();
//     let mint_b = CreateMint::new(&mut svm, &maker)
//         .authority(&taker.pubkey())
//         .decimals(DECIMALS)
//         .send()
//         .unwrap();

//     // Create all associated token accounts upfront for clarity
//     let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
//         .owner(&maker.pubkey())
//         .send()
//         .unwrap();

//     // Mint initial token balances using litesvm-token MintTo builder
//     MintTo::new(&mut svm, &maker, &mint_a, &maker_ata_a, 10_000_000)
//         .send()
//         .unwrap();

//     // ============================================================================
//     // Test: Execute the "make" instruction to create escrow
//     // ============================================================================
//     let seed: u64 = 1234567890;
//     let (escrow, _bump) = Pubkey::find_program_address(
//         &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
//         &program_id,
//     );

//     let vault = get_associated_token_address(&escrow, &mint_a);

//     let deposit: u64 = 10_000_000;
//     let receive: u64 = 5_000_000;

//     let make_accounts = accounts::Make {
//         maker: maker.pubkey(),
//         mint_a: mint_a,
//         mint_b: mint_b,
//         maker_ata_a: maker_ata_a,
//         escrow: escrow,
//         vault: vault,
//         associated_token_program: anchor_spl::associated_token::ID,
//         system_program: system_program::ID,
//         token_program: anchor_spl::token::ID,
//     };
//     let make_data = instruction::Make {
//         seed: seed,
//         deposit: deposit,
//         receive: receive,
//     };

//     let make_instruction = Instruction {
//         program_id: program_id,
//         accounts: make_accounts.to_account_metas(None),
//         data: make_data.data(),
//     };

//     // Send make transaction
//     let tx = Transaction::new_signed_with_payer(
//         &[make_instruction],
//         Some(&maker.pubkey()),
//         &[&maker],
//         svm.latest_blockhash(),
//     );
//     svm.send_transaction(tx).unwrap();

//     println!("Escrow created successfully");

//     // Now test the refund instruction

//     let refund_accounts = accounts::Refund {
//         maker: maker.pubkey(),
//         mint_a: mint_a,
//         maker_ata_a: maker_ata_a,
//         escrow: escrow,
//         vault: vault,
//         associated_token_program: anchor_spl::associated_token::ID,
//         system_program: system_program::ID,
//         token_program: anchor_spl::token::ID,
//     };
//     let refund_data = instruction::Refund {};

//     let refund_instruction = Instruction {
//         program_id: program_id,
//         accounts: refund_accounts.to_account_metas(None),
//         data: refund_data.data(),
//     };

//     // Send refund transaction
//     let tx = Transaction::new_signed_with_payer(
//         &[refund_instruction],
//         Some(&maker.pubkey()),
//         &[&maker],
//         svm.latest_blockhash(),
//     );
//     svm.send_transaction(tx).unwrap();

//     println!("Escrow refunded successfully");
// }

// #[test]
// fn test_make_and_take() {
//     // ============================================================================
//     // Test Env Setup: Initialize environment and deploy escrow program
//     // ============================================================================
//     let mut svm = LiteSVM::new();

//     let program_keypair = read_keypair_file("target/deploy/anchor_escrow_q4_25-keypair.json").unwrap();
//     let program_id = program_keypair.pubkey();
//     let program_bytes = include_bytes!("../target/deploy/anchor_escrow_q4_25.so");

//     svm.add_program(program_id, program_bytes);

//     // ============================================================================
//     // Create and fund test accounts
//     // ============================================================================
//     let maker = Keypair::new();
//     let taker = Keypair::new();
//     svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap(); // 10 SOL
//     svm.airdrop(&taker.pubkey(), 10_000_000_000).unwrap(); // 10 SOL

//     // ============================================================================
//     // Token Setup: Create mints and token accounts
//     // Token swap flow: Maker offers mint_a tokens, wants mint_b tokens in return
//     // ============================================================================

//     // Create two token mints
//     let mint_a = CreateMint::new(&mut svm, &maker)
//         .authority(&maker.pubkey())
//         .decimals(DECIMALS)
//         .send()
//         .unwrap();
//     let mint_b = CreateMint::new(&mut svm, &maker)
//         .authority(&taker.pubkey())
//         .decimals(DECIMALS)
//         .send()
//         .unwrap();

//     // Create all associated token accounts upfront for clarity
//     let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
//         .owner(&maker.pubkey())
//         .send()
//         .unwrap();
//     let maker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_b)
//         .owner(&maker.pubkey())
//         .send()
//         .unwrap();

//     let taker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_a)
//         .owner(&taker.pubkey())
//         .send()
//         .unwrap();
//     let taker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_b)
//         .owner(&taker.pubkey())
//         .send()
//         .unwrap();

//     // Mint initial token balances using litesvm-token MintTo builder
//     MintTo::new(&mut svm, &maker, &mint_a, &maker_ata_a, 10_000_000)
//         .send()
//         .unwrap();
//     MintTo::new(&mut svm, &taker, &mint_b, &taker_ata_b, 5_000_000)
//         .send()
//         .unwrap();

//     // ============================================================================
//     // Test: Execute the "make" instruction to create escrow
//     // ============================================================================
//     let seed: u64 = 1234567890;
//     let (escrow, _bump) = Pubkey::find_program_address(
//         &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
//         &program_id,
//     );

//     let vault = get_associated_token_address(&escrow, &mint_a);

//     let deposit: u64 = 10_000_000;
//     let receive: u64 = 5_000_000;

//     let make_accounts = accounts::Make {
//         maker: maker.pubkey(),
//         mint_a: mint_a,
//         mint_b: mint_b,
//         maker_ata_a: maker_ata_a,
//         escrow: escrow,
//         vault: vault,
//         associated_token_program: anchor_spl::associated_token::ID,
//         system_program: system_program::ID,
//         token_program: anchor_spl::token::ID,
//     };
//     let make_data = instruction::Make {
//         seed: seed,
//         deposit: deposit,
//         receive: receive,
//     };

//     let make_instruction = Instruction {
//         program_id: program_id,
//         accounts: make_accounts.to_account_metas(None),
//         data: make_data.data(),
//     };

//     // Send make transaction
//     let tx = Transaction::new_signed_with_payer(
//         &[make_instruction],
//         Some(&maker.pubkey()),
//         &[&maker],
//         svm.latest_blockhash(),
//     );
//     svm.send_transaction(tx).unwrap();

//     println!("Escrow created successfully");

//     // Now test the take instruction

//     let take_accounts = accounts::Take {
//         taker: taker.pubkey(),
//         maker: maker.pubkey(),
//         mint_a: mint_a,
//         mint_b: mint_b,
//         taker_ata_a: taker_ata_a,
//         taker_ata_b: taker_ata_b,
//         maker_ata_b: maker_ata_b,
//         escrow: escrow,
//         vault: vault,
//         associated_token_program: anchor_spl::associated_token::ID,
//         system_program: system_program::ID,
//         token_program: anchor_spl::token::ID,
//     };
//     let take_data = instruction::Take {};

//     let take_instruction = Instruction {
//         program_id: program_id,
//         accounts: take_accounts.to_account_metas(None),
//         data: take_data.data(),
//     };

//     // Send take transaction
//     let tx = Transaction::new_signed_with_payer(
//         &[take_instruction],
//         Some(&taker.pubkey()),
//         &[&taker],
//         svm.latest_blockhash(),
//     );
//     svm.send_transaction(tx).unwrap();

//     println!("Escrow taken successfully");

//     // Verify escrow account was closed
//     // In LiteSVM, closed accounts might still exist with 0 lamports and 0 data
//     let escrow_closed = match svm.get_account(&escrow) {
//         None => true,
//         Some(account) => account.lamports == 0 && account.data.is_empty(),
//     };
//     assert!(escrow_closed, "Escrow account should be closed (0 lamports, 0 data)");
//     println!("\nEscrow account closed successfully");

//     // Verify vault account was closed
//     let vault_closed = match svm.get_account(&vault) {
//         None => true,
//         Some(account) => account.lamports == 0 && account.data.is_empty(),
//     };
//     assert!(vault_closed, "Vault account should be closed (0 lamports, 0 data)");
//     println!("Vault account closed successfully");

//     // Check final token balances
//     // Taker should have received tokens from mint_a
//     let taker_ata_a_state = litesvm_token::get_spl_account::<spl_token::state::Account>(&svm, &taker_ata_a).unwrap();
//     assert_eq!(taker_ata_a_state.amount, 10_000_000, "Taker should have received 1 token from mint_a");
//     println!("Taker received {} tokens from mint_a", taker_ata_a_state.amount);

//     // Taker should have sent tokens from mint_b
//     let taker_ata_b_state = litesvm_token::get_spl_account::<spl_token::state::Account>(&svm, &taker_ata_b).unwrap();
//     assert_eq!(taker_ata_b_state.amount, 0, "Taker should have sent all tokens from mint_b");
//     println!("Taker has {} tokens from mint_b (after sending)", taker_ata_b_state.amount);

//     // Maker should have received tokens from mint_b
//     let maker_ata_b_state = litesvm_token::get_spl_account::<spl_token::state::Account>(&svm, &maker_ata_b).unwrap();
//     assert_eq!(maker_ata_b_state.amount, 5_000_000, "Maker should have received 0.5 tokens from mint_b");
//     println!("Maker received {} tokens from mint_b", maker_ata_b_state.amount);

//     println!("\nTake instruction test passed successfully!");
// }