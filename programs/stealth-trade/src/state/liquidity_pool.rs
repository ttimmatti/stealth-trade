use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq, Eq, Debug)]
pub enum LiquidityPoolStatus {
    Active,
    Paused,
}

#[account]
#[derive(InitSpace, Debug)]
pub struct LiquidityPool {
    pub bump: u8,
    pub mint_lp_bump: u8,
    pub status: LiquidityPoolStatus,
    pub authority: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
    pub virtual_reserve_a: u64,
    pub virtual_reserve_b: u64,
    pub lp_mint: Pubkey,
    pub lp_supply: u64,
    /// Fees are collected in virtual reserves,
    /// generating yield for lp providers.
    pub pool_fee_bps: u16,
}
