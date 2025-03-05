use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{create, Create},
    token::{self, Burn, MintTo},
};
use crate::contexts::{CreateUserAta, StakeTopicSpecificTokens};
use crate::error::ErrorCode;
use crate::data::TopicToken;

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

// Removed legacy stake_alignment_tokens function

/// Stakes tempAlign tokens for a specific topic to earn topic-specific tempRep
pub fn stake_topic_specific_tokens(ctx: Context<StakeTopicSpecificTokens>, amount: u64) -> Result<()> {
    // Validate the stake amount
    if amount == 0 {
        return Err(ErrorCode::ZeroStakeAmount.into());
    }
    
    // Double-check user profile is properly initialized
    if ctx.accounts.user_profile.user != ctx.accounts.user.key() {
        return Err(ErrorCode::InvalidUserProfile.into());
    }
    
    // Get the topic ID
    let topic_id = ctx.accounts.topic.id;
    
    // Check if the user has enough topic-specific temp alignment tokens
    let user_profile = &mut ctx.accounts.user_profile;
    let mut found_topic = false;
    let mut topic_temp_align = 0;
    
    // Find the topic in the user's topic_tokens collection
    for (id, token_balance) in user_profile.topic_tokens.iter() {
        if *id == topic_id {
            found_topic = true;
            topic_temp_align = token_balance.temp_align_amount;
            break;
        }
    }
    
    // Ensure the user has enough topic-specific tokens
    if !found_topic || topic_temp_align < amount {
        return Err(ErrorCode::InsufficientTopicTokens.into());
    }
    
    // Check the global token balance
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
    
    // Update the topic-specific token balances in the user profile
    for (id, token_balance) in user_profile.topic_tokens.iter_mut() {
        if *id == topic_id {
            // Decrease tempAlign for this topic
            token_balance.temp_align_amount = token_balance.temp_align_amount
                .checked_sub(amount)
                .ok_or(ErrorCode::Overflow)?;
            
            // Increase tempRep for this topic
            token_balance.temp_rep_amount = token_balance.temp_rep_amount
                .checked_add(amount)
                .ok_or(ErrorCode::Overflow)?;
            
            break;
        }
    }
    
    // Legacy temp_rep_amount field has been removed
    
    msg!(
        "Staked {} topic-specific tempAlign tokens for topic {} for {} tempRep tokens for user {}",
        amount,
        topic_id,
        amount,
        ctx.accounts.user.key()
    );
    
    Ok(())
}