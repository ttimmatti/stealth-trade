use crate::errors::ErrorCode;
use crate::state::Position;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::spl_pod::option::Nullable;

/// Returns the index of the position if it exists, otherwise returns the next available index
/// - Returns an error if the maximum number of positions is reached
/// - Convert, withdraw, burn or transfer position to new subaccount to free up positions
fn get_position_index(positions: &[Position], mint: Pubkey) -> Result<usize> {
    for (i, position) in positions.iter().enumerate() {
        if position.mint.eq(&mint) {
            // Return the index of the position if exists
            return Ok(i);
        }
    }

    // Return the next available index if the position does not exist
    positions
        .iter()
        .position(|p| p.mint.is_none())
        .ok_or(error!(ErrorCode::MaxPositionsReached))
}

/// Returns the position if it exists, otherwise returns the next available position
/// - Returns an error if the maximum number of positions is reached
/// - Convert, withdraw, burn or transfer position to new subaccount to free up positions
fn get_position(positions: &mut [Position], mint: Pubkey) -> Result<&mut Position> {
    let position_index = get_position_index(positions, mint)?;
    Ok(&mut positions[position_index])
}

/// Decreases the amount of a position by the given amount
/// - Checks if the position exists and has enough balance
/// - If the amount is 0, the position is set to default
pub fn decrease_position(positions: &mut [Position], mint: Pubkey, amount: u64) -> Result<()> {
    let position = get_position(positions, mint)?;
    require_keys_eq!(position.mint, mint, ErrorCode::PositionNotFound);
    require!(position.amount >= amount, ErrorCode::InsufficientBalance);
    position.amount -= amount;

    if position.amount == 0 {
        position.mint = Pubkey::default();
    }

    Ok(())
}

/// Increases the amount of a position by the given amount
/// - If the position does not exist, it is created
pub fn increase_position(positions: &mut [Position], mint: Pubkey, amount: u64) -> Result<()> {
    let position = get_position(positions, mint)?;
    if position.mint.is_none() {
        position.mint = mint;
    }
    position.amount += amount;
    Ok(())
}
