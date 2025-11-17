use crate::errors::ErrorCode;
use crate::utils::{decrease_position, increase_position};
use crate::state::{Config, User};
use crate::constants::*;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenInterface},
};
use session_keys::{Session, SessionToken};

#[derive(Accounts, Session)]
pub struct Transfer<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Matched against the user account
    pub user: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [USER_SEED, user.key().as_ref()],
        bump = user_account.bump
    )]
    pub user_account: Account<'info, User>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [
            USER_SEED, 
            destination_user_account.authority.as_ref()
        ],
        bump = destination_user_account.bump
    )]
    pub destination_user_account: Account<'info, User>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[session(
        signer = payer,
        authority = user.key()
    )]
    pub session_token: Option<Account<'info, SessionToken>>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Transfer<'info> {
    pub fn transfer(&mut self, amount: u64) -> Result<()> {
        require!(!self.config.paused, ErrorCode::Paused);

        decrease_position(&mut self.user_account.positions, self.mint.key(), amount)?;
        
        increase_position(&mut self.destination_user_account.positions, self.mint.key(), amount)?;

        Ok(())
    }
}
