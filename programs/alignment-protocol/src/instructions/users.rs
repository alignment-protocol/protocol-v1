use crate::contexts::{CreateUserProfile, InitializeUserTopicBalance};
use anchor_lang::prelude::*;

// Re-export other user-related instructions if moved here
// pub use crate::instructions::topics::create_user_profile; // Example
pub fn create_user_profile(ctx: Context<CreateUserProfile>) -> Result<()> {
    // Initialize the user profile fields
    let user_profile = &mut ctx.accounts.user_profile;
    user_profile.user = ctx.accounts.user.key();
    user_profile.user_submission_count = 0;
    user_profile.user_temp_align_account = Pubkey::default();
    user_profile.user_temp_rep_account = Pubkey::default();
    user_profile.user_align_ata = Pubkey::default();
    user_profile.user_rep_ata = Pubkey::default();
    user_profile.bump = ctx.bumps.user_profile;

    msg!("Created user profile for {}", ctx.accounts.user.key());
    Ok(())
}

pub fn initialize_user_topic_balance(ctx: Context<InitializeUserTopicBalance>) -> Result<()> {
    let user_topic_balance = &mut ctx.accounts.user_topic_balance;
    user_topic_balance.user = ctx.accounts.user.key();
    user_topic_balance.topic = ctx.accounts.topic.key();
    user_topic_balance.temp_align_amount = 0;
    user_topic_balance.temp_rep_amount = 0;
    user_topic_balance.locked_temp_rep_amount = 0;
    user_topic_balance.bump = ctx.bumps.user_topic_balance;

    msg!(
        "Initialized UserTopicBalance PDA for user {} and topic {}",
        ctx.accounts.user.key(),
        ctx.accounts.topic.key()
    );
    Ok(())
}

// Make sure to declare this new instruction in lib.rs and mod.rs
