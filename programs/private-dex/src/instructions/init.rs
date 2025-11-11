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

    /// CHECK: MagicBlock delegate program
    pub delegate_program: UncheckedAccount<'info>,
    /// CHECK: MagicBlock ER validator
    pub er_validator: UncheckedAccount<'info>,
    /// CHECK: MagicBlock permission program
    pub permission_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        require!(self.sender.key() == ADMIN, ErrorCode::NotAdmin);

        self.config.set_inner(Config {
            bump: bumps.config,
            paused: false,
            admin: self.sender.key(),
            delegate_program: self.delegate_program.key(),
            er_validator: self.er_validator.key(),
            permission_program: self.permission_program.key(),
            default_pool_fee_bps: 100, // 1%
        });

        Ok(())
    }
}
