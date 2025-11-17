use crate::errors::ErrorCode;
use crate::state::{Config, LiquidityPool, LiquidityPoolStatus, User};
use crate::constants::*;
use crate::utils::{decrease_position, increase_position};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        Mint, TokenInterface,
    },
};
use constant_product_curve::{ConstantProduct, LiquidityPair};
use session_keys::{Session, SessionToken};

#[derive(Accounts, Session)]
pub struct Swap<'info> {
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

    #[session(
        signer = payer,
        authority = user.key()
    )]
    pub session_token: Option<Account<'info, SessionToken>>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Swap<'info> {
    pub fn swap(&mut self, is_x: bool, amount: u64, min: u64) -> Result<()> {
        require!(!self.config.paused, ErrorCode::Paused);

        require!(self.lp.status == LiquidityPoolStatus::Active, ErrorCode::PoolNotActive);

        require!(self.lp.lp_supply != 0, ErrorCode::NoLiquidityInPool);

        let (deposit, withdraw) = self.swap_amounts_xy(is_x, amount, min)?;

        // virtual transfers
        self.deposit_tokens(is_x, deposit)?;
        self.withdraw_tokens(!is_x, withdraw)?;

        Ok(())
    }

    pub fn swap_amounts_xy(&self, is_x: bool, amount: u64, min: u64) -> Result<(u64, u64)> {
        let mut c = ConstantProduct::init(
            self.lp.virtual_reserve_a,
            self.lp.virtual_reserve_b,
            self.lp.lp_supply,
            self.lp.pool_fee_bps,
            Some(LP_DECIMALS),
        ).unwrap();

        let lp = match is_x {
            true => LiquidityPair::X,
            false => LiquidityPair::Y,
        };

        let res = c.swap(lp, amount, min).unwrap();
        
        Ok((res.deposit, res.withdraw))
    }

    /// Virtual transfers of tokens from user to pool
    pub fn deposit_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let mint = match is_x {
            true => self.mint_a.to_account_info(),
            false => self.mint_b.to_account_info(),
        };

        decrease_position(&mut self.user_account.positions, mint.key(), amount)?;

        let virtual_reserve = match is_x {
            true => &mut self.lp.virtual_reserve_a,
            false => &mut self.lp.virtual_reserve_b,
        };
        *virtual_reserve += amount;

        Ok(())
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
}
