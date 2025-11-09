use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub bump: u8,
    pub paused: bool,
    pub admin: Pubkey,
    pub default_pool_fee_bps: u16,
}
