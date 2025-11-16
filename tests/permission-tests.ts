import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PrivateDex } from "../target/types/private_dex";
import {
  getGroupPda,
  getPermissionPda,
  getUserPda,
  getConfigPda,
  getLiquidityPoolPda,
  PERMISSION_PROGRAM_ID,
  DELEGATE_PROGRAM_ID,
  ER_VALIDATOR_ID,
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

// Load static keys for testing
const adminSecretKey = readFileSync("tmp/admin.json");
const adminKp = Keypair.fromSecretKey(new Uint8Array(JSON.parse(adminSecretKey.toString())));

describe("Permission Tests", () => {
  const wallet = new anchor.Wallet(adminKp);

  const provider = new anchor.AnchorProvider(
    new anchor.web3.Connection("http://localhost:8899", {
      wsEndpoint: "ws://localhost:8900",
      commitment: "confirmed",
    }),
    wallet
  );
  anchor.setProvider(provider);

  const program = new Program<PrivateDex>(privateDexIdl as any, provider);
  const configPda = getConfigPda(program.programId);

  before(async () => {
    // Airdrop SOL to admin
    try {
      const sig = await provider.connection.requestAirdrop(
        adminKp.publicKey,
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
        .rpc();
      console.log("✅ Config initialized");
    } catch (e) {
      console.log("Config may already be initialized:", e.message);
    }
  });

  it("Creates user permission", async () => {
    console.log("\n=== Test: Create User Permission ===");

    const userKp = Keypair.generate();
    
    // Airdrop to user
    const sig = await provider.connection.requestAirdrop(
      userKp.publicKey,
      LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig);

    const userPda = getUserPda(userKp.publicKey, program.programId);

    // Create user account
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
    console.log("✅ User created at", userPda.toString());

    // Create permission
    const groupId = Keypair.generate().publicKey;
    const groupPda = getGroupPda(groupId, PERMISSION_PROGRAM_ID);
    const permissionPda = getPermissionPda(userPda, groupPda, PERMISSION_PROGRAM_ID);

    await program.methods
      .createUserPermission(groupId)
      .accountsPartial({
        payer: userKp.publicKey,
        user: userKp.publicKey,
        userAccount: userPda,
        permission: permissionPda,
        group: groupPda,
        permissionProgram: PERMISSION_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([userKp])
      .rpc();

    console.log("✅ User permission created successfully");
    console.log("   Permission PDA:", permissionPda.toString());
    console.log("   Group PDA:", groupPda.toString());
  });

  it("Creates LP permission group", async () => {
    console.log("\n=== Test: Create LP Permission Group ===");

    const user1Kp = Keypair.generate();
    const user2Kp = Keypair.generate();
    const user3Kp = Keypair.generate();

    // Airdrop to users
    for (const kp of [user1Kp, user2Kp, user3Kp]) {
      const sig = await provider.connection.requestAirdrop(
        kp.publicKey,
        LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig);
    }

    // Create user accounts
    for (const kp of [user1Kp, user2Kp, user3Kp]) {
      const userPda = getUserPda(kp.publicKey, program.programId);
      await program.methods
        .createUser()
        .accountsPartial({
          sender: kp.publicKey,
          user: userPda,
          config: configPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([kp])
        .rpc();
    }
    console.log("✅ Users created");

    // Create permission group
    const groupId = Keypair.generate().publicKey;
    const groupPda = getGroupPda(groupId, PERMISSION_PROGRAM_ID);
    const users = [user1Kp.publicKey, user2Kp.publicKey, user3Kp.publicKey];

    await program.methods
      .createLpPermissionGroup(groupId)
      .accountsPartial({
        sender: user1Kp.publicKey,
        group: groupPda,
        permissionProgram: PERMISSION_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .remainingAccounts(
        users.map((user) => ({
          pubkey: user,
          isSigner: false,
          isWritable: false,
        }))
      )
      .signers([user1Kp])
      .rpc();

    console.log("✅ LP permission group created successfully");
    console.log("   Group PDA:", groupPda.toString());
    console.log("   Members:", users.length, "users");
  });

  it("Creates LP permission", async () => {
    console.log("\n=== Test: Create LP Permission ===");

    const creatorKp = Keypair.generate();

    // Airdrop to creator
    const sig = await provider.connection.requestAirdrop(
      creatorKp.publicKey,
      LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig);

    // Create two token mints
    const mintA = await createMint(
      provider.connection,
      creatorKp,
      creatorKp.publicKey,
      null,
      DECIMALS
    );

    const mintB = await createMint(
      provider.connection,
      creatorKp,
      creatorKp.publicKey,
      null,
      DECIMALS
    );

    // Create liquidity pool
    const [lpPda, lpPdaBump] = getLiquidityPoolPda(mintA, mintB, program.programId);
    const lpMintPda = PublicKey.findProgramAddressSync(
      [Buffer.from("lp_mint"), lpPda.toBuffer()],
      program.programId
    )[0];

    const vaultA = getAssociatedTokenAddressSync(mintA, configPda, true);
    const vaultB = getAssociatedTokenAddressSync(mintB, configPda, true);
    const vaultLp = getAssociatedTokenAddressSync(lpMintPda, configPda, true);

    await program.methods
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
    console.log("✅ Liquidity pool created at", lpPda.toString());

    // Create permission group
    const groupId = Keypair.generate().publicKey;
    const groupPda = getGroupPda(groupId, PERMISSION_PROGRAM_ID);

    await program.methods
      .createLpPermissionGroup(groupId)
      .accountsPartial({
        sender: creatorKp.publicKey,
        group: groupPda,
        permissionProgram: PERMISSION_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .remainingAccounts([
        {
          pubkey: creatorKp.publicKey,
          isSigner: false,
          isWritable: false,
        },
      ])
      .signers([creatorKp])
      .rpc();
    console.log("✅ LP permission group created:", groupPda.toString());

    // Create LP permission
    const permissionPda = getPermissionPda(lpPda, groupPda, PERMISSION_PROGRAM_ID);

    await program.methods
      .createLpPermission(mintA, mintB, lpPdaBump)
      .accountsPartial({
        sender: creatorKp.publicKey,
        lp: lpPda,
        permission: permissionPda,
        group: groupPda,
        permissionProgram: PERMISSION_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([creatorKp])
      .rpc();

    console.log("✅ LP permission created successfully");
    console.log("   Permission PDA:", permissionPda.toString());
    console.log("   LP PDA:", lpPda.toString());
    console.log("   Group PDA:", groupPda.toString());
  });

  it("Full permission flow", async () => {
    console.log("\n=== Test: Full Permission Flow ===");

    const user1Kp = Keypair.generate();
    const user2Kp = Keypair.generate();

    // Airdrop to users
    for (const kp of [user1Kp, user2Kp]) {
      const sig = await provider.connection.requestAirdrop(
        kp.publicKey,
        LAMPORTS_PER_SOL
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
    }
    console.log("✅ 1. Users created");

    // Create user permissions
    const groupId1 = Keypair.generate().publicKey;
    const groupId2 = Keypair.generate().publicKey;

    for (const [kp, pda, groupId] of [
      [user1Kp, user1Pda, groupId1],
      [user2Kp, user2Pda, groupId2],
    ] as const) {
      const groupPda = getGroupPda(groupId, PERMISSION_PROGRAM_ID);
      const permissionPda = getPermissionPda(pda, groupPda, PERMISSION_PROGRAM_ID);

      await program.methods
        .createUserPermission(groupId)
        .accountsPartial({
          payer: kp.publicKey,
          user: kp.publicKey,
          userAccount: pda,
          permission: permissionPda,
          group: groupPda,
          permissionProgram: PERMISSION_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([kp])
        .rpc();
    }
    console.log("✅ 2. User permissions created");

    // Create tokens and LP
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
    console.log("✅ 3. Tokens created");

    const [lpPda, lpPdaBump] = getLiquidityPoolPda(mintA, mintB, program.programId);
    const lpMintPda = PublicKey.findProgramAddressSync(
      [Buffer.from("lp_mint"), lpPda.toBuffer()],
      program.programId
    )[0];

    const vaultA = getAssociatedTokenAddressSync(mintA, configPda, true);
    const vaultB = getAssociatedTokenAddressSync(mintB, configPda, true);
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
    console.log("✅ 4. LP created");

    // Create LP permission group
    const lpGroupId = Keypair.generate().publicKey;
    const lpGroupPda = getGroupPda(lpGroupId, PERMISSION_PROGRAM_ID);

    await program.methods
      .createLpPermissionGroup(lpGroupId)
      .accountsPartial({
        sender: user1Kp.publicKey,
        group: lpGroupPda,
        permissionProgram: PERMISSION_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .remainingAccounts([
        {
          pubkey: user1Kp.publicKey,
          isSigner: false,
          isWritable: false,
        },
        {
          pubkey: user2Kp.publicKey,
          isSigner: false,
          isWritable: false,
        },
      ])
      .signers([user1Kp])
      .rpc();
    console.log("✅ 5. LP permission group created");

    // Create LP permission
    const lpPermissionPda = getPermissionPda(lpPda, lpGroupPda, PERMISSION_PROGRAM_ID);

    await program.methods
      .createLpPermission(mintA, mintB, lpPdaBump)
      .accountsPartial({
        sender: user1Kp.publicKey,
        lp: lpPda,
        permission: lpPermissionPda,
        group: lpGroupPda,
        permissionProgram: PERMISSION_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user1Kp])
      .rpc();
    console.log("✅ 6. LP permission created");

    console.log("\n🎉 Full permission flow completed successfully!");
  });
});

