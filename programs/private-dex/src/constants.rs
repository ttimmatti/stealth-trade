use anchor_lang::prelude::*;

pub const MAX_POSITIONS: usize = 10;

pub const ADMIN: Pubkey = Pubkey::from_str_const("3Qu3rYLyv2BCjkpBGufgnyjtHAvEzgKVR5AHMpgAaGqS");

pub const USER_SEED: &[u8] = b"user";
pub const LIQUIDITY_POOL_SEED: &[u8] = b"liquidity_pool";
pub const CONFIG_SEED: &[u8] = b"config";
pub const LP_MINT_SEED: &[u8] = b"lp_mint";

pub const LP_DECIMALS: u8 = 6;
pub const LP_SUPPLY: u64 = 1_000_000_000_000_000; // 1 billion

pub const PRECISION_DECIMALS: u32 = 9;
