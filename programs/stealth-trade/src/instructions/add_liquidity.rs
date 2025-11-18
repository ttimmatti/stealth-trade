use crate::errors::ErrorCode;
use crate::state::{Config, LiquidityPool, LiquidityPoolStatus, User};
use crate::constants::*;
use crate::utils::{decrease_position, increase_position};
use anchor_lang::prelude::*;
use constant_product_curve::{ConstantProduct, XYAmounts};
use session_keys::{Session, SessionToken};

#[derive(Accounts, Session)]
pub struct AddLiquidity<'info> {
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

    /// CHECK: Token Mint to add liquidity. Matched against the liquidity pool mint a
    pub mint_a: UncheckedAccount<'info>,
    /// CHECK: Token Mint to add liquidity. Matched against the liquidity pool mint b
    pub mint_b: UncheckedAccount<'info>,
    /// CHECK: Liquidity Pool LP Mint. Matched against the liquidity pool lp mint
    #[account(
        seeds = [LP_MINT_SEED, lp.key().as_ref()],
        bump = lp.mint_lp_bump,
    )]
    pub mint_lp: UncheckedAccount<'info>,

    #[session(
        signer = payer,
        authority = user.key()
    )]
    pub session_token: Option<Account<'info, SessionToken>>,

    pub system_program: Program<'info, System>,
}

impl<'info> AddLiquidity<'info> {
    pub fn add_liquidity(&mut self, amount: u64, max_x: u64, max_y: u64) -> Result<()> {
        require!(!self.config.paused, ErrorCode::Paused);

        if self.lp.lp_supply == 0 && self.lp.virtual_reserve_a == 0 && self.lp.virtual_reserve_b == 0 {
            self.lp.status = LiquidityPoolStatus::Active;
        }

        let (x, y) = self.add_liquidity_amounts_xy(amount, max_x, max_y)?;

        // virtual transfers
        self.deposit_tokens(true, x)?;
        self.deposit_tokens(false, y)?;

        self.mint_lp_tokens(amount)?;

        Ok(())
    }

    pub fn add_liquidity_amounts_xy(&self, amount: u64, max_x: u64, max_y: u64) -> Result<(u64, u64)> {
        let (x, y) = match self.lp.lp_supply == 0
            && self.lp.virtual_reserve_a == 0
            && self.lp.virtual_reserve_b == 0
        {
            true => (max_x, max_y),
            false => {
                let amounts: XYAmounts = ConstantProduct::xy_deposit_amounts_from_l(
                    self.lp.virtual_reserve_a,
                    self.lp.virtual_reserve_b,
                    self.lp.lp_supply,
                    amount,
                    10_u32.pow(PRECISION_DECIMALS)
                )
                .unwrap();
                (amounts.x, amounts.y)
            }
        };

        require!(x <= max_x && y <= max_y, ErrorCode::SlippageExceeded);

        Ok((x, y))
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

    /// Virtual minting of LP tokens to user
    pub fn mint_lp_tokens(&mut self, amount: u64) -> Result<()> {
        increase_position(&mut self.user_account.positions, self.mint_lp.key(), amount)?;

        self.lp.lp_supply += amount;
        require!(self.lp.lp_supply < LP_SUPPLY, ErrorCode::LiquidityPoolOverflow);

        Ok(())
    }
}
