use crate::errors::ErrorCode;
use crate::state::{
    Config, LiquidityPool, LiquidityPoolStatus,
};
use crate::constants::*;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        mint_to, Mint, MintTo, TokenAccount, TokenInterface,
    },
};

#[derive(Accounts)]
pub struct CreateLp<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    #[account(
        init,
        payer = sender,
        space = LiquidityPool::DISCRIMINATOR.len() + LiquidityPool::INIT_SPACE,
        seeds = [LIQUIDITY_POOL_SEED, mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump
    )]
    pub lp: Account<'info, LiquidityPool>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    pub mint_a: InterfaceAccount<'info, Mint>,
    pub mint_b: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = sender,
        seeds = [LP_MINT_SEED, lp.key().as_ref()],
        bump,
        mint::decimals = LP_DECIMALS,
        mint::authority = config,
    )]
    pub mint_lp: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = sender,
        associated_token::mint = mint_a,
        associated_token::authority = config,
    )]
    pub vault_a: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = sender,
        associated_token::mint = mint_b,
        associated_token::authority = config,
    )]
    pub vault_b: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init,
        payer = sender,
        associated_token::mint = mint_lp,
        associated_token::authority = config,
    )]
    pub vault_lp: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateLp<'info> {
    pub fn create_lp(&mut self, bumps: &CreateLpBumps) -> Result<()> {
        require!(!self.config.paused, ErrorCode::Paused);

        self.initialize(bumps.lp, bumps.mint_lp)?;

        self.mint_lp_supply()?;

        Ok(())
    }

    pub fn initialize(&mut self, bump: u8, mint_lp_bump: u8) -> Result<()> {
        self.lp.set_inner(LiquidityPool {
            bump,
            mint_lp_bump,
            status: LiquidityPoolStatus::Paused,
            authority: self.sender.key(),
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            vault_a: self.vault_a.key(),
            vault_b: self.vault_b.key(),
            virtual_reserve_a: 0,
            virtual_reserve_b: 0,
            lp_mint: self.mint_lp.key(),
            lp_supply: 0,
            pool_fee_bps: self.config.default_pool_fee_bps,
        });

        Ok(())
    }

    pub fn mint_lp_supply(&self) -> Result<()> {
        let signer_seeds: [&[&[u8]]; 1] = [&[CONFIG_SEED, &[self.config.bump]]];

        let mint_to_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            MintTo {
                mint: self.mint_lp.to_account_info(),
                to: self.vault_lp.to_account_info(),
                authority: self.config.to_account_info(),
            },
            &signer_seeds,
        );

        mint_to(mint_to_ctx, LP_SUPPLY)?;

        Ok(())
    }
}
