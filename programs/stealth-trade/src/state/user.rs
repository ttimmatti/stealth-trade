use anchor_lang::prelude::*;
use crate::constants::MAX_POSITIONS;

#[account]
#[derive(InitSpace, Debug)]
pub struct User {
    pub bump: u8,
    pub authority: Pubkey,
    pub positions: [Position; MAX_POSITIONS],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, Debug)]
pub struct Position {
    pub mint: Pubkey,
    pub amount: u64,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            mint: Pubkey::default(),
            amount: 0,
        }
    }
}
