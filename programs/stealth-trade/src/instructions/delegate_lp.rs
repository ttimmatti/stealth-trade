use crate::errors::ErrorCode;
use crate::state::Config;
use crate::constants::*;
use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::{commit, delegate};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;
use session_keys::{Session, SessionToken};

#[delegate]
#[derive(Accounts)]
#[instruction(mint_a: Pubkey, mint_b: Pubkey)]
pub struct DelegateLp<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,  // allow unauthorized delegation for destination transfer
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,

    /// CHECK: MagicBlock ER validator
    #[account(address = config.er_validator @ ErrorCode::InvalidERValidator)]
    pub validator: UncheckedAccount<'info>,

    /// CHECK: LP account checked by the delegate program
    #[account(
        mut,
        del,
        seeds = [LIQUIDITY_POOL_SEED, mint_a.as_ref(), mint_b.as_ref()],
        bump,
    )]
    pub lp_account: UncheckedAccount<'info>,
}

impl<'info> DelegateLp<'info> {
    /// Delegates the liquidity pool account to the ephemeral rollups delegate program.
    ///
    /// Uses the ephemeral rollups delegate CPI to delegate the liquidity pool account.
    pub fn delegate(&mut self, mint_a: Pubkey, mint_b: Pubkey) -> Result<()> {
        let signer_seeds: &[&[u8]] = &[LIQUIDITY_POOL_SEED, mint_a.as_ref(), mint_b.as_ref()];

        self.delegate_lp_account(
            &self.payer,
            signer_seeds,
            DelegateConfig {
                validator: Some(self.validator.key()),
                ..DelegateConfig::default()
            },
        )?;
        
        Ok(())
    }
}

#[commit]
#[derive(Accounts, Session)]
#[instruction(mint_a: Pubkey, mint_b: Pubkey)]
pub struct UndelegateLp<'info> {
    #[account(
        mut,
        address = config.admin  // only admin can undelegate lp
    )]
    pub payer: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    #[session(
        signer = payer,
        authority = payer.key()
    )]
    pub session_token: Option<Account<'info, SessionToken>>,

    /// CHECK: LP account checked by the delegate program
    #[account(
        mut,
        seeds = [LIQUIDITY_POOL_SEED, mint_a.as_ref(), mint_b.as_ref()],
        bump
    )]
    pub lp_account: UncheckedAccount<'info>,
}

impl<'info> UndelegateLp<'info> {
    /// Commits and undelegates the deposit account from the ephemeral rollups program.
    ///
    /// Uses the ephemeral rollups SDK to commit and undelegate the deposit account.
    pub fn commit_and_undelegate(&mut self, _mint_a: Pubkey, _mint_b: Pubkey) -> Result<()> {
        commit_and_undelegate_accounts(
            &self.payer,
            vec![&self.lp_account.to_account_info()],
            &self.magic_context,
            &self.magic_program,
        )?;
        Ok(())
    }
}