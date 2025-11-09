use crate::errors::ErrorCode;
use crate::state::Config;
use crate::constants::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    #[account(
        init,
        payer = sender,
        space = Config::DISCRIMINATOR.len() + Config::INIT_SPACE,
        seeds = [CONFIG_SEED],
        bump
    )]
    pub config: Account<'info, Config>,

    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        require!(self.sender.key() == ADMIN, ErrorCode::NotAdmin);

        self.config.set_inner(Config {
            bump: bumps.config,
            admin: self.sender.key(),
            paused: false,
            default_pool_fee_bps: 100, // 1%
        });

        Ok(())
    }
}
