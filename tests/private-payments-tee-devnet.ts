import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PrivatePayments } from "../target/types/private_payments";
import {
  groupPdaFromId,
  PERMISSION_PROGRAM_ID,
  permissionPdaFromAccount,
} from "@magicblock-labs/ephemeral-rollups-sdk/privacy";
import { DEPOSIT_PDA_SEED, VAULT_PDA_SEED } from "../frontend/lib/constants";
import privatePaymentsIdl from "../target/idl/private_payments.json";
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

const DEVNET_EPHEMERAL_TEE_URL = "https://tee.magicblock.app/";

// localnet validator
const TEE_DEVNET_VALIDATOR = new PublicKey(
  "FnE6VJT5QNZdedZPnCoLsARgBwoE6DeJNjBs2H1gySXA"
);

// static keys for users for easier debuggin
const userSecretKey = readFileSync("tmp/devnet-user.json");
const otherUserSecretKey = readFileSync("tmp/devnet-otherUser.json");
const mintSecretKey = readFileSync("tmp/devnet-mint.json");
const groupSecretKey = readFileSync("tmp/devnet-group.json");
const otherGroupSecretKey = readFileSync("tmp/devnet-otherGroup.json");
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

describe("private-payments-tee-devnet", () => {
  const wallet = new anchor.Wallet(userKp);

  const provider = new anchor.AnchorProvider(
    new anchor.web3.Connection(DEVNET_RPC_URL, {
      commitment: "confirmed",
    }),
    wallet
  );
  let ephemeralProvider: anchor.AnchorProvider;
  anchor.setProvider(provider);

  const program = new Program<PrivatePayments>(privatePaymentsIdl, provider);
  let ephemeralProgram: Program<PrivatePayments>;

  const user = userKp.publicKey;
  const otherUser = otherUserKp.publicKey;
  let tokenMint: PublicKey,
    userTokenAccount: PublicKey,
    otherUserTokenAccount: PublicKey,
    vaultPda: PublicKey,
    vaultTokenAccount: PublicKey;
  const initialAmount = 1000000;
  const groupId = groupKp.publicKey;
  const otherGroupId = otherGroupKp.publicKey;
  let depositPda: PublicKey, otherDepositPda: PublicKey;
  let sessionKp: Keypair, sessionToken: PublicKey;
  let otherSessionKp: Keypair, otherSessionToken: PublicKey;

  const sessionManager = new SessionTokenManager(wallet, provider.connection);

  before(async () => {
    ephemeralProvider = await getPrivateRollupProvider(wallet);
    ephemeralProgram = new Program<PrivatePayments>(
      privatePaymentsIdl,
      ephemeralProvider
    );

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

    tokenMint = mintKp.publicKey;
    if ((await provider.connection.getAccountInfo(tokenMint)) === null) {
      console.log("Creating mint...");
      tokenMint = await createMint(
        provider.connection,
        userKp,
        user,
        null,
        6,
        mintKp,
        undefined,
        TOKEN_PROGRAM_ID
      );
    }

    while ((await provider.connection.getAccountInfo(tokenMint)) === null) {
      console.log("Waiting for mint to be created...");
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    depositPda = PublicKey.findProgramAddressSync(
      [Buffer.from(DEPOSIT_PDA_SEED), user.toBuffer(), tokenMint.toBuffer()],
      program.programId
    )[0];
    otherDepositPda = PublicKey.findProgramAddressSync(
      [
        Buffer.from(DEPOSIT_PDA_SEED),
        otherUser.toBuffer(),
        tokenMint.toBuffer(),
      ],
      program.programId
    )[0];
    vaultPda = PublicKey.findProgramAddressSync(
      [Buffer.from(VAULT_PDA_SEED), tokenMint.toBuffer()],
      program.programId
    )[0];
    vaultTokenAccount = getAssociatedTokenAddressSync(
      tokenMint,
      vaultPda,
      true,
      TOKEN_PROGRAM_ID
    );

    userTokenAccount = getAssociatedTokenAddressSync(
      tokenMint,
      user,
      false,
      TOKEN_PROGRAM_ID
    );
    if ((await provider.connection.getAccountInfo(userTokenAccount)) === null) {
      console.log("Creating user token account...");
      userTokenAccount = await createAssociatedTokenAccountIdempotent(
        provider.connection,
        userKp,
        tokenMint,
        user,
        undefined,
        TOKEN_PROGRAM_ID
      );
    }

    otherUserTokenAccount = getAssociatedTokenAddressSync(
      tokenMint,
      otherUser,
      false,
      TOKEN_PROGRAM_ID
    );
    if (
      (await provider.connection.getAccountInfo(otherUserTokenAccount)) === null
    ) {
      console.log("Creating other user token account...");
      otherUserTokenAccount = await createAssociatedTokenAccountIdempotent(
        provider.connection,
        otherUserKp,
        tokenMint,
        otherUser,
        undefined,
        TOKEN_PROGRAM_ID
      );
    }

    while (
      (await provider.connection.getAccountInfo(userTokenAccount)) === null
    ) {
      console.log("Waiting for user token account to be created...");
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    // Mint tokens to the user
    const userAta = await provider.connection.getTokenAccountBalance(
      userTokenAccount
    );
    if (userAta.value.uiAmount === 0) {
      console.log("Minting tokens to user...");
      await mintToChecked(
        provider.connection,
        userKp,
        tokenMint,
        userTokenAccount,
        user,
        new anchor.BN(initialAmount) as any,
        6,
        undefined,
        undefined,
        TOKEN_PROGRAM_ID
      );
    }

    console.log("User token account", userTokenAccount.toBase58());
    console.log("Vault token account", vaultTokenAccount.toBase58());
    console.log("Deposit PDA", depositPda.toBase58());
    console.log("Other deposit PDA", otherDepositPda.toBase58());
    console.log("User", user.toBase58());
    console.log("Other user", otherUser.toBase58());
    console.log("Token mint", tokenMint.toBase58());
    console.log("Group ID", groupId.toBase58());
    console.log("Other group ID", otherGroupId.toBase58());
  });

  it("Initialize deposits", async () => {
    let sig = await program.methods
      .initializeDeposit()
      .accountsStrict({
        payer: user,
        user,
        deposit: depositPda,
        tokenMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    await provider.connection.confirmTransaction(sig);
    console.log("Sig", sig);

    let deposit = await program.account.deposit.fetch(depositPda);
    assert.equal(deposit.amount.toNumber(), 0);

    sig = await program.methods
      .initializeDeposit()
      .accountsStrict({
        payer: otherUser,
        user: otherUser,
        deposit: otherDepositPda,
        tokenMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([otherUserKp])
      .rpc();
    await provider.connection.confirmTransaction(sig);
    console.log("Sig", sig);

    deposit = await program.account.deposit.fetch(otherDepositPda);
    assert.equal(deposit.amount.toNumber(), 0);
  });

  it("Modify balance", async () => {
    let sig = await program.methods
      .modifyBalance({
        amount: new anchor.BN(initialAmount / 2),
        increase: true,
      })
      .accountsStrict({
        user,
        payer: user,
        deposit: depositPda,
        userTokenAccount,
        vault: vaultPda,
        vaultTokenAccount,
        tokenMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    await provider.connection.confirmTransaction(sig);
    console.log("Sig", sig);

    let deposit = await program.account.deposit.fetch(depositPda);
    assert.equal(deposit.amount.toNumber(), initialAmount / 2);

    sig = await program.methods
      .modifyBalance({
        amount: new anchor.BN(initialAmount / 4),
        increase: false,
      })
      .accountsStrict({
        user,
        payer: user,
        deposit: depositPda,
        userTokenAccount,
        vault: vaultPda,
        vaultTokenAccount,
        tokenMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    await provider.connection.confirmTransaction(sig);
    console.log("Sig", sig);

    deposit = await program.account.deposit.fetch(depositPda);
    assert.equal(deposit.amount.toNumber(), initialAmount / 4);

    sig = await program.methods
      .modifyBalance({
        amount: new anchor.BN((3 * initialAmount) / 4),
        increase: true,
      })
      .accountsStrict({
        user,
        payer: user,
        deposit: depositPda,
        userTokenAccount,
        vault: vaultPda,
        vaultTokenAccount,
        tokenMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    await provider.connection.confirmTransaction(sig);
    console.log("Sig", sig);

    deposit = await program.account.deposit.fetch(depositPda);
    assert.equal(deposit.amount.toNumber(), initialAmount);
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

  it("Create permission", async () => {
    for (const { deposit, kp, id } of [
      { deposit: depositPda, kp: userKp, id: groupId },
      { deposit: otherDepositPda, kp: otherUserKp, id: otherGroupId },
    ]) {
      const permission = permissionPdaFromAccount(deposit);
      console.log("Permission", permission.toBase58());
      const group = groupPdaFromId(id);
      console.log("Group", group.toBase58());

      const sig = await program.methods
        .createPermission(id)
        .accountsStrict({
          payer: kp.publicKey,
          user: kp.publicKey,
          deposit,
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

  it("Delegate", async () => {
    for (const { deposit, kp } of [
      { deposit: depositPda, kp: userKp },
      { deposit: otherDepositPda, kp: otherUserKp },
    ]) {
      console.log("Delegating account", deposit.toBase58());

      const sig = await program.methods
        .delegate(kp.publicKey, tokenMint)
        .accountsPartial({
          payer: kp.publicKey,
          deposit,
          validator: TEE_DEVNET_VALIDATOR,
        })
        .signers([kp])
        .rpc();
      console.log("Sig", sig);
      await provider.connection.confirmTransaction(sig);

      console.log("Delegated account", deposit.toBase58());
    }
  });

  it("Transfer", async () => {
    // Used to force fetching accounts from the base validator
    try {
      await ephemeralProvider.connection.requestAirdrop(depositPda, 1000);
    } catch (error) {
      console.error(error);
      // fails to airdrop but loads the accounts into the er
      // console.log("Error airdropping deposit PDA", error);
    }
    try {
      await ephemeralProvider.connection.requestAirdrop(otherDepositPda, 1000);
    } catch (error) {
      // fails to airdrop but loads the accounts into the er
      // console.log("Error airdropping other deposit PDA", error);
    }

    const depositBefore = await ephemeralProgram.account.deposit.fetch(
      depositPda
    );
    console.log("Deposit before", depositBefore.amount.toNumber());

    const otherDepositBefore = await ephemeralProgram.account.deposit.fetch(
      otherDepositPda
    );
    console.log("Other deposit before", otherDepositBefore.amount.toNumber());

    const sig = await ephemeralProgram.methods
      .transferDeposit(new anchor.BN(initialAmount / 2))
      .accountsStrict({
        user,
        payer: sessionKp.publicKey,
        sourceDeposit: depositPda,
        destinationDeposit: otherDepositPda,
        tokenMint,
        systemProgram: SystemProgram.programId,
        sessionToken,
      })
      .signers([sessionKp])
      .rpc();
    console.log("Sig", sig);
    await ephemeralProvider.connection.confirmTransaction(sig);

    const userDepositAfter = await ephemeralProgram.account.deposit.fetch(
      depositPda
    );
    console.log("Deposit after", userDepositAfter.amount.toNumber());
    assert.equal(userDepositAfter.amount.toNumber(), initialAmount / 2);

    const otherDepositAfter = await ephemeralProgram.account.deposit.fetch(
      otherDepositPda
    );
    console.log("Other deposit after", otherDepositAfter.amount.toNumber());
    assert.equal(otherDepositAfter.amount.toNumber(), initialAmount / 2);

    console.log("Other deposit PDA transfered", otherDepositPda.toBase58());
  });

  it("Undelegate Deposits", async () => {
    for (const { deposit, kp, session, sessionKey } of [
      {
        deposit: depositPda,
        kp: userKp,
        session: sessionToken,
        sessionKey: sessionKp,
      },
      {
        deposit: otherDepositPda,
        kp: otherUserKp,
        session: otherSessionToken,
        sessionKey: otherSessionKp,
      },
    ]) {
      console.log("Undelegating account", deposit.toBase58());

      const sig = await ephemeralProgram.methods
        .undelegate()
        .accountsPartial({
          payer: sessionKey.publicKey,
          user: kp.publicKey,
          sessionToken: session,
          deposit,
        })
        .signers([sessionKey])
        .rpc();
      console.log("Sig undelegate", sig);
      await ephemeralProvider.connection.confirmTransaction(sig, "finalized");

      console.log("Undelegated account", deposit.toBase58());
    }
  });

  it("Withdraw from deposit", async () => {
    const depositBefore = await program.account.deposit.fetch(depositPda);
    console.log("Deposit before", depositBefore.amount.toNumber());

    // Wait for the undelegation to be complete
    await new Promise((resolve) => setTimeout(resolve, 3000));

    let sig = await program.methods
      .modifyBalance({
        amount: new anchor.BN(initialAmount / 2),
        increase: false,
      })
      .accountsStrict({
        user,
        payer: user,
        deposit: depositPda,
        userTokenAccount,
        vault: vaultPda,
        vaultTokenAccount,
        tokenMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    await provider.connection.confirmTransaction(sig);
    console.log("Sig", sig);

    const depositAfter = await program.account.deposit.fetch(depositPda);
    const userAfter = await getAccount(provider.connection, userTokenAccount);
    console.log("Deposit after", depositAfter.amount.toNumber());
    console.log("User after", userAfter.amount);
    assert.equal(depositAfter.amount.toNumber(), 0);
    assert.equal(Number(userAfter.amount), initialAmount / 2);

    const otherDepositBefore = await program.account.deposit.fetch(
      otherDepositPda
    );
    console.log("Other deposit before", otherDepositBefore.amount.toNumber());

    sig = await program.methods
      .modifyBalance({
        amount: new anchor.BN(initialAmount / 2),
        increase: false,
      })
      .accountsStrict({
        user: otherUser,
        payer: otherUser,
        deposit: otherDepositPda,
        userTokenAccount: otherUserTokenAccount,
        vault: vaultPda,
        vaultTokenAccount,
        tokenMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([otherUserKp])
      .rpc({ skipPreflight: true });
    console.log("Sig", sig);
    await provider.connection.confirmTransaction(sig);

    const otherDepositAfter = await program.account.deposit.fetch(
      otherDepositPda
    );
    const otherUserAfter = await getAccount(
      provider.connection,
      otherUserTokenAccount
    );
    console.log("Other deposit after", otherDepositAfter.amount.toNumber());
    console.log("Other user after", otherUserAfter.amount);
    assert.equal(otherDepositAfter.amount.toNumber(), 0);
    assert.equal(Number(otherUserAfter.amount), initialAmount / 2);
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
  const token = await getAuthToken(DEVNET_EPHEMERAL_TEE_URL, wallet.payer);
  return new anchor.AnchorProvider(
    new anchor.web3.Connection(`${DEVNET_EPHEMERAL_TEE_URL}?token=${token}`, {
    // new anchor.web3.Connection("https://devnet.magicblock.app", {
      commitment: "confirmed",
    }),
    wallet
  );
}
