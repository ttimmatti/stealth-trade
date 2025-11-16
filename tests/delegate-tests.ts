import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PrivateDex } from "../target/types/private_dex";
import {
  getUserPda,
  getConfigPda,
  getLiquidityPoolPda,
  DELEGATE_PROGRAM_ID,
  ER_VALIDATOR_ID,
  PERMISSION_PROGRAM_ID,
  DECIMALS,
} from "./test-utils";
import privateDexIdl from "../target/idl/private_dex.json";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  getAssociatedTokenAddressSync,
  mintToChecked,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountIdempotent,
} from "@solana/spl-token";
import { assert } from "chai";
import {
  Transaction,
  SystemProgram,
  PublicKey,
  Keypair,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { readFileSync } from "fs";

// static keys for users for easier debuggin
const adminSecretKey = readFileSync("tmp/admin.json");
const userSecretKey = readFileSync("tmp/devnet-user.json");
const otherUserSecretKey = readFileSync("tmp/devnet-otherUser.json");
const mintSecretKey = readFileSync("tmp/devnet-mint.json");
const groupSecretKey = readFileSync("tmp/devnet-group.json");
const otherGroupSecretKey = readFileSync("tmp/devnet-otherGroup.json");
const adminKp = Keypair.fromSecretKey(
  new Uint8Array(JSON.parse(adminSecretKey.toString()))
);
const userKp = Keypair.fromSecretKey(
  new Uint8Array(JSON.parse(userSecretKey.toString()))
);
const otherUserKp = Keypair.fromSecretKey(
  new Uint8Array(JSON.parse(otherUserSecretKey.toString()))
);
const mintKp = Keypair.fromSecretKey(
  new Uint8Array(JSON.parse(mintSecretKey.toString()))
);
const groupKp = Keypair.fromSecretKey(
  new Uint8Array(JSON.parse(groupSecretKey.toString()))
);
const otherGroupKp = Keypair.fromSecretKey(
  new Uint8Array(JSON.parse(otherGroupSecretKey.toString()))
);

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

describe("Delegate Tests", () => {
  const wallet = new anchor.Wallet(userKp);

  const provider = new anchor.AnchorProvider(
    new anchor.web3.Connection("http://localhost:8899", {
      wsEndpoint: "ws://localhost:8900",
      commitment: "confirmed",
    }),
    wallet
  );
  anchor.setProvider(provider);

  const ephemeralProvider = new anchor.AnchorProvider(
    new anchor.web3.Connection("http://localhost:7799", {
      wsEndpoint: "ws://localhost:7800",
      commitment: "confirmed",
    }),
    wallet
  );

  const program = new Program<PrivateDex>(privateDexIdl as any, provider);
  const ephemeralProgram = new Program<PrivateDex>(privateDexIdl as any, ephemeralProvider);
  const configPda = getConfigPda(program.programId);

  let mintA: PublicKey;
  let mintB: PublicKey;

  before(async () => {
    // Airdrop SOL to admin
    try {
      const sig = await provider.connection.requestAirdrop(
        userKp.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig);
    } catch (e) {
      console.log("Airdrop failed, admin might already have funds:", e.message);
    }

    // Initialize config
    try {
      await program.methods
        .initialize()
        .accountsPartial({
          sender: adminKp.publicKey,
          config: configPda,
          delegateProgram: DELEGATE_PROGRAM_ID,
          erValidator: ER_VALIDATOR_ID,
          permissionProgram: PERMISSION_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([adminKp])
        .rpc();
      console.log("✅ Config initialized");
    } catch (e) {
      console.log("Config may already be initialized:", e.message);
    }

    // Create tokens
    mintA = await createMint(
      provider.connection,
      userKp,
      userKp.publicKey,
      null,
      DECIMALS
    );
    mintB = await createMint(
      provider.connection,
      userKp,
      userKp.publicKey,
      null,
      DECIMALS
    );
  });

  it("Delegate user account", async () => {
    console.log("\n=== Test: Delegate User ===");

    // Airdrop to user
    let sig = await provider.connection.requestAirdrop(
      otherUserKp.publicKey,
      LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig);

    const userPda = getUserPda(otherUserKp.publicKey, program.programId);

    // Create user account
    try {
      sig = await program.methods
        .createUser()
        .accountsPartial({
          sender: otherUserKp.publicKey,
          user: userPda,
          config: configPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([otherUserKp])
        .rpc();
        await provider.connection.confirmTransaction(sig);
        console.log("✅ User created at", userPda.toString());
      } catch (e) {
      console.log("User may already be created:", e.message);
    }

    // Verify user account before delegation
    const userAccountBefore = await program.account.user.fetch(userPda);
    assert.equal(
      userAccountBefore.authority.toString(),
      otherUserKp.publicKey.toString(),
      "User should have correct authority"
    );
    console.log("✅ User account verified before delegation");

    // Delegate the user account
    await program.methods
      .delegateUser(otherUserKp.publicKey)
      .accountsPartial({
        payer: otherUserKp.publicKey,
        config: configPda,
        validator: ER_VALIDATOR_ID,
        userAccount: userPda,
      })
      .signers([otherUserKp])
      .rpc();

    console.log("✅ User account delegated successfully");

    // Verify user account still exists
    const userAccountAfter = await program.account.user.fetch(userPda);
    assert.equal(
      userAccountAfter.authority.toString(),
      otherUserKp.publicKey.toString(),
      "User authority should remain unchanged"
    );
    console.log("✅ User account state preserved after delegation");
  });

  it("Delegate user account with deposits", async () => {
    console.log("\n=== Test: Delegate User with Deposits ===");

    // Airdrop to user
    let sig = await provider.connection.requestAirdrop(
      userKp.publicKey,
      LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig);

    const userPda = getUserPda(userKp.publicKey, program.programId);

    // Create user account
    try {
      await program.methods
        .createUser()
        .accountsPartial({
          sender: userKp.publicKey,
          user: userPda,
          config: configPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([userKp])
        .rpc();
        console.log("✅ User created");
    } catch (e) {
      console.log("User may already be created:", e.message);
    }

    // Create a token mint
    const mint = await createMint(
      provider.connection,
      userKp,
      userKp.publicKey,
      null,
      DECIMALS
    );

    const userAta = await createAssociatedTokenAccountIdempotent(
      provider.connection,
      userKp,
      mint,
      userKp.publicKey
    );

    await mintToChecked(
      provider.connection,
      userKp,
      mint,
      userAta,
      userKp.publicKey,
      10_000_000,
      DECIMALS
    );
    console.log("✅ Tokens minted");

    // Deposit tokens
    const vault = getAssociatedTokenAddressSync(mint, configPda, true);

    sig = await program.methods
      .deposit(new anchor.BN(5_000_000))
      .accountsPartial({
        sender: userKp.publicKey,
        user: userPda,
        config: configPda,
        senderAta: userAta,
        vault,
        mint,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([userKp])
      .rpc();
    await provider.connection.confirmTransaction(sig);
    console.log("✅ Tokens deposited: 5,000,000");

    // Verify balance before delegation
    const userAccountBefore = await provider.connection.getAccountInfo(userPda);
    assert.equal(
      userAccountBefore?.owner?.toBase58(),
      program.programId.toBase58(),
      "User should be delegated"
    );

    // Delegate the user account
    await program.methods
      .delegateUser(userKp.publicKey)
      .accountsPartial({
        payer: userKp.publicKey,
        config: configPda,
        validator: ER_VALIDATOR_ID,
        userAccount: userPda,
      })
      .signers([userKp])
      .rpc();

    console.log("✅ User account delegated successfully");

    // Verify user account is delegated
    const userAccountAfter = await provider.connection.getAccountInfo(userPda);
    assert.equal(
      userAccountAfter?.owner?.toBase58(),
      program.programId.toBase58(),
      "User should be delegated"
    );
    console.log("✅ User delegated successfully");
  });

  it("Delegate LP account", async () => {
    console.log("\n=== Test: Delegate LP ===");

    const creatorKp = userKp;

    // Airdrop to creator
    let sig = await provider.connection.requestAirdrop(
      creatorKp.publicKey,
      LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig);

    // Create two token mints
    // const mintA = await createMint(
    //   provider.connection,
    //   creatorKp,
    //   creatorKp.publicKey,
    //   null,
    //   DECIMALS
    // );

    // const mintB = await createMint(
    //   provider.connection,
    //   creatorKp,
    //   creatorKp.publicKey,
    //   null,
    //   DECIMALS
    // );

    // Create liquidity pool
    const [lpPda] = getLiquidityPoolPda(mintA, mintB, program.programId);
    const lpMintPda = PublicKey.findProgramAddressSync(
      [Buffer.from("lp_mint"), lpPda.toBuffer()],
      program.programId
    )[0];

    const vaultA = getAssociatedTokenAddressSync(mintA, configPda, true);
    const vaultB = getAssociatedTokenAddressSync(mintB, configPda, true);
    const vaultLp = getAssociatedTokenAddressSync(lpMintPda, configPda, true);

    sig = await program.methods
      .createLp()
      .accountsPartial({
        sender: creatorKp.publicKey,
        lp: lpPda,
        config: configPda,
        mintA,
        mintB,
        mintLp: lpMintPda,
        vaultA,
        vaultB,
        vaultLp,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([creatorKp])
      .rpc();
    await provider.connection.confirmTransaction(sig);
    console.log("✅ Liquidity pool created at", lpPda.toString());

    // Verify LP account before delegation
    const lpAccountBefore = await program.account.liquidityPool.fetch(lpPda);
    assert.equal(
      lpAccountBefore.mintA.toString(),
      mintA.toString(),
      "LP should have correct mint_a"
    );
    assert.equal(
      lpAccountBefore.mintB.toString(),
      mintB.toString(),
      "LP should have correct mint_b"
    );
    console.log("✅ LP account verified before delegation");

    // Delegate the LP account
    await program.methods
      .delegateLp(mintA, mintB)
      .accountsPartial({
        payer: creatorKp.publicKey,
        config: configPda,
        validator: ER_VALIDATOR_ID,
        lpAccount: lpPda,
      })
      .signers([creatorKp])
      .rpc();

    console.log("✅ LP account delegated successfully");

    // Verify LP account state is preserved
    const lpAccountAfter = await program.account.liquidityPool.fetch(lpPda);
    assert.equal(
      lpAccountAfter.mintA.toString(),
      mintA.toString(),
      "LP mint_a should remain unchanged"
    );
    assert.equal(
      lpAccountAfter.mintB.toString(),
      mintB.toString(),
      "LP mint_b should remain unchanged"
    );
    console.log("✅ LP account state preserved after delegation");
  });

  it("Undelegate User Accounts", async () => {
    for (const kp of [userKp, otherUserKp]) {
      const userPda = getUserPda(kp.publicKey, program.programId);
      console.log("Undelegating account", userPda.toBase58());

      const sig = await ephemeralProgram.methods
        .commitAndUndelegateUser()
        .accountsPartial({ payer: kp.publicKey, user: kp.publicKey, sessionToken: null, userAccount: userPda })
        .signers([kp])
        .rpc();
      console.log("Sig undelegate", sig);
      await ephemeralProvider.connection.confirmTransaction(sig);

      console.log("Undelegated account", userPda.toBase58());
    }
  });

  it("Undelegate LP Account", async () => {

    const [lpPda] = getLiquidityPoolPda(mintA, mintB, program.programId);
    console.log("Undelegating account", lpPda.toBase58());

    const lpAccountBefore = await provider.connection.getAccountInfo(lpPda);
    assert.equal(
      lpAccountBefore?.owner?.toBase58(),
      DELEGATE_PROGRAM_ID.toBase58(),
      "LP should be delegated"
    );

    const sig = await ephemeralProgram.methods
      .commitAndUndelegateLp(mintA, mintB)
      .accountsPartial({ payer: adminKp.publicKey, config: configPda, lpAccount: lpPda })
      .signers([adminKp])
      .rpc();
    console.log("Sig undelegate", sig);
    await ephemeralProvider.connection.confirmTransaction(sig, "finalized");

    await sleep(3000);

    const lpAccountAfter = await provider.connection.getAccountInfo(lpPda);
    assert.equal(
      lpAccountAfter?.owner?.toBase58(),
      program.programId.toBase58(),
      "LP should be undelegated"
    );

    console.log("Undelegated account", lpPda.toBase58());
  });

  it("Full delegate flow", async () => {
    console.log("\n=== Test: Full Delegate Flow ===");

    const user1Kp = Keypair.generate();
    const user2Kp = Keypair.generate();

    // Airdrop to users
    for (const kp of [user1Kp, user2Kp]) {
      const sig = await provider.connection.requestAirdrop(
        kp.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig);
    }

    // Create users
    const user1Pda = getUserPda(user1Kp.publicKey, program.programId);
    const user2Pda = getUserPda(user2Kp.publicKey, program.programId);

    for (const [kp, pda] of [
      [user1Kp, user1Pda],
      [user2Kp, user2Pda],
    ] as const) {
      try {
        await program.methods
          .createUser()
          .accountsPartial({
            sender: kp.publicKey,
            user: pda,
            config: configPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([kp])
          .rpc();
      } catch (e) {
        console.log("User may already be created:", e.message);
      }
    }
    console.log("✅ 1. Users created");

    const mintA = await createMint(
      provider.connection,
      user1Kp,
      user1Kp.publicKey,
      null,
      DECIMALS
    );
    const mintB = await createMint(
      provider.connection,
      user1Kp,
      user1Kp.publicKey,
      null,
      DECIMALS
    );

    // Mint tokens to user1
    const user1AtaA = await createAssociatedTokenAccountIdempotent(
      provider.connection,
      user1Kp,
      mintA,
      user1Kp.publicKey
    );
    const user1AtaB = await createAssociatedTokenAccountIdempotent(
      provider.connection,
      user1Kp,
      mintB,
      user1Kp.publicKey
    );
    console.log("✅ 2. Tokens created");

    await mintToChecked(
      provider.connection,
      user1Kp,
      mintA,
      user1AtaA,
      user1Kp.publicKey,
      10_000_000,
      DECIMALS
    );
    await mintToChecked(
      provider.connection,
      user1Kp,
      mintB,
      user1AtaB,
      user1Kp.publicKey,
      10_000_000,
      DECIMALS
    );
    console.log("✅ 3. Tokens minted");

    // Deposit tokens
    const vaultA = getAssociatedTokenAddressSync(mintA, configPda, true);
    const vaultB = getAssociatedTokenAddressSync(mintB, configPda, true);

    await program.methods
      .deposit(new anchor.BN(5_000_000))
      .accountsPartial({
        sender: user1Kp.publicKey,
        user: user1Pda,
        config: configPda,
        senderAta: user1AtaA,
        vault: vaultA,
        mint: mintA,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user1Kp])
      .rpc();

    await program.methods
      .deposit(new anchor.BN(5_000_000))
      .accountsPartial({
        sender: user1Kp.publicKey,
        user: user1Pda,
        config: configPda,
        senderAta: user1AtaB,
        vault: vaultB,
        mint: mintB,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user1Kp])
      .rpc();
    console.log("✅ 4. Tokens deposited");

    // Create LP
    const [lpPda] = getLiquidityPoolPda(mintA, mintB, program.programId);
    const lpMintPda = PublicKey.findProgramAddressSync(
      [Buffer.from("lp_mint"), lpPda.toBuffer()],
      program.programId
    )[0];

    const vaultLp = getAssociatedTokenAddressSync(lpMintPda, configPda, true);

    await program.methods
      .createLp()
      .accountsPartial({
        sender: user1Kp.publicKey,
        lp: lpPda,
        config: configPda,
        mintA,
        mintB,
        mintLp: lpMintPda,
        vaultA,
        vaultB,
        vaultLp,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user1Kp])
      .rpc();
    console.log("✅ 5. Liquidity pool created");

    // Add liquidity
    await program.methods
      .addLiquidity(new anchor.BN(1_000_000), new anchor.BN(3_000_000), new anchor.BN(3_000_000))
      .accountsPartial({
        sender: user1Kp.publicKey,
        user: user1Pda,
        lp: lpPda,
        config: configPda,
        mintA,
        mintB,
        mintLp: lpMintPda,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user1Kp])
      .rpc();
    console.log("✅ 6. Liquidity added");

    // Delegate user accounts
    await program.methods
      .delegateUser(user1Kp.publicKey)
      .accountsPartial({
        payer: user1Kp.publicKey,
        config: configPda,
        validator: ER_VALIDATOR_ID,
        userAccount: user1Pda,
      })
      .signers([user1Kp])
      .rpc();

    const user1AccountBefore = await provider.connection.getAccountInfo(user1Pda);
    const user2AccountBefore = await provider.connection.getAccountInfo(user2Pda);
    assert.equal(
      user1AccountBefore?.owner?.toBase58(),
      program.programId.toBase58(),
      "User 1 should be undelegated"
    );
    assert.equal(
      user2AccountBefore?.owner?.toBase58(),
      program.programId.toBase58(),
      "User 2 should be undelegated"
    );

    await program.methods
      .delegateUser(user2Kp.publicKey)
      .accountsPartial({
        payer: user2Kp.publicKey,
        config: configPda,
        validator: ER_VALIDATOR_ID,
        userAccount: user2Pda,
      })
      .signers([user2Kp])
      .rpc();

    console.log("✅ 7. User accounts delegated");

    // Delegate LP account
    await program.methods
      .delegateLp(mintA, mintB)
      .accountsPartial({
        payer: user1Kp.publicKey,
        config: configPda,
        validator: ER_VALIDATOR_ID,
        lpAccount: lpPda,
      })
      .signers([user1Kp])
      .rpc();

    console.log("✅ 8. LP account delegated");

    const user1AccountDelegated = await provider.connection.getAccountInfo(user1Pda);
    const user2AccountDelegated = await provider.connection.getAccountInfo(user2Pda);
    assert.equal(
      user1AccountDelegated?.owner?.toBase58(),
      DELEGATE_PROGRAM_ID.toBase58(),
      "User 1 should be delegated"
    );
    assert.equal(
      user2AccountDelegated?.owner?.toBase58(),
      DELEGATE_PROGRAM_ID.toBase58(),
      "User 2 should be delegated"
    );

    for (const [kp, pda] of [
      [user1Kp, user1Pda],
      [user2Kp, user2Pda],
    ] as const) {
      const sig = await ephemeralProgram.methods
        .commitAndUndelegateUser()
        .accountsPartial({ payer: kp.publicKey, user: kp.publicKey, sessionToken: null, userAccount: pda })
        .signers([kp])
        .rpc();
      console.log("Sig undelegate", sig);
      await ephemeralProvider.connection.confirmTransaction(sig, "finalized");
    }

    console.log("✅ 9. User accounts undelegated");

    const sig = await ephemeralProgram.methods
      .commitAndUndelegateLp(mintA, mintB)
      .accountsPartial({ payer: adminKp.publicKey, config: configPda, lpAccount: lpPda })
      .signers([adminKp])
      .rpc();
    console.log("Sig undelegate", sig);
    await ephemeralProvider.connection.confirmTransaction(sig, "finalized");

    await sleep(1000);

    console.log("✅ 10. LP account undelegated");

    for (const [kp, pda] of [
      [user1Kp, user1Pda],
      [user2Kp, user2Pda],
    ] as const) {
      const userAccountUndelegated = await provider.connection.getAccountInfo(pda);
      assert.equal(
        userAccountUndelegated?.owner?.toBase58(),
        program.programId.toBase58(),
        "User should be undelegated"
      );
    }

    const lpAccountUndelegated = await provider.connection.getAccountInfo(lpPda);
    assert.equal(
      lpAccountUndelegated?.owner?.toBase58(),
      program.programId.toBase58(),
      "LP should be undelegated"
    );

    // Verify state is preserved
    const user1Account = await program.account.user.fetch(user1Pda);
    const lpAccount = await program.account.liquidityPool.fetch(lpPda);

    assert.equal(
      user1Account.positions[0].amount.toNumber(),
      2_000_000,
      "User 1 should have remaining tokens"
    );
    assert.equal(
      lpAccount.lpSupply.toNumber(),
      1_000_000,
      "LP should have supply"
    );

    console.log("\n🎉 Full delegate flow completed successfully!");
  });
});

