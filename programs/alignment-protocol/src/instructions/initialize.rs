use anchor_lang::prelude::*;
use crate::contexts::Initialize;
use crate::contexts::UpdateTokensToMint;

pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    let state_acc = &mut ctx.accounts.state;
    
    // Store all four token mint addresses
    state_acc.temp_align_mint = ctx.accounts.temp_align_mint.key();
    state_acc.align_mint = ctx.accounts.align_mint.key();
    state_acc.temp_rep_mint = ctx.accounts.temp_rep_mint.key();
    state_acc.rep_mint = ctx.accounts.rep_mint.key();
    
    // Set other state properties
    state_acc.authority = ctx.accounts.authority.key();
    state_acc.bump = ctx.bumps.state;
    state_acc.submission_count = 0;
    state_acc.topic_count = 0;
    state_acc.tokens_to_mint = 0;
    
    // Set default voting phase durations (24 hours each by default)
    state_acc.default_commit_phase_duration = 24 * 60 * 60; // 24 hours in seconds
    state_acc.default_reveal_phase_duration = 24 * 60 * 60; // 24 hours in seconds
    
    msg!("Initialized protocol with four token mints:");
    msg!("temp_align_mint = {}", state_acc.temp_align_mint);
    msg!("align_mint = {}", state_acc.align_mint);
    msg!("temp_rep_mint = {}", state_acc.temp_rep_mint);
    msg!("rep_mint = {}", state_acc.rep_mint);
    msg!("Default commit phase duration: {} seconds", state_acc.default_commit_phase_duration);
    msg!("Default reveal phase duration: {} seconds", state_acc.default_reveal_phase_duration);
    
    Ok(())
}

pub fn update_tokens_to_mint(
    ctx: Context<UpdateTokensToMint>,
    new_tokens_to_mint: u64,
) -> Result<()> {
    let state_acc = &mut ctx.accounts.state;
    let previous_tokens_to_mint = state_acc.tokens_to_mint;
    state_acc.tokens_to_mint = new_tokens_to_mint;
    msg!(
        "Updated tokens_to_mint from {} to {}",
        previous_tokens_to_mint,
        new_tokens_to_mint
    );
    Ok(())
}