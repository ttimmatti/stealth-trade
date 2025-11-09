use crate::errors::ErrorCode;
use crate::state::Position;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::spl_pod::option::Nullable;

pub fn get_position_index(positions: &[Position], mint: Pubkey) -> Result<usize> {
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

pub fn get_position(positions: &mut [Position], mint: Pubkey) -> Result<&mut Position> {
    let position_index = get_position_index(positions, mint)?;
    Ok(&mut positions[position_index])
}
