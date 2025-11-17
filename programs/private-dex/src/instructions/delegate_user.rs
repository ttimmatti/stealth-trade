use crate::errors::ErrorCode;
use crate::state::{Config, User};
use crate::constants::*;
use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::{commit, delegate};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;
use session_keys::{SessionToken, Session};

#[delegate]
#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct DelegateUser<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,  // allow unauthorized delegation for destination transfer
    
    /// CHECK: Checked by the delegate program
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
    
    /// CHECK: MagicBlock ER validator
    #[account(address = config.er_validator @ ErrorCode::InvalidERValidator)]
    pub validator: UncheckedAccount<'info>,

    /// CHECK: User account checked by the delegate program
    #[account(
        mut,
        del,
        seeds = [USER_SEED, user.as_ref()],
        bump,
    )]
    pub user_account: UncheckedAccount<'info>,
}

impl<'info> DelegateUser<'info> {
    /// Delegates the deposit account to the ephemeral rollups delegate program.
    ///
    /// Uses the ephemeral rollups delegate CPI to delegate the deposit account.
    pub fn delegate(&mut self, user: Pubkey) -> Result<()> {
        let signer_seeds: &[&[u8]] = &[USER_SEED, user.as_ref()];

        self.delegate_user_account(
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
pub struct UndelegateUser<'info> {
    /// CHECK: Matched against the user account
    /// for non session user is signer to prevent unauthorized account reveal
    /// later can add admin to undelegate for accounts migration
    pub user: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,

    #[session(
        signer = payer,
        authority = user.key()
    )]
    pub session_token: Option<Account<'info, SessionToken>>,

    /// CHECK: User account checked by the delegate program
    #[account(
        mut,
        seeds = [USER_SEED, user.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, User>,
}

impl<'info> UndelegateUser<'info> {
    /// Commits and undelegates the deposit account from the ephemeral rollups program.
    ///
    /// Uses the ephemeral rollups SDK to commit and undelegate the deposit account.
    pub fn commit_and_undelegate(&mut self) -> Result<()> {
        commit_and_undelegate_accounts(
            &self.payer,
            vec![&self.user_account.to_account_info()],
            &self.magic_context,
            &self.magic_program,
        )?;
        Ok(())
    }
}