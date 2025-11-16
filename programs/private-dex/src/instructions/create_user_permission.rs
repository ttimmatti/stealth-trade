use anchor_lang::prelude::*;
use magicblock_permission_client::instructions::{CreateGroupCpiBuilder, CreatePermissionCpiBuilder};

use crate::{constants::USER_SEED, state::User};


#[derive(Accounts)]
pub struct CreateUserPermission<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: Anyone can create the permission for user account
    /// to allow transfers to new accounts. check seeds to only include user in the permission
    pub user: UncheckedAccount<'info>,
    #[account(
        seeds = [USER_SEED, user.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, User>,
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

impl<'info> CreateUserPermission<'info> {
    /// Creates a permission group and permission for a player choice account using the external permission program.
    ///
    /// Calls out to the permission program to create a group and permission for the deposit account.
    pub fn create_permission(&mut self, group_id: Pubkey) -> Result<()> {
        let Self {
            payer,
            user,
            user_account,
            permission,
            group,
            permission_program,
            system_program,
        } = self;

        // is it neccessaty to check permission program id?
        CreateGroupCpiBuilder::new(&permission_program)
            .group(&group)
            .id(group_id)
            .members(vec![user.key()])
            .payer(&payer)
            .system_program(system_program)
            .invoke()?;

        let binding = user.key();
        let signer_seeds: &[&[&[u8]]; 1] = &[&[USER_SEED, binding.as_ref(), &[user_account.bump]]];

        CreatePermissionCpiBuilder::new(&permission_program)
            .permission(&permission)
            .delegated_account(&user_account.to_account_info())
            .group(&group)
            .payer(&payer)
            .system_program(system_program)
            .invoke_signed(signer_seeds)?;

        Ok(())
    }
}