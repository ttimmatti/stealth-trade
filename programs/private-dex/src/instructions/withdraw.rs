use crate::errors::ErrorCode;
use crate::utils::get_position;
use crate::state::{Config, User};
use crate::constants::*;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

#[derive(Accounts)]
pub struct Withdraw<'info> {
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
        init_if_needed,
        payer = sender,
        associated_token::mint = mint,
        associated_token::authority = sender,
    )]
    pub sender_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = config,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub mint: InterfaceAccount<'info, Mint>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        require!(!self.config.paused, ErrorCode::Paused);

        self.decrease_position(amount)?;
        self.withdraw_tokens(amount)?;

        Ok(())
    }

    pub fn withdraw_tokens(&mut self, amount: u64) -> Result<()> {
        let signer_seeds: [&[&[u8]]; 1] = [&[CONFIG_SEED, &[self.config.bump]]];

        let transfer_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            TransferChecked {
                from: self.vault.to_account_info(),
                mint: self.mint.to_account_info(),
                to: self.sender_ata.to_account_info(),
                authority: self.config.to_account_info(),
            },
            &signer_seeds,
        );

        transfer_checked(transfer_ctx, amount, self.mint.decimals)?;

        Ok(())
    }

    pub fn decrease_position(&mut self, amount: u64) -> Result<()> {
        let position = get_position(&mut self.user.positions, self.mint.key())?;

        require_keys_eq!(position.mint, self.mint.key(), ErrorCode::PositionNotFound);
        require!(position.amount >= amount, ErrorCode::InsufficientBalance);
        position.amount -= amount;

        Ok(())
    }
}
