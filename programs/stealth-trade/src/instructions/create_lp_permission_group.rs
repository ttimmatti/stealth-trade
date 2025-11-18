use anchor_lang::prelude::*;
use magicblock_permission_client::instructions::CreateGroupCpiBuilder;



#[derive(Accounts)]
#[instruction(game_id: u64)]
pub struct CreateLpPermissionGroup<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    /// CHECK: Checked by the permission program
    #[account(mut)]
    pub group: UncheckedAccount<'info>,
    /// CHECK: Checked by the permission program
    pub permission_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateLpPermissionGroup<'info> {
    /// Adds a user to a shared permission group for a liquidity pool account using the external permission program.
    ///
    /// Calls out to the permission program to add a user to a shared group and permission for the liquidity pool account.
    pub fn create_permission_group(&mut self, group_id: Pubkey, users: Vec<Pubkey>) -> Result<()> {
        let Self {
            sender,
            group,
            permission_program,
            system_program,
        } = self;

        // the update group cpi is not yet supported
        // so we need to create a new group for all users each time new user is added
        CreateGroupCpiBuilder::new(&permission_program)
            .group(&group)
            .id(group_id)
            .members(users)
            .payer(&sender)
            .system_program(system_program)
            .invoke()?;

        Ok(())
    }
}