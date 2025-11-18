use crate::errors::ErrorCode;
use crate::utils::{decrease_position, increase_position};
use crate::state::{Config, User};
use crate::constants::*;
use anchor_lang::prelude::*;
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

    /// CHECK: Token Mint to transfer
    pub mint: UncheckedAccount<'info>,

    #[session(
        signer = payer,
        authority = user.key()
    )]
    pub session_token: Option<Account<'info, SessionToken>>,

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
