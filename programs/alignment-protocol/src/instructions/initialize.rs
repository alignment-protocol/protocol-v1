use crate::contexts::{
    InitializeAlignMint, InitializeRepMint, InitializeState, InitializeTempAlignMint,
    InitializeTempRepMint, UpdateTokensToMint,
};
use anchor_lang::prelude::*;

pub fn initialize_state(ctx: Context<InitializeState>, oracle_pubkey: Pubkey) -> Result<()> {
    let state_acc = &mut ctx.accounts.state;

    // Set state properties
    state_acc.authority = ctx.accounts.authority.key();
    state_acc.oracle_pubkey = oracle_pubkey;
    state_acc.bump = ctx.bumps.state;
    state_acc.topic_count = 0;
    state_acc.tokens_to_mint = 0;

    // Set default voting phase durations (24 hours each by default)
    state_acc.default_commit_phase_duration = 24 * 60 * 60; // 24 hours in seconds
    state_acc.default_reveal_phase_duration = 24 * 60 * 60; // 24 hours in seconds

    // Initialize mint fields to default (they will be set in separate instructions)
    state_acc.temp_align_mint = Pubkey::default();
    state_acc.align_mint = Pubkey::default();
    state_acc.temp_rep_mint = Pubkey::default();
    state_acc.rep_mint = Pubkey::default();

    msg!("Initialized protocol state account");
    msg!(
        "Default commit phase duration: {} seconds",
        state_acc.default_commit_phase_duration
    );
    msg!(
        "Default reveal phase duration: {} seconds",
        state_acc.default_reveal_phase_duration
    );
    msg!("Authority: {}", state_acc.authority);
    msg!("Oracle Pubkey: {}", state_acc.oracle_pubkey);

    Ok(())
}

pub fn initialize_temp_align_mint(ctx: Context<InitializeTempAlignMint>) -> Result<()> {
    let state_acc = &mut ctx.accounts.state;

    // Store the temp_align_mint address
    state_acc.temp_align_mint = ctx.accounts.temp_align_mint.key();

    msg!(
        "Initialized temp_align_mint = {}",
        state_acc.temp_align_mint
    );

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
