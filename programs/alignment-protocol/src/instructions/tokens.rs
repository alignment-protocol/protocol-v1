use crate::contexts::{
    CreateUserAta, CreateUserTempAlignAccount, CreateUserTempRepAccount, StakeTopicSpecificTokens,
};
use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{create, Create},
    token::{self, Burn, MintTo},
};

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

    msg!(
        "Created permanent token ATA for user {}",
        ctx.accounts.user.key()
    );
    Ok(())
}

/// Creates a protocol-owned tempAlign token account for a user
///
/// This creates a token account that:
/// 1. Is owned by the protocol (state PDA) rather than the user
/// 2. Has the state PDA as the authority, allowing burns without user signature
/// 3. Uses PDA with seeds ["user_temp_align", user.key()]
pub fn create_user_temp_align_account(ctx: Context<CreateUserTempAlignAccount>) -> Result<()> {
    // The token account is initialized in the context with proper ownership and authority

    msg!(
        "Created protocol-owned tempAlign token account for user {}",
        ctx.accounts.user.key()
    );

    Ok(())
}

/// Creates a protocol-owned tempRep token account for a user
///
/// This creates a token account that:
/// 1. Is owned by the protocol (state PDA) rather than the user
/// 2. Has the state PDA as the authority, allowing burns without user signature
/// 3. Uses PDA with seeds ["user_temp_rep", user.key()]
pub fn create_user_temp_rep_account(ctx: Context<CreateUserTempRepAccount>) -> Result<()> {
    // The token account is initialized in the context with proper ownership and authority

    msg!(
        "Created protocol-owned tempRep token account for user {}",
        ctx.accounts.user.key()
    );

    Ok(())
}

// Removed legacy stake_alignment_tokens function

/// Stakes tempAlign tokens for a specific topic to earn topic-specific tempRep
/// Burns from the protocol-owned tempAlign PDA and Mints to the protocol-owned tempRep PDA.
/// Updates the balances tracked in the UserTopicBalance account.
pub fn stake_topic_specific_tokens(
    ctx: Context<StakeTopicSpecificTokens>,
    amount: u64,
) -> Result<()> {
    // Validate the stake amount
    if amount == 0 {
        // Use existing error code from error.rs
        return Err(ErrorCode::ZeroStakeAmount.into());
    }

    let user_topic_balance = &mut ctx.accounts.user_topic_balance;
    let state = &ctx.accounts.state;
    let user = &ctx.accounts.user;
    let topic = &ctx.accounts.topic; // Needed for logging/events

    // 1. Check if the user has enough *allocated* tempAlign in UserTopicBalance for this topic
    // Although the global tempAlign PDA might have tokens, this checks the topic-specific assignment.
    // NOTE: This check assumes tempAlign is *consumed* during staking. If tempAlign should remain
    // in UserTopicBalance and only tempRep increases, this check needs adjustment.
    // Based on the variable names, consuming tempAlign to produce tempRep seems intended.
    if user_topic_balance.temp_align_amount < amount {
        // Use existing error code from error.rs
        return Err(ErrorCode::InsufficientTopicTokens.into());
    }

    // 2. Check if the global protocol-owned tempAlign account has enough tokens to burn
    // This is a sanity check; usually the UserTopicBalance check above should suffice if balances are synced.
    if ctx.accounts.user_temp_align_account.amount < amount {
        // Use existing error code from error.rs
        return Err(ErrorCode::InsufficientTokenBalance.into());
    }

    // Get the state PDA signer seeds for CPI calls (authority is the state PDA)
    let state_bump = state.bump;
    let state_seeds = &[b"state".as_ref(), &[state_bump]];
    let signer = &[&state_seeds[..]];

    // 3. Burn the temporary alignment tokens from the protocol-owned PDA
    let burn_cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Burn {
            mint: ctx.accounts.temp_align_mint.to_account_info(),
            from: ctx.accounts.user_temp_align_account.to_account_info(),
            authority: state.to_account_info(), // State PDA is the authority
        },
        signer, // Provide state PDA seeds as signer
    );
    token::burn(burn_cpi_ctx, amount)?;

    // 4. Mint temporary reputation tokens into the protocol-owned PDA
    let mint_cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
            mint: ctx.accounts.temp_rep_mint.to_account_info(),
            to: ctx.accounts.user_temp_rep_account.to_account_info(),
            authority: state.to_account_info(), // State PDA is the authority
        },
        signer, // Provide state PDA seeds as signer
    );
    token::mint_to(mint_cpi_ctx, amount)?;

    // 5. Update the topic-specific balances in the UserTopicBalance account
    // Decrease tempAlign for this topic
    user_topic_balance.temp_align_amount = user_topic_balance
        .temp_align_amount
        .checked_sub(amount)
        .ok_or(ErrorCode::Overflow)?;

    // Increase tempRep for this topic
    user_topic_balance.temp_rep_amount = user_topic_balance
        .temp_rep_amount
        .checked_add(amount)
        .ok_or(ErrorCode::Overflow)?;

    msg!(
        "User {} staked {} tempAlign for topic {}, received {} tempRep. New topic balances: Align={}, Rep={}",
        user.key(),
        amount,
        topic.key(), // Using topic key for clarity, could use topic.id
        amount,
        user_topic_balance.temp_align_amount,
        user_topic_balance.temp_rep_amount
    );

    // (Optional) Emit event
    // emit!(TokensStaked { ... });

    Ok(())
}
