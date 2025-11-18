import { PublicKey } from "@solana/web3.js";

// Program IDs
export const DELEGATE_PROGRAM_ID = new PublicKey("DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh");
export const DEVNET_VALIDATOR_ID = new PublicKey("MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57");
export const ER_VALIDATOR_ID = new PublicKey("mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev");
export const PERMISSION_PROGRAM_ID = new PublicKey("BTWAqWNBmF2TboMh3fxMJfgR16xGHYD7Kgr2dPwbRPBi");

// Program seeds
export const CONFIG_SEED = Buffer.from("config");
export const USER_SEED = Buffer.from("user");
export const LIQUIDITY_POOL_SEED = Buffer.from("liquidity_pool");
export const LP_MINT_SEED = Buffer.from("lp_mint");

// Token decimals
export const DECIMALS = 6;

/**
 * Derives the Group PDA for a given group_id
 */
export function getGroupPda(groupId: PublicKey, permissionProgramId: PublicKey): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("group:"), groupId.toBuffer()],
    permissionProgramId
  );
  return pda;
}

/**
 * Derives the Permission PDA for a given delegated account and group
 */
export function getPermissionPda(
  delegatedAccount: PublicKey,
  group: PublicKey,
  permissionProgramId: PublicKey
): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("permission:"), delegatedAccount.toBuffer()],
    permissionProgramId
  );
  return pda;
}

/**
 * Derives the User PDA
 */
export function getUserPda(user: PublicKey, programId: PublicKey): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [USER_SEED, user.toBuffer()],
    programId
  );
  return pda;
}

/**
 * Derives the Config PDA
 */
export function getConfigPda(programId: PublicKey): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [CONFIG_SEED],
    programId
  );
  return pda;
}

/**
 * Derives the Liquidity Pool PDA
 */
export function getLiquidityPoolPda(
  mintA: PublicKey,
  mintB: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [LIQUIDITY_POOL_SEED, mintA.toBuffer(), mintB.toBuffer()],
    programId
  );
}

/**
 * Derives the LP Mint PDA
 */
export function getLpMintPda(liquidityPool: PublicKey, programId: PublicKey): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [LP_MINT_SEED, liquidityPool.toBuffer()],
    programId
  );
  return pda;
}

