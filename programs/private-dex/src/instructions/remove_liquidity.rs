use crate::errors::ErrorCode;
use crate::state::{
    Config, LiquidityPool, User
};
use crate::constants::*;
use crate::utils::get_position;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        Mint, TokenInterface,
    },
};
use constant_product_curve::{ConstantProduct, XYAmounts};

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    #[account(
        mut,
        seeds = [USER_SEED, sender.key().as_ref()],
        bump = user.bump
    )]
    pub user: Account<'info, User>,

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

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> RemoveLiquidity<'info> {
    pub fn remove_liquidity(&mut self, amount: u64, max_x: u64, max_y: u64) -> Result<()> {
        require!(!self.config.paused, ErrorCode::Paused);

        let (x, y) = self.remove_liquidity_amounts_xy(amount, max_x, max_y)?;

        // virtual transfers
        self.withdraw_tokens(true, x)?;
        self.withdraw_tokens(false, y)?;

        self.burn_lp_tokens(amount)?;

        Ok(())
    }

    pub fn remove_liquidity_amounts_xy(&self, amount: u64, min_x: u64, min_y: u64) -> Result<(u64, u64)> {
        require!(self.lp.lp_supply != 0, ErrorCode::NoLiquidityInPool);
        
        let amounts: XYAmounts = ConstantProduct::xy_withdraw_amounts_from_l(
            self.lp.virtual_reserve_a,
            self.lp.virtual_reserve_b,
            self.lp.lp_supply,
            amount,
            10_u32.pow(PRECISION_DECIMALS)
        )
        .unwrap();

        let (x, y) = (amounts.x, amounts.y);

        require!(x >= min_x && y >= min_y, ErrorCode::SlippageExceeded);

        Ok((x, y))
    }

    /// Virtual transfers of tokens from pool to user
    pub fn withdraw_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let mint = match is_x {
            true => self.mint_a.to_account_info(),
            false => self.mint_b.to_account_info(),
        };

        let user_position = get_position(&mut self.user.positions, mint.key())?;

        user_position.mint = mint.key();
        user_position.amount += amount;

        let virtual_reserve = match is_x {
            true => &mut self.lp.virtual_reserve_a,
            false => &mut self.lp.virtual_reserve_b,
        };
        require!(*virtual_reserve >= amount, ErrorCode::InsufficientPoolBalance);
        *virtual_reserve -= amount;

        Ok(())
    }

    /// Virtual burning of LP tokens from user
    pub fn burn_lp_tokens(&mut self, amount: u64) -> Result<()> {
        let user_position = get_position(&mut self.user.positions, self.mint_lp.key())?;

        require_keys_eq!(user_position.mint, self.mint_lp.key(), ErrorCode::PositionNotFound);
        require!(user_position.amount >= amount, ErrorCode::InsufficientBalance);

        user_position.mint = self.mint_lp.key();
        user_position.amount -= amount;

        Ok(())
    }
}
