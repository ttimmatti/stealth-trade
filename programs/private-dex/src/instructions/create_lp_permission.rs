use anchor_lang::prelude::*;
use magicblock_permission_client::instructions::CreatePermissionCpiBuilder;

use crate::constants::LIQUIDITY_POOL_SEED;


#[derive(Accounts)]
pub struct CreateLpPermission<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    /// CHECK: Checked by the permission program
    // #[account(seeds = [LIQUIDITY_POOL_SEED, lp.mint_a.key().as_ref(), lp.mint_b.key().as_ref()], bump = lp.bump)]
    pub lp: UncheckedAccount<'info>,
    /// CHECK: Checked by the permission program
    #[account(mut)]
    pub permission: UncheckedAccount<'info>,
    /// CHECK: Checked by the permission program
    #[account(mut)]
    pub group: UncheckedAccount<'info>,
    /// CHECK: Checked by the permission program
    pub permission_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateLpPermission<'info> {
    /// Adds a user to a shared permission group for a liquidity pool account using the external permission program.
    ///
    /// Calls out to the permission program to add a user to a shared group and permission for the liquidity pool account.
    pub fn create_permission(&mut self, mint_a: Pubkey, mint_b: Pubkey, bump: u8) -> Result<()> {
        let Self {
            sender,
            lp,
            permission,
            group,
            permission_program,
            system_program,
        } = self;

        let signer_seeds: &[&[&[u8]]; 1] = &[&[LIQUIDITY_POOL_SEED, mint_a.as_ref(), mint_b.as_ref(), &[bump]]];

        CreatePermissionCpiBuilder::new(&permission_program)
            .permission(&permission)
            .delegated_account(&lp)
            .group(&group)
            .payer(&sender)
            .system_program(system_program)
            .invoke_signed(signer_seeds)?;

        Ok(())
    }
}