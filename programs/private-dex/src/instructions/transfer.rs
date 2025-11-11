use crate::errors::ErrorCode;
use crate::utils::{decrease_position, increase_position};
use crate::state::{Config, User};
use crate::constants::*;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenInterface},
};

#[derive(Accounts)]
pub struct Transfer<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    #[account(
        mut,
        seeds = [USER_SEED, sender.key().as_ref()],
        bump = user.bump
    )]
    pub user: Account<'info, User>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [
            USER_SEED, 
            destination_user.authority.as_ref()
        ],
        bump = destination_user.bump
    )]
    pub destination_user: Account<'info, User>,

    pub mint: InterfaceAccount<'info, Mint>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Transfer<'info> {
    pub fn transfer(&mut self, amount: u64) -> Result<()> {
        require!(!self.config.paused, ErrorCode::Paused);

        decrease_position(&mut self.user.positions, self.mint.key(), amount)?;
        
        increase_position(&mut self.destination_user.positions, self.mint.key(), amount)?;

        Ok(())
    }
}
