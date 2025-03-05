use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{create, Create},
    token::{self, Burn, MintTo},
};
use crate::contexts::{CreateUserAta, StakeAlignmentTokens};
use crate::error::ErrorCode;

pub fn create_user_ata(ctx: Context<CreateUserAta>) -> Result<()> {
    // Build a CPI context for the associated token program
    let cpi_ctx = CpiContext::new(
        ctx.accounts.associated_token_program.to_account_info(),
        Create {
            payer: ctx.accounts.payer.to_account_info(),
            associated_token: ctx.accounts.user_ata.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        },
    );

    // If the ATA already exists, create(...) will throw an error
    create(cpi_ctx)?;

    msg!("Created ATA for user {}", ctx.accounts.user.key());
    Ok(())
}

pub fn stake_alignment_tokens(ctx: Context<StakeAlignmentTokens>, amount: u64) -> Result<()> {
    // Validate the stake amount
    if amount == 0 {
        return Err(ErrorCode::ZeroStakeAmount.into());
    }
    
    // Double-check user profile is properly initialized
    // (This is redundant with Anchor's deserialization, but adds an extra safety check)
    if ctx.accounts.user_profile.user != ctx.accounts.user.key() {
        return Err(ErrorCode::InvalidUserProfile.into());
    }
    
    // Check if the user has enough temp alignment tokens
    if ctx.accounts.user_temp_align_ata.amount < amount {
        return Err(ErrorCode::InsufficientTokenBalance.into());
    }
    
    // Burn the temporary alignment tokens
    let burn_cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Burn {
            mint: ctx.accounts.temp_align_mint.to_account_info(),
            from: ctx.accounts.user_temp_align_ata.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    
    token::burn(burn_cpi_ctx, amount)?;
    
    // Mint temporary reputation tokens
    let state_bump = ctx.accounts.state.bump;
    let seeds = &[b"state".as_ref(), &[state_bump]];
    let signer = &[&seeds[..]];
    
    let mint_cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
            mint: ctx.accounts.temp_rep_mint.to_account_info(),
            to: ctx.accounts.user_temp_rep_ata.to_account_info(),
            authority: ctx.accounts.state.to_account_info(),
        },
    )
    .with_signer(signer);
    
    token::mint_to(mint_cpi_ctx, amount)?;
    
    // Update the user profile with the new reputation amount
    let user_profile = &mut ctx.accounts.user_profile;
    user_profile.temp_rep_amount = user_profile.temp_rep_amount
        .checked_add(amount)
        .ok_or(ErrorCode::Overflow)?;
    
    msg!(
        "Staked {} tempAlign tokens for {} tempRep tokens for user {}",
        amount,
        amount,
        ctx.accounts.user.key()
    );
    
    Ok(())
}