import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PrivateDex } from "../target/types/private_dex";
import privateDexIdl from "../target/idl/private_dex.json";
import {
  groupPdaFromId,
  PERMISSION_PROGRAM_ID,
  permissionPdaFromAccount,
} from "@magicblock-labs/ephemeral-rollups-sdk/privacy";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountIdempotent,
  createMint,
  getAccount,
  getAssociatedTokenAddressSync,
  mintToChecked,
  TOKEN_PROGRAM_ID,
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
import { SessionTokenManager } from "@magicblock-labs/gum-sdk";
import { DEVNET_RPC_URL } from "./config";
import { getAuthToken } from "./tee-getAuthToken";
import { ER_VALIDATOR_ID, DELEGATE_PROGRAM_ID, LIQUIDITY_POOL_SEED, USER_SEED, CONFIG_SEED, LP_MINT_SEED } from "./test-utils";

const DEVNET_EPHEMERAL_TEE_URL = "https://tee.magicblock.app/";

// tee validator
const TEE_DEVNET_VALIDATOR = new PublicKey(
  "FnE6VJT5QNZdedZPnCoLsARgBwoE6DeJNjBs2H1gySXA"
);
// localnet validator
const LOCALNET_ER_VALIDATOR = new PublicKey(
  "mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev"
);

const isLocalnet = true;
const erValidator = isLocalnet ? LOCALNET_ER_VALIDATOR : TEE_DEVNET_VALIDATOR;

// static keys for users for easier debuggin
const adminSecretKey = readFileSync("tmp/admin.json");
const userSecretKey = readFileSync("tmp/devnet-user.json");
const otherUserSecretKey = readFileSync("tmp/devnet-otherUser.json");
const mintASecretKey = readFileSync("tmp/devnet-mint-a.json");
const mintBSecretKey = readFileSync("tmp/devnet-mint-b.json");
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
const mintAKp = Keypair.fromSecretKey(
  new Uint8Array(JSON.parse(mintASecretKey.toString()))
);
const mintBKp = Keypair.fromSecretKey(
  new Uint8Array(JSON.parse(mintBSecretKey.toString()))
);
const groupKp = Keypair.fromSecretKey(
  new Uint8Array(JSON.parse(groupSecretKey.toString()))
);
const otherGroupKp = Keypair.fromSecretKey(
  new Uint8Array(JSON.parse(otherGroupSecretKey.toString()))
);

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

describe("private-dex-tee-devnet", () => {
  const wallet = new anchor.Wallet(userKp);

  const provider = new anchor.AnchorProvider(
    new anchor.web3.Connection(isLocalnet ? "http://localhost:8899" : DEVNET_RPC_URL, {
      commitment: "confirmed",
    }),
    wallet
  );
  let ephemeralProvider: anchor.AnchorProvider;
  anchor.setProvider(provider);

  const program = new Program<PrivateDex>(privateDexIdl, provider);
  let ephemeralProgram: Program<PrivateDex>;

  const user = userKp.publicKey;
  const otherUser = otherUserKp.publicKey;
  let tokenMintA: PublicKey,
    tokenMintB: PublicKey,
    lpMint: PublicKey,
    userTokenAAccount: PublicKey,
    userTokenBAccount: PublicKey,
    userLPAccount: PublicKey,
    otherUserTokenAAccount: PublicKey,
    otherUserTokenBAccount: PublicKey,
    vaultAAccount: PublicKey,
    vaultBAccount: PublicKey,
    vaultLPAccount: PublicKey;
  const initialAmount = 10_000_000;
  const groupId = groupKp.publicKey;
  const otherGroupId = otherGroupKp.publicKey;
  let configPda: PublicKey;
  let userPda: PublicKey, otherUserPda: PublicKey;
  let vaultA: PublicKey, vaultB: PublicKey, vaultLP: PublicKey
  let lpPda: PublicKey;
  let sessionKp: Keypair, sessionToken: PublicKey;
  let otherSessionKp: Keypair, otherSessionToken: PublicKey;
  let userTeeProvider: anchor.AnchorProvider, otherUserTeeProvider: anchor.AnchorProvider;
  let userTeeProgram: Program<PrivateDex>, otherUserTeeProgram: Program<PrivateDex>;

  const sessionManager = new SessionTokenManager(wallet, provider.connection);

  before(async () => {
    ephemeralProvider = await getPrivateRollupProvider(wallet);
    ephemeralProgram = new Program<PrivateDex>(
      privateDexIdl,
      ephemeralProvider
    );
    userTeeProvider = await getPrivateRollupProvider(new anchor.Wallet(userKp));
    otherUserTeeProvider = await getPrivateRollupProvider(new anchor.Wallet(otherUserKp));
    userTeeProgram = new Program<PrivateDex>(privateDexIdl, userTeeProvider);
    otherUserTeeProgram = new Program<PrivateDex>(privateDexIdl, otherUserTeeProvider);

    const faucet = anchor.Wallet.local();

    // Airdrop SOL to the users
    for (const kp of [userKp, otherUserKp]) {
      let balance = await provider.connection.getBalance(kp.publicKey);
      if (balance > 0.1 * LAMPORTS_PER_SOL) continue;

      const tx = new Transaction().add(
        SystemProgram.transfer({
          fromPubkey: faucet.publicKey,
          toPubkey: kp.publicKey,
          lamports: 0.1 * LAMPORTS_PER_SOL,
        })
      );
      tx.recentBlockhash = (
        await provider.connection.getLatestBlockhash()
      ).blockhash;
      tx.feePayer = faucet.publicKey;
      let signedTx = await faucet.signTransaction(tx);
      let rawTx = signedTx.serialize();
      let sig = await provider.connection.sendRawTransaction(rawTx);
      await provider.connection.confirmTransaction(sig);
    }

    let balance = await provider.connection.getBalance(userKp.publicKey);
    console.log("Balance", balance);
    while (balance === 0) {
      console.log("Airdropping...");
      await new Promise((resolve) => setTimeout(resolve, 1000));
      balance = await provider.connection.getBalance(userKp.publicKey);
    }
    if (balance === 0) throw new Error("airdrop failed...");

    tokenMintA = mintAKp.publicKey;
    if ((await provider.connection.getAccountInfo(tokenMintA)) === null) {
      console.log("Creating mint...");
      tokenMintA = await createMint(
        provider.connection,
        userKp,
        user,
        null,
        6,
        mintAKp,
        undefined,
        TOKEN_PROGRAM_ID
      );
    }

    tokenMintB = mintBKp.publicKey;
    if ((await provider.connection.getAccountInfo(tokenMintB)) === null) {
      console.log("Creating mint...");
      tokenMintB = await createMint(
        provider.connection,
        userKp,
        user,
        null,
        6,
        mintBKp,
        undefined,
        TOKEN_PROGRAM_ID
      );
    }

    while ((await provider.connection.getAccountInfo(tokenMintA)) === null || (await provider.connection.getAccountInfo(tokenMintB)) === null) {
      console.log("Waiting for mint to be created...");
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    configPda = PublicKey.findProgramAddressSync(
      [CONFIG_SEED],
      program.programId
    )[0];
    userPda = PublicKey.findProgramAddressSync(
      [USER_SEED, user.toBuffer()],
      program.programId
    )[0];
    otherUserPda = PublicKey.findProgramAddressSync(
      [
        USER_SEED,
        otherUser.toBuffer(),
      ],
      program.programId
    )[0];
    lpPda = PublicKey.findProgramAddressSync(
      [LIQUIDITY_POOL_SEED, tokenMintA.toBuffer(), tokenMintB.toBuffer()],
      program.programId
    )[0];
    lpMint = PublicKey.findProgramAddressSync(
      [LP_MINT_SEED, lpPda.toBuffer()],
      program.programId
    )[0];
    vaultAAccount = getAssociatedTokenAddressSync(
      tokenMintA,
      configPda,
      true,
      TOKEN_PROGRAM_ID
    );
    vaultBAccount = getAssociatedTokenAddressSync(
      tokenMintB,
      configPda,
      true,
      TOKEN_PROGRAM_ID
    );
    vaultLPAccount = getAssociatedTokenAddressSync(
      lpMint,
      configPda,
      true,
      TOKEN_PROGRAM_ID
    );

    userTokenAAccount = getAssociatedTokenAddressSync(
      tokenMintA,
      userKp.publicKey,
      false,
      TOKEN_PROGRAM_ID
    );
    userTokenBAccount = getAssociatedTokenAddressSync(
      tokenMintB,
      userKp.publicKey,
      false,
      TOKEN_PROGRAM_ID
    );
    userLPAccount = getAssociatedTokenAddressSync(
      lpMint,
      userKp.publicKey,
      false,
      TOKEN_PROGRAM_ID
    );
    for (const [kp, mint, ata] of [
      [userKp, tokenMintA, userTokenAAccount],
      [userKp, tokenMintB, userTokenBAccount],
      // [userKp, lpMint, userLPAccount],
      // [otherUserKp, tokenMintA, otherUserTokenAAccount],
      // [otherUserKp, tokenMintB, otherUserTokenBAccount],
    ]) {
      if ((await provider.connection.getAccountInfo(ata as PublicKey)) === null) {
        console.log("Creating user token account...");
        await createAssociatedTokenAccountIdempotent(
          provider.connection,
          kp as Keypair,
          mint as PublicKey,
          (kp as Keypair).publicKey,
          undefined,
          TOKEN_PROGRAM_ID
        );
      }
      while (
        (await provider.connection.getAccountInfo(ata as PublicKey)) === null
      ) {
        console.log("Waiting for token account to be created...");
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
    }

    // Mint tokens to the user
    let userTokenAAta = await provider.connection.getTokenAccountBalance(
      userTokenAAccount
    );
    if (Number(userTokenAAta.value.amount) < 5_000_000) {
      console.log("Minting tokens A to user...");
      await mintToChecked(
        provider.connection,
        userKp,
        tokenMintA,
        userTokenAAccount,
        user,
        new anchor.BN(initialAmount) as any,
        6,
        undefined,
        undefined,
        TOKEN_PROGRAM_ID
      );
    }

    let userTokenBAta = await provider.connection.getTokenAccountBalance(
      userTokenBAccount
    );
    if (Number(userTokenBAta.value.amount) < 5_000_000) {
      console.log("Minting tokens B to user...");
      await mintToChecked(
        provider.connection,
        userKp,
        tokenMintB,
        userTokenBAccount,
        user,
        new anchor.BN(initialAmount) as any,
        6,
        undefined,
        undefined,
        TOKEN_PROGRAM_ID
      );
    }

    console.log("User token A account", userTokenAAccount.toBase58());
    console.log("User token B account", userTokenBAccount.toBase58());
    console.log("User LP account", userLPAccount.toBase58());
    console.log("Vault A account", vaultAAccount.toBase58());
    console.log("Vault B account", vaultBAccount.toBase58());
    console.log("Vault LP account", vaultLPAccount.toBase58());
    console.log("LP PDA", lpPda.toBase58());
    console.log("User PDA", userPda.toBase58());
    console.log("Other user PDA", otherUserPda.toBase58());
    console.log("Token mint A", tokenMintA.toBase58());
    console.log("Token mint B", tokenMintB.toBase58());
    console.log("LP mint", lpMint.toBase58());
  });

  it("Initialize config", async () => {
    if ((await provider.connection.getAccountInfo(configPda)) === null) {
      let sig = await program.methods
        .initialize()
        .accountsStrict({
          sender: adminKp.publicKey,
          config: configPda,
          delegateProgram: DELEGATE_PROGRAM_ID,
          erValidator: isLocalnet ? LOCALNET_ER_VALIDATOR : TEE_DEVNET_VALIDATOR,
          permissionProgram: PERMISSION_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([adminKp])
        .rpc();
      await provider.connection.confirmTransaction(sig);
      console.log("Sig initialize config", sig);
    } else {
      console.log("Config already initialized");
    }
  });

  it("Initialize users", async () => {
    if ((await provider.connection.getAccountInfo(userPda)) !== null) {
      console.log("Users already initialized");
      return;
    }

    let sig = await program.methods
      .createUser()
      .accountsStrict({
        sender: user,
        user: userPda,
        config: configPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    await provider.connection.confirmTransaction(sig);
    console.log("Sig create user", sig);

    let userAccount = await program.account.user.fetch(userPda);
    assert.equal(userAccount.authority.toBase58(), user.toBase58());

    sig = await program.methods
      .createUser()
      .accountsStrict({
        sender: otherUser,
        user: otherUserPda,
        config: configPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([otherUserKp])
      .rpc();
    await provider.connection.confirmTransaction(sig);
    console.log("Sig create other user", sig);

    userAccount = await program.account.user.fetch(otherUserPda);
    assert.equal(userAccount.authority.toBase58(), otherUser.toBase58());
  });

  // deposits
  it("Deposit", async () => {
    const userBefore = await program.account.user.fetch(userPda);
    let mintAPositionBefore = userBefore.positions.find(p => p.mint.toBase58() === tokenMintA.toBase58());
    if (mintAPositionBefore === undefined) {
      mintAPositionBefore = userBefore.positions[userBefore.positions.length - 1];
    }

    let sig = await program.methods
      .deposit(new anchor.BN(5_000_000))
      .accountsStrict({
        sender: user,
        config: configPda,
        user: userPda,
        senderAta: userTokenAAccount,
        vault: vaultAAccount,
        mint: tokenMintA,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    await provider.connection.confirmTransaction(sig);
    console.log("Sig deposit", sig);

    const userDeposit = await program.account.user.fetch(userPda);
    const mintAPositionAfter = userDeposit.positions.find(p => p.mint.toBase58() === tokenMintA.toBase58());
    assert.equal(mintAPositionAfter.amount.toNumber(), mintAPositionBefore.amount.toNumber() + 5_000_000);

    let mintBPositionBefore = userBefore.positions.find(p => p.mint.toBase58() === tokenMintB.toBase58());
    if (mintBPositionBefore === undefined) {
      mintBPositionBefore = userBefore.positions[userBefore.positions.length - 1];
    }

    sig = await program.methods
      .deposit(new anchor.BN(5_000_000))
      .accountsStrict({
        sender: user,
        config: configPda,
        user: userPda,
        senderAta: userTokenBAccount,
        vault: vaultBAccount,
        mint: tokenMintB,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    await provider.connection.confirmTransaction(sig);
    console.log("Sig deposit B", sig);

    const userAfter = await program.account.user.fetch(userPda);
    const mintBPositionAfter = userAfter.positions.find(p => p.mint.toBase58() === tokenMintB.toBase58());
    assert.equal(mintBPositionAfter.amount.toNumber(), mintBPositionBefore.amount.toNumber() + 5_000_000);
  });

  // create lp
  it("Create LP", async () => {
    if ((await provider.connection.getAccountInfo(lpPda)) !== null) {
      console.log("LP already created");
      return;
    }

    let sig = await program.methods
      .createLp()
      .accountsPartial({
        sender: user,
        config: configPda,
        mintA: tokenMintA,
        mintB: tokenMintB,
        lp: lpPda,
        mintLp: lpMint,
        vaultA: vaultAAccount,
        vaultB: vaultBAccount,
        vaultLp: vaultLPAccount,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ skipPreflight: true });
    await provider.connection.confirmTransaction(sig);
    console.log("Sig create LP", sig);

    const lp = await program.account.liquidityPool.fetch(lpPda);
    assert.equal(lp.mintA.toBase58(), tokenMintA.toBase58());
    assert.equal(lp.mintB.toBase58(), tokenMintB.toBase58());
    assert.equal(lp.lpMint.toBase58(), lpMint.toBase58());
    assert.equal(lp.vaultA.toBase58(), vaultAAccount.toBase58());
    assert.equal(lp.vaultB.toBase58(), vaultBAccount.toBase58());
  });

  // add lp
  it("Add Liquidity", async () => {
    const userBefore = await program.account.user.fetch(userPda);
    let mintAPositionBefore = userBefore.positions.find(p => p.mint.toBase58() === tokenMintA.toBase58());
    if (!mintAPositionBefore) {
      throw new Error("Mint A position not found");
    }
    let mintBPositionBefore = userBefore.positions.find(p => p.mint.toBase58() === tokenMintB.toBase58());
    if (!mintBPositionBefore) {
      throw new Error("Mint B position not found");
    }
    let lpMintPositionBefore = userBefore.positions.find(p => p.mint.toBase58() === lpMint.toBase58());
    if (lpMintPositionBefore === undefined) {
      lpMintPositionBefore = userBefore.positions[userBefore.positions.length - 1];
    }

    const lpBefore = await program.account.liquidityPool.fetch(lpPda);

    let sig = await program.methods
      .addLiquidity(new anchor.BN(500_000), new anchor.BN(500_000), new anchor.BN(500_000))
      .accountsStrict({
        payer: user,
        user: user,
        userAccount: userPda,
        config: configPda,
        mintA: tokenMintA,
        mintB: tokenMintB,
        lp: lpPda,
        mintLp: lpMint,
        sessionToken: null,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    await provider.connection.confirmTransaction(sig);
    console.log("Sig add liquidity", sig);

    const userAfter = await program.account.user.fetch(userPda);
    const mintAPositionAfter = userAfter.positions.find(p => p.mint.toBase58() === tokenMintA.toBase58());
    assert.equal(mintAPositionAfter.amount.toNumber(), mintAPositionBefore.amount.toNumber() - 500_000);
    const mintBPositionAfter = userAfter.positions.find(p => p.mint.toBase58() === tokenMintB.toBase58());
    assert.equal(mintBPositionAfter.amount.toNumber(), mintBPositionBefore.amount.toNumber() - 500_000);
    const lpMintPositionAfter = userAfter.positions.find(p => p.mint.toBase58() === lpMint.toBase58());
    assert.equal(lpMintPositionAfter.amount.toNumber(), lpMintPositionBefore.amount.toNumber() + 500_000);

    const lpAfter = await program.account.liquidityPool.fetch(lpPda);
    assert.equal(lpAfter.virtualReserveA.toNumber(), lpBefore.virtualReserveA.toNumber() + 500_000);
    assert.equal(lpAfter.virtualReserveB.toNumber(), lpBefore.virtualReserveB.toNumber() + 500_000);
    assert.equal(lpAfter.lpSupply.toNumber(), lpBefore.lpSupply.toNumber() + 500_000);
  });

  it("Create User Permission", async () => {
    for (const { userAccount, kp, id } of [
      { userAccount: userPda, kp: userKp, id: groupId },
      { userAccount: otherUserPda, kp: otherUserKp, id: otherGroupId },
    ]) {
      const permission = permissionPdaFromAccount(userAccount);
      console.log("Permission", permission.toBase58());
      const group = groupPdaFromId(id);
      console.log("Group", group.toBase58());

      if ((await provider.connection.getAccountInfo(permission)) !== null) {
        console.log("Permission already created");
        continue;
      }

      const sig = await program.methods
        .createUserPermission(id)
        .accountsStrict({
          payer: kp.publicKey,
          user: kp.publicKey,
          userAccount,
          permission,
          group,
          permissionProgram: PERMISSION_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([kp])
        .rpc();
      await provider.connection.confirmTransaction(sig);
      console.log("Sig create permission", sig);
    }
  });

  it("Create session", async () => {
    sessionKp = Keypair.generate();
    sessionToken = PublicKey.findProgramAddressSync(
      [
        Buffer.from("session_token"),
        program.programId.toBuffer(),
        sessionKp.publicKey.toBuffer(),
        wallet.publicKey.toBuffer(),
      ],
      sessionManager.program.programId
    )[0];
    const sig = await sessionManager.program.methods
      .createSession(true, null, null)
      .accountsPartial({
        sessionToken,
        sessionSigner: sessionKp.publicKey,
        authority: wallet.publicKey,
        targetProgram: program.programId,
      })
      .signers([userKp, sessionKp])
      .rpc();
    await provider.connection.confirmTransaction(sig);
    console.log("Sig create session", sig);

    const session = await sessionManager.program.account.sessionToken.fetch(
      sessionToken
    );
    console.log("Session", session);

    otherSessionKp = Keypair.generate();
    otherSessionToken = PublicKey.findProgramAddressSync(
      [
        Buffer.from("session_token"),
        program.programId.toBuffer(),
        otherSessionKp.publicKey.toBuffer(),
        otherUserKp.publicKey.toBuffer(),
      ],
      sessionManager.program.programId
    )[0];
    const otherSessionSig = await sessionManager.program.methods
      .createSession(true, null, null)
      .accountsPartial({
        sessionToken: otherSessionToken,
        sessionSigner: otherSessionKp.publicKey,
        authority: otherUserKp.publicKey,
        targetProgram: program.programId,
      })
      .signers([otherUserKp, otherSessionKp])
      .rpc();
    await provider.connection.confirmTransaction(otherSessionSig);
    console.log("Sig create session", otherSessionSig);

    const otherSession =
      await sessionManager.program.account.sessionToken.fetch(
        otherSessionToken
      );
    console.log("Other session", otherSession);
  });

  // delegate users
  it("Delegate Users", async () => {
    for (const { userAccount, kp } of [
      { userAccount: userPda, kp: userKp },
      { userAccount: otherUserPda, kp: otherUserKp },
    ]) {
      console.log("Delegating account", userAccount.toBase58());

      const sig = await program.methods
        .delegateUser(kp.publicKey)
        .accountsPartial({
          payer: kp.publicKey,
          userAccount,
          validator: erValidator,
        })
        .signers([kp])
        .rpc();
      console.log("Sig delegate user", sig);
      await provider.connection.confirmTransaction(sig);

      console.log("Delegated account", userAccount.toBase58());
    }
  });

  // delegate lp
  it("Delegate LP", async () => {
    console.log("Delegating account", lpPda.toBase58());

    const sig = await program.methods
      .delegateLp(tokenMintA, tokenMintB)
      .accountsPartial({
        payer: user,
        config: configPda,
        lpAccount: lpPda,
        validator: erValidator,
      })
      .rpc();
    console.log("Sig delegate lp", sig);
    await provider.connection.confirmTransaction(sig);
  });

  // private transfer
  // TODO: add session token
  it("Transfer", async () => {
    // Used to force fetching accounts from the base validator for localnet
    try {
      await ephemeralProvider.connection.requestAirdrop(userPda, 1000);
    } catch (error) {
      console.error(error);
      // fails to airdrop but loads the accounts into the er
      // console.log("Error airdropping deposit PDA", error);
    }
    try {
      await ephemeralProvider.connection.requestAirdrop(otherUserPda, 1000);
    } catch (error) {
      // fails to airdrop but loads the accounts into the er
      // console.log("Error airdropping other deposit PDA", error);
    }

    const userBefore = await userTeeProgram.account.user.fetch(
      userPda
    );
    const mintAPositionBefore = userBefore.positions.find(p => p.mint.toBase58() === tokenMintA.toBase58());
    if (!mintAPositionBefore) {
      throw new Error("Mint A position not found");
    }

    const otherUserBefore = await otherUserTeeProgram.account.user.fetch(
      otherUserPda
    );
    let otherMintAPositionBefore = otherUserBefore.positions.find(p => p.mint.toBase58() === tokenMintA.toBase58());
    if (otherMintAPositionBefore === undefined) {
      otherMintAPositionBefore = otherUserBefore.positions[otherUserBefore.positions.length - 1];
    }

    const sig = await ephemeralProgram.methods
      .transfer(new anchor.BN(100_000))
      .accountsStrict({
        payer: sessionKp.publicKey,
        user: user,
        userAccount: userPda,
        destinationUserAccount: otherUserPda,
        config: configPda,
        mint: tokenMintA,
        sessionToken: sessionToken,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([sessionKp])
      .rpc();
    console.log("Sig transfer", sig);
    await ephemeralProvider.connection.confirmTransaction(sig);

    const userAfter = await userTeeProgram.account.user.fetch(
      userPda
    );
    const mintAPositionAfter = userAfter.positions.find(p => p.mint.toBase58() === tokenMintA.toBase58());
    assert.equal(mintAPositionAfter.amount.toNumber(), mintAPositionBefore.amount.toNumber() - 100_000);

    const otherUserAfter = await otherUserTeeProgram.account.user.fetch(
      otherUserPda
    );
    const otherMintAPositionAfter = otherUserAfter.positions.find(p => p.mint.toBase58() === tokenMintA.toBase58());
    assert.equal(otherMintAPositionAfter.amount.toNumber(), otherMintAPositionBefore.amount.toNumber() + 100_000);
  });

  // add liquidity in ER
  // TODO: add session token
  it("Add Liquidity in ER", async () => {
    // Used to force fetching accounts from the base validator for localnet
    try {
      await ephemeralProvider.connection.requestAirdrop(lpPda, 1000);
    } catch (error) {
      // fails to airdrop but loads the accounts into the er
      // console.log("Error airdropping other deposit PDA", error);
    }

    const userBefore = await userTeeProgram.account.user.fetch(userPda);
    let mintAPositionBefore = userBefore.positions.find(p => p.mint.toBase58() === tokenMintA.toBase58());
    if (!mintAPositionBefore) {
      throw new Error("Mint A position not found");
    }
    let mintBPositionBefore = userBefore.positions.find(p => p.mint.toBase58() === tokenMintB.toBase58());
    if (!mintBPositionBefore) {
      throw new Error("Mint B position not found");
    }
    let lpMintPositionBefore = userBefore.positions.find(p => p.mint.toBase58() === lpMint.toBase58());
    if (lpMintPositionBefore === undefined) {
      lpMintPositionBefore = userBefore.positions[userBefore.positions.length - 1];
    }

    const lpBefore = await userTeeProgram.account.liquidityPool.fetch(lpPda);

    let sig = await ephemeralProgram.methods
      .addLiquidity(new anchor.BN(100_000), new anchor.BN(100_000), new anchor.BN(100_000))
      .accountsStrict({
        payer: sessionKp.publicKey,
        user: user,
        userAccount: userPda,
        config: configPda,
        mintA: tokenMintA,
        mintB: tokenMintB,
        lp: lpPda,
        mintLp: lpMint,
        sessionToken: sessionToken,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([sessionKp])
      .rpc();
    await ephemeralProvider.connection.confirmTransaction(sig);
    console.log("Sig add liquidity in ER", sig);

    const userAfter = await userTeeProgram.account.user.fetch(userPda);
    const mintAPositionAfter = userAfter.positions.find(p => p.mint.toBase58() === tokenMintA.toBase58());
    assert.equal(mintAPositionAfter.amount.toNumber(), mintAPositionBefore.amount.toNumber() - 100_000);
    const mintBPositionAfter = userAfter.positions.find(p => p.mint.toBase58() === tokenMintB.toBase58());
    assert.equal(mintBPositionAfter.amount.toNumber(), mintBPositionBefore.amount.toNumber() - 100_000);
    const lpMintPositionAfter = userAfter.positions.find(p => p.mint.toBase58() === lpMint.toBase58());
    assert.equal(lpMintPositionAfter.amount.toNumber(), lpMintPositionBefore.amount.toNumber() + 100_000);

    const lpAfter = await ephemeralProgram.account.liquidityPool.fetch(lpPda);
    assert.equal(lpAfter.virtualReserveA.toNumber(), lpBefore.virtualReserveA.toNumber() + 100_000);
    assert.equal(lpAfter.virtualReserveB.toNumber(), lpBefore.virtualReserveB.toNumber() + 100_000);
    assert.equal(lpAfter.lpSupply.toNumber(), lpBefore.lpSupply.toNumber() + 100_000);
  });

  // private swap
  // TODO: add session token
  it("Swap", async () => {
    const userBefore = await userTeeProgram.account.user.fetch(
      userPda
    );
    const mintAPositionBefore = userBefore.positions.find(p => p.mint.toBase58() === tokenMintA.toBase58());
    if (!mintAPositionBefore) {
      throw new Error("Mint A position not found");
    }
    let mintBPositionBefore = userBefore.positions.find(p => p.mint.toBase58() === tokenMintB.toBase58());
    if (mintBPositionBefore === undefined) {
      mintBPositionBefore = userBefore.positions[userBefore.positions.length - 1];
    }

    const sig = await ephemeralProgram.methods
      .swap(true, new anchor.BN(20_000), new anchor.BN(15_000))
      .accountsStrict({
        payer: sessionKp.publicKey,
        user: user,
        userAccount: userPda,
        lp: lpPda,
        config: configPda,
        mintA: tokenMintA,
        mintB: tokenMintB,
        sessionToken: sessionToken,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([sessionKp])
      .rpc();
    console.log("Sig swap", sig);
    await ephemeralProvider.connection.confirmTransaction(sig);

    const userAfter = await userTeeProgram.account.user.fetch(
      userPda
    );
    const mintAPositionAfter = userAfter.positions.find(p => p.mint.toBase58() === tokenMintA.toBase58());
    assert.equal(mintAPositionAfter.amount.toNumber(), mintAPositionBefore.amount.toNumber() - 20_000);

    const mintBPositionAfter = userAfter.positions.find(p => p.mint.toBase58() === tokenMintB.toBase58());
    assert.approximately(mintBPositionAfter.amount.toNumber(), mintBPositionBefore.amount.toNumber() + 20_000, 5_000);
  });

  it("Undelegate Users", async () => {
    for (const { userAccount, kp, session, sessionKey } of [
      {
        userAccount: userPda,
        kp: userKp,
        session: sessionToken,
        sessionKey: sessionKp,
      },
      {
        userAccount: otherUserPda,
        kp: otherUserKp,
        session: otherSessionToken,
        sessionKey: otherSessionKp,
      },
    ]) {
      console.log("Undelegating account", userAccount.toBase58());

      const sig = await ephemeralProgram.methods
        .commitAndUndelegateUser()
        .accountsPartial({
          payer: kp.publicKey,
          user: kp.publicKey,
          sessionToken: null,
          userAccount,
        })
        .signers([kp])
        .rpc();
      console.log("Sig undelegate", sig);
      await ephemeralProvider.connection.confirmTransaction(sig, "finalized");

      while (!(await provider.connection.getAccountInfo(userAccount))?.owner.equals(program.programId)) {
        console.log("Waiting for account to be undelegated...");
        await sleep(1000);
      }

      console.log("Account undelegated", userAccount.toBase58());
    }
  });

  it("Withdraw from user account", async () => {
    const userAccountInfo = await provider.connection.getAccountInfo(userPda);
    if (userAccountInfo === null) {
      throw new Error("User account not found");
    }
    if (userAccountInfo?.owner.equals(DELEGATE_PROGRAM_ID)) {
      throw new Error("User account is delegated");
    }

    const userBefore = await userTeeProgram.account.user.fetch(userPda);
    const mintAPositionBefore = userBefore.positions.find(p => p.mint.toBase58() === tokenMintA.toBase58());
    if (!mintAPositionBefore) {
      throw new Error("Mint A position not found");
    }

    const userTokenABalanceBefore = await provider.connection.getTokenAccountBalance(userTokenAAccount);

    // Wait for the undelegation to be complete
    await new Promise((resolve) => setTimeout(resolve, 3000));

    let sig = await program.methods
      .withdraw(new anchor.BN(120_000))
      .accountsStrict({
        sender: user,
        user: userPda,
        senderAta: userTokenAAccount,
        vault: vaultAAccount,
        config: configPda,
        mint: tokenMintA,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    await provider.connection.confirmTransaction(sig);
    console.log("Sig withdraw from user", sig);

    const userAfter = await program.account.user.fetch(userPda);
    const mintAPositionAfter = userAfter.positions.find(p => p.mint.toBase58() === tokenMintA.toBase58());
    assert.equal(mintAPositionAfter.amount.toNumber(), mintAPositionBefore.amount.toNumber() - 120_000);
    const userTokenABalanceAfter = await provider.connection.getTokenAccountBalance(userTokenAAccount);
    assert.equal(Number(userTokenABalanceAfter.value.amount), Number(userTokenABalanceBefore.value.amount) + 120_000);
  });

  it("Revoke session", async () => {
    const revokeSessionSig = await sessionManager.program.methods
      .revokeSession()
      .accountsPartial({
        sessionToken,
        authority: wallet.publicKey,
      })
      .rpc();
    await provider.connection.confirmTransaction(revokeSessionSig);
    console.log("Sig revoke session", revokeSessionSig);
  });
});

async function getPrivateRollupProvider(wallet: anchor.Wallet) {
  if (isLocalnet) {
    return new anchor.AnchorProvider(
      new anchor.web3.Connection("http://localhost:7799", {
        commitment: "confirmed",
      }),
      wallet
    )
  }

  const token = await getAuthToken(DEVNET_EPHEMERAL_TEE_URL, wallet.payer);
  return new anchor.AnchorProvider(
    new anchor.web3.Connection(`${DEVNET_EPHEMERAL_TEE_URL}?token=${token}`, {
    // new anchor.web3.Connection("https://devnet.magicblock.app", {
      commitment: "confirmed",
    }),
    wallet
  );
}
