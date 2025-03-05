use anchor_lang::prelude::*;
use crate::contexts::{
    InitializeState, 
    InitializeTempAlignMint, 
    InitializeAlignMint, 
    InitializeTempRepMint, 
    InitializeRepMint,
    UpdateTokensToMint
};

pub fn initialize_state(ctx: Context<InitializeState>) -> Result<()> {
    let state_acc = &mut ctx.accounts.state;
    
    // Set state properties
    state_acc.authority = ctx.accounts.authority.key();
    state_acc.bump = ctx.bumps.state;
    state_acc.submission_count = 0;
    state_acc.topic_count = 0;
    state_acc.tokens_to_mint = 0;
    
    // Set default voting phase durations (24 hours each by default)
    state_acc.default_commit_phase_duration = 24 * 60 * 60; // 24 hours in seconds
    state_acc.default_reveal_phase_duration = 24 * 60 * 60; // 24 hours in seconds
    
    msg!("Initialized protocol state account");
    msg!("Default commit phase duration: {} seconds", state_acc.default_commit_phase_duration);
    msg!("Default reveal phase duration: {} seconds", state_acc.default_reveal_phase_duration);
    
    Ok(())
}

pub fn initialize_temp_align_mint(ctx: Context<InitializeTempAlignMint>) -> Result<()> {
    let state_acc = &mut ctx.accounts.state;
    
    // Store the temp_align_mint address
    state_acc.temp_align_mint = ctx.accounts.temp_align_mint.key();
    
    msg!("Initialized temp_align_mint = {}", state_acc.temp_align_mint);
    
    Ok(())
}

pub fn initialize_align_mint(ctx: Context<InitializeAlignMint>) -> Result<()> {
    let state_acc = &mut ctx.accounts.state;
    
    // Store the align_mint address
    state_acc.align_mint = ctx.accounts.align_mint.key();
    
    msg!("Initialized align_mint = {}", state_acc.align_mint);
    
    Ok(())
}

pub fn initialize_temp_rep_mint(ctx: Context<InitializeTempRepMint>) -> Result<()> {
    let state_acc = &mut ctx.accounts.state;
    
    // Store the temp_rep_mint address
    state_acc.temp_rep_mint = ctx.accounts.temp_rep_mint.key();
    
    msg!("Initialized temp_rep_mint = {}", state_acc.temp_rep_mint);
    
    Ok(())
}

pub fn initialize_rep_mint(ctx: Context<InitializeRepMint>) -> Result<()> {
    let state_acc = &mut ctx.accounts.state;
    
    // Store the rep_mint address
    state_acc.rep_mint = ctx.accounts.rep_mint.key();
    
    msg!("Initialized rep_mint = {}", state_acc.rep_mint);
    
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