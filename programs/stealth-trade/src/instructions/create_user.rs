use crate::errors::ErrorCode;
use crate::state::{Config, Position, User};
use crate::constants::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CreateUser<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    #[account(
        init,
        payer = sender,
        space = User::DISCRIMINATOR.len() + User::INIT_SPACE,
        seeds = [USER_SEED, sender.key().as_ref()],
        bump
    )]
    pub user: Account<'info, User>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    pub system_program: Program<'info, System>,
}

impl<'info> CreateUser<'info> {
    pub fn create_user(&mut self, bumps: &CreateUserBumps) -> Result<()> {
        require!(!self.config.paused, ErrorCode::Paused);

        self.user.set_inner(User {
            bump: bumps.user,
            authority: self.sender.key(),
            positions: [Position::default(); MAX_POSITIONS],
        });

        Ok(())
    }
}
