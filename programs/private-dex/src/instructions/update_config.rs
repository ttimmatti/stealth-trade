use crate::errors::ErrorCode;
use crate::state::Config;
use crate::constants::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    /// CHECK: MagicBlock ER validator
    pub er_validator: Option<UncheckedAccount<'info>>,

    pub system_program: Program<'info, System>,
}

impl<'info> UpdateConfig<'info> {
    pub fn update_config(
        &mut self,
        is_paused: Option<bool>,
        default_pool_fee_bps: Option<u16>,
    ) -> Result<()> {
        require!(self.sender.key() == self.config.admin, ErrorCode::NotAdmin);

        if let Some(is_paused) = is_paused {
            self.config.paused = is_paused;
        }

        if let Some(er_validator) = &self.er_validator {
            self.config.er_validator = er_validator.to_account_info().key();
        }

        if let Some(default_pool_fee_bps) = default_pool_fee_bps {
            self.config.default_pool_fee_bps = default_pool_fee_bps;
        }

        Ok(())
    }
}
