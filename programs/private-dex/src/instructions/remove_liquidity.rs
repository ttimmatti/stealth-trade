use crate::errors::ErrorCode;
use crate::state::{
    Config, LiquidityPool, User
};
use crate::constants::*;
use crate::utils::{decrease_position, increase_position};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        Mint, TokenInterface,
    },
};
use constant_product_curve::{ConstantProduct, XYAmounts};
use session_keys::{Session, SessionToken};

#[derive(Accounts, Session)]
pub struct RemoveLiquidity<'info> {
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
        mut,
        seeds = [LIQUIDITY_POOL_SEED, mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump = lp.bump
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
        mut,
        seeds = [LP_MINT_SEED, lp.key().as_ref()],
        bump = lp.mint_lp_bump,
        mint::decimals = LP_DECIMALS,
        mint::authority = config,
    )]
    pub mint_lp: InterfaceAccount<'info, Mint>,

    #[session(
        signer = payer,
        authority = user.key()
    )]
    pub session_token: Option<Account<'info, SessionToken>>,

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

        increase_position(&mut self.user_account.positions, mint.key(), amount)?;

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
        decrease_position(&mut self.user_account.positions, self.mint_lp.key(), amount)?;

        require!(self.lp.lp_supply >= amount, ErrorCode::InsufficientPoolBalance);
        self.lp.lp_supply -= amount;

        Ok(())
    }
}
