use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use crate::data::*;

// Removed legacy context structures

/// Account constraints for creating a new topic
#[derive(Accounts)]
pub struct CreateTopic<'info> {
    #[account(mut, has_one = authority)]
    pub state: Account<'info, State>,
    
    #[account(
        init,
        payer = authority,
        seeds = [
            b"topic",
            state.topic_count.to_le_bytes().as_ref(),
        ],
        bump,
        space = 8 + // discriminator
                8 + // id
                4 + MAX_TOPIC_NAME_LENGTH + // name (string)
                4 + MAX_TOPIC_DESCRIPTION_LENGTH + // description (string)
                32 + // authority
                8 + // submission_count
                8 + // commit_phase_duration
                8 + // reveal_phase_duration
                1 + // is_active
                1   // bump
    )]
    pub topic: Account<'info, Topic>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Account constraints for submitting data to a specific topic
#[derive(Accounts)]
pub struct SubmitDataToTopic<'info> {
    #[account(mut, seeds = [b"state"], bump)]
    pub state: Account<'info, State>,
    
    #[account(mut, constraint = topic.is_active == true)]
    pub topic: Account<'info, Topic>,
    
    /// The temporary alignment token mint, must be mutable for minting
    #[account(
        mut,
        constraint = *temp_align_mint.to_account_info().key == state.temp_align_mint
    )]
    pub temp_align_mint: Account<'info, Mint>,
    
    /// The protocol-owned tempAlign token account for this contributor
    #[account(
        mut,
        seeds = [b"user_temp_align", contributor.key().as_ref()],
        bump,
        constraint = contributor_temp_align_account.mint == state.temp_align_mint,
        constraint = contributor_temp_align_account.owner == state.key()
    )]
    pub contributor_temp_align_account: Account<'info, TokenAccount>,
    
    /// The new Submission account
    #[account(
        init,
        payer = contributor,
        // Use seeds to ensure uniqueness
        seeds = [
            b"submission",
            state.submission_count.to_le_bytes().as_ref(),
        ],
        bump,
        // Discriminator + contributor pubkey + timestamp + data field + submission PDA bump
        space = 8 + 32 + 8 + (4 + MAX_DATA_REFERENCE_LENGTH) + 1
    )]
    pub submission: Account<'info, Submission>,
    
    /// The link between submission and topic
    #[account(
        init,
        payer = contributor,
        seeds = [
            b"submission_topic_link",
            submission.key().as_ref(),
            topic.key().as_ref(),
        ],
        bump,
        // Discriminator + submission pubkey + topic pubkey + status + phase timestamps + vote counts + committed/revealed counts + bump
        space = 8 + 32 + 32 + 1 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 1
    )]
    pub submission_topic_link: Account<'info, SubmissionTopicLink>,
    
    /// The contributor's user profile (optional)
    #[account(
        mut, 
        seeds = [b"user_profile", contributor.key().as_ref()],
        bump,
        constraint = contributor_profile.user == contributor.key()
    )]
    pub contributor_profile: Option<Account<'info, UserProfile>>,
    
    /// The user making the submission
    #[account(mut)]
    pub contributor: Signer<'info>,
    
    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Account constraints for linking an existing submission to a topic
#[derive(Accounts)]
pub struct LinkSubmissionToTopic<'info> {
    #[account(mut)]
    pub state: Account<'info, State>,
    
    #[account(mut, constraint = topic.is_active == true)]
    pub topic: Account<'info, Topic>,
    
    /// The existing submission to link to the topic
    pub submission: Account<'info, Submission>,
    
    /// The link between submission and topic
    #[account(
        init,
        payer = authority,
        seeds = [
            b"submission_topic_link",
            submission.key().as_ref(),
            topic.key().as_ref(),
        ],
        bump,
        // Discriminator + submission pubkey + topic pubkey + status + phase timestamps + vote counts + committed/revealed counts + bump
        space = 8 + 32 + 32 + 1 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 1
    )]
    pub submission_topic_link: Account<'info, SubmissionTopicLink>,
    
    /// The user linking the submission to the topic (could be contributor or authority)
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Account constraints for committing a vote on a submission within a topic
#[derive(Accounts)]
pub struct CommitVote<'info> {
    pub state: Account<'info, State>,
    
    #[account(mut, constraint = submission_topic_link.status == SubmissionStatus::Pending)]
    pub submission_topic_link: Account<'info, SubmissionTopicLink>,
    
    pub topic: Account<'info, Topic>,
    
    pub submission: Account<'info, Submission>,
    
    #[account(
        init,
        payer = validator,
        seeds = [
            b"vote_commit",
            submission_topic_link.key().as_ref(),
            validator.key().as_ref(),
        ],
        bump,
        // Discriminator + submission_topic_link pubkey + validator pubkey + vote_hash + revealed + finalized + 
        // vote_choice (option) + commit_timestamp + vote_amount + is_permanent_rep + bump
        space = 8 + 32 + 32 + 32 + 1 + 1 + (1 + 1) + 8 + 8 + 1 + 1
    )]
    pub vote_commit: Account<'info, VoteCommit>,
    
    #[account(mut)]
    pub user_profile: Account<'info, UserProfile>,
    
    /// The validator committing the vote
    #[account(mut, constraint = user_profile.user == validator.key())]
    pub validator: Signer<'info>,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Account constraints for revealing a previously committed vote
#[derive(Accounts)]
pub struct RevealVote<'info> {
    pub state: Account<'info, State>,
    
    #[account(mut, constraint = submission_topic_link.status == SubmissionStatus::Pending)]
    pub submission_topic_link: Account<'info, SubmissionTopicLink>,
    
    pub topic: Account<'info, Topic>,
    
    pub submission: Account<'info, Submission>,
    
    #[account(
        mut,
        seeds = [
            b"vote_commit",
            submission_topic_link.key().as_ref(),
            validator.key().as_ref(),
        ],
        bump = vote_commit.bump,
        constraint = vote_commit.revealed == false,
        constraint = vote_commit.validator == validator.key(),
        constraint = vote_commit.submission_topic_link == submission_topic_link.key()
    )]
    pub vote_commit: Account<'info, VoteCommit>,
    
    #[account(mut)]
    pub user_profile: Account<'info, UserProfile>,
    
    /// The validator revealing the vote (must match the original committer)
    #[account(mut, constraint = user_profile.user == validator.key())]
    pub validator: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

/// Account constraints for modifying voting phases (for testing and admin controls)
#[derive(Accounts)]
pub struct SetVotingPhases<'info> {
    pub state: Account<'info, State>,
    
    #[account(mut, constraint = submission_topic_link.status == SubmissionStatus::Pending)]
    pub submission_topic_link: Account<'info, SubmissionTopicLink>,
    
    pub topic: Account<'info, Topic>,
    
    pub submission: Account<'info, Submission>,
    
    /// Only authority can modify phases
    #[account(mut, constraint = state.authority == authority.key())]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

/// Account constraints for finalizing a submission within a topic after voting
#[derive(Accounts)]
pub struct FinalizeSubmission<'info> {
    #[account(seeds = [b"state"], bump)]
    pub state: Account<'info, State>,
    
    #[account(
        mut,
        constraint = submission_topic_link.status == SubmissionStatus::Pending,
        constraint = Clock::get()?.unix_timestamp as u64 > submission_topic_link.reveal_phase_end
    )]
    pub submission_topic_link: Account<'info, SubmissionTopicLink>,
    
    pub topic: Account<'info, Topic>,
    
    pub submission: Account<'info, Submission>,
    
    /// The contributor's user profile with topic-specific token balances
    #[account(
        mut,
        seeds = [b"user_profile", submission.contributor.as_ref()],
        bump,
        constraint = contributor_profile.user == submission.contributor
    )]
    pub contributor_profile: Account<'info, UserProfile>,
    
    /// The protocol-owned tempAlign token account for the contributor
    #[account(
        mut,
        seeds = [b"user_temp_align", submission.contributor.as_ref()],
        bump,
        constraint = contributor_temp_align_account.mint == state.temp_align_mint,
        constraint = contributor_temp_align_account.owner == state.key()
    )]
    pub contributor_temp_align_account: Account<'info, TokenAccount>,
    
    /// The contributor's ATA for permanent alignment tokens (regular user-owned ATA)
    #[account(
        mut,
        constraint = contributor_align_ata.mint == state.align_mint,
        constraint = contributor_align_ata.owner == submission.contributor
    )]
    pub contributor_align_ata: Account<'info, TokenAccount>,
    
    /// The tempAlign mint (for burning)
    #[account(
        mut,
        constraint = temp_align_mint.key() == state.temp_align_mint
    )]
    pub temp_align_mint: Account<'info, Mint>,
    
    /// The Align mint (for minting)
    #[account(
        mut,
        constraint = align_mint.key() == state.align_mint
    )]
    pub align_mint: Account<'info, Mint>,
    
    /// The authority calling this instruction (can be any user)
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,
    
    pub system_program: Program<'info, System>,
}

/// Account constraints for finalizing a validator's vote after submission finalization
/// 
/// Note: This design allows anyone to call finalize_vote, not just the validator themselves.
/// This ensures validators receive rewards or penalties even if they don't explicitly claim them.
/// Future enhancements could include:
/// 1. Batch processing multiple vote finalizations in a single transaction for efficiency
/// 2. An escrow-based approach where tokens are locked during voting and auto-converted after
#[derive(Accounts)]
pub struct FinalizeVote<'info> {
    #[account(seeds = [b"state"], bump)]
    pub state: Account<'info, State>,
    
    #[account(
        mut,
        constraint = submission_topic_link.status != SubmissionStatus::Pending,
    )]
    pub submission_topic_link: Account<'info, SubmissionTopicLink>,
    
    pub topic: Account<'info, Topic>,
    
    pub submission: Account<'info, Submission>,
    
    #[account(
        mut,
        constraint = vote_commit.revealed == true,
        constraint = vote_commit.validator == validator_profile.user,
        constraint = vote_commit.submission_topic_link == submission_topic_link.key()
    )]
    pub vote_commit: Account<'info, VoteCommit>,
    
    /// The validator's user profile
    #[account(mut)]
    pub validator_profile: Account<'info, UserProfile>,
    
    /// The protocol-owned tempRep token account for this validator (for burning)
    #[account(
        mut,
        seeds = [b"user_temp_rep", validator_profile.user.as_ref()],
        bump,
        constraint = validator_temp_rep_account.mint == state.temp_rep_mint,
        constraint = validator_temp_rep_account.owner == state.key()
    )]
    pub validator_temp_rep_account: Account<'info, TokenAccount>,
    
    /// The validator's ATA for permanent reputation tokens (for minting)
    /// This remains user-owned since permanent tokens belong to users
    #[account(
        mut,
        constraint = validator_rep_ata.mint == state.rep_mint,
        constraint = validator_rep_ata.owner == validator_profile.user
    )]
    pub validator_rep_ata: Account<'info, TokenAccount>,
    
    /// The tempRep mint (for burning)
    #[account(
        mut,
        constraint = temp_rep_mint.key() == state.temp_rep_mint
    )]
    pub temp_rep_mint: Account<'info, Mint>,
    
    /// The Rep mint (for minting)
    #[account(
        mut,
        constraint = rep_mint.key() == state.rep_mint
    )]
    pub rep_mint: Account<'info, Mint>,
    
    /// The signer finalizing the vote (can be anyone, not just the validator)
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,
    
    pub system_program: Program<'info, System>,
}

/// Instruction: Initialize the protocol state (Part 1)
///
/// 1) Creates the `State` account (PDA with seeds=["state"]).
/// 2) Initializes the state account with default values
/// 3) Sets `submission_count = 0` and `topic_count = 0`.
#[derive(Accounts)]
pub struct InitializeState<'info> {
    #[account(
        init,
        seeds = [b"state"],
        bump,
        payer = authority,
        space = 8 + 32 + 32 + 32 + 32 + 32 + 1 + 8 + 8 + 8 + 8 + 8 // Discriminator + 4 mints + authority + bump + submission_count + topic_count + tokens_to_mint + 2 phase durations
    )]
    pub state: Account<'info, State>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Instruction: Initialize temporary alignment token mint
///
/// Creates the temp_align_mint token mint with seeds=["temp_align_mint"]
/// This is a non-transferable temporary alignment token
#[derive(Accounts)]
pub struct InitializeTempAlignMint<'info> {
    #[account(mut, seeds = [b"state"], bump)]
    pub state: Account<'info, State>,

    #[account(
        init,
        seeds = [b"temp_align_mint"],
        bump,
        payer = authority,
        mint::decimals = 0,            
        mint::authority = state.key(), // The state PDA is the mint authority
        mint::freeze_authority = state.key()
    )]
    pub temp_align_mint: Account<'info, Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Instruction: Initialize permanent alignment token mint
///
/// Creates the align_mint token mint with seeds=["align_mint"]
/// This is a transferable permanent alignment token
#[derive(Accounts)]
pub struct InitializeAlignMint<'info> {
    #[account(mut, seeds = [b"state"], bump)]
    pub state: Account<'info, State>,

    #[account(
        init,
        seeds = [b"align_mint"],
        bump,
        payer = authority,
        mint::decimals = 0,            
        mint::authority = state.key(), 
        mint::freeze_authority = state.key()
    )]
    pub align_mint: Account<'info, Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Instruction: Initialize temporary reputation token mint
///
/// Creates the temp_rep_mint token mint with seeds=["temp_rep_mint"]
/// This is a non-transferable temporary reputation token
#[derive(Accounts)]
pub struct InitializeTempRepMint<'info> {
    #[account(mut, seeds = [b"state"], bump)]
    pub state: Account<'info, State>,

    #[account(
        init,
        seeds = [b"temp_rep_mint"],
        bump,
        payer = authority,
        mint::decimals = 0,            
        mint::authority = state.key(), 
        mint::freeze_authority = state.key()
    )]
    pub temp_rep_mint: Account<'info, Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Instruction: Initialize permanent reputation token mint
///
/// Creates the rep_mint token mint with seeds=["rep_mint"]
/// This is a non-transferable permanent reputation token
#[derive(Accounts)]
pub struct InitializeRepMint<'info> {
    #[account(mut, seeds = [b"state"], bump)]
    pub state: Account<'info, State>,

    #[account(
        init,
        seeds = [b"rep_mint"],
        bump,
        payer = authority,
        mint::decimals = 0,            
        mint::authority = state.key(), 
        mint::freeze_authority = state.key()
    )]
    pub rep_mint: Account<'info, Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Instruction: Update the number of tokens to mint for each submission
///
/// 1) Updates the `tokens_to_mint` field in the `State` account.
/// 2) Requires the authority to sign.
#[derive(Accounts)]
pub struct UpdateTokensToMint<'info> {
    #[account(mut, has_one = authority)]
    pub state: Account<'info, State>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CreateUserAta<'info> {
    /// The state account containing all mint references
    pub state: Account<'info, State>,

    /// The person paying for creating the ATA
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The user for whom we want to create an ATA
    #[account(mut)]
    pub user: Signer<'info>,

    /// The mint for which we want the user's ATA (only permanent token mints)
    #[account(mut, constraint = 
        *mint.to_account_info().key == state.align_mint || 
        *mint.to_account_info().key == state.rep_mint
    )]
    pub mint: Account<'info, Mint>,

    /// The Associated Token Account (will be created if it doesn't exist)
    /// We do not use `init_if_needed`; we do a CPI call to the ATA program explicitly below.
    /// CHECK: We do not check the ATA account here because it's created by the ATA program.
    #[account(mut)]
    pub user_ata: UncheckedAccount<'info>,

    /// Programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

/// Account constraints for creating protocol-owned temporary tempAlign token account
#[derive(Accounts)]
pub struct CreateUserTempAlignAccount<'info> {
    /// The state account containing protocol configuration
    #[account(seeds = [b"state"], bump)]
    pub state: Account<'info, State>,
    
    /// The payer for the transaction
    #[account(mut)]
    pub payer: Signer<'info>,
    
    /// The user for whom we're creating the account (but not the account owner)
    pub user: Signer<'info>,
    
    /// The mint must be the tempAlign mint
    #[account(mut, constraint = 
        *mint.to_account_info().key == state.temp_align_mint
    )]
    pub mint: Account<'info, Mint>,
    
    /// The token account will be a PDA owned by the program
    /// With the state as the authority, not the user
    #[account(
        init,
        payer = payer,
        token::mint = mint,
        token::authority = state,
        seeds = [b"user_temp_align", user.key().as_ref()],
        bump
    )]
    pub token_account: Account<'info, TokenAccount>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

/// Account constraints for creating protocol-owned temporary tempRep token account
#[derive(Accounts)]
pub struct CreateUserTempRepAccount<'info> {
    /// The state account containing protocol configuration
    #[account(seeds = [b"state"], bump)]
    pub state: Account<'info, State>,
    
    /// The payer for the transaction
    #[account(mut)]
    pub payer: Signer<'info>,
    
    /// The user for whom we're creating the account (but not the account owner)
    pub user: Signer<'info>,
    
    /// The mint must be the tempRep mint
    #[account(mut, constraint = 
        *mint.to_account_info().key == state.temp_rep_mint
    )]
    pub mint: Account<'info, Mint>,
    
    /// The token account will be a PDA owned by the program
    /// With the state as the authority, not the user
    #[account(
        init,
        payer = payer,
        token::mint = mint,
        token::authority = state,
        seeds = [b"user_temp_rep", user.key().as_ref()],
        bump
    )]
    pub token_account: Account<'info, TokenAccount>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

// Removed legacy SubmitData context

/// Account constraints for creating a user profile
#[derive(Accounts)]
pub struct CreateUserProfile<'info> {
    /// The state account containing protocol configuration
    pub state: Account<'info, State>,
    
    /// The user profile to be created (PDA)
    #[account(
        init,
        payer = user,
        seeds = [b"user_profile", user.key().as_ref()],
        bump,
        space = 8 + 32 + 8 + 4 + 200 + 1  // Discriminator + user pubkey + permanent_rep_amount + vector length + topic tokens (initially space for ~10 topics) + bump
    )]
    pub user_profile: Account<'info, UserProfile>,
    
    /// The user creating the profile and paying for the account
    #[account(mut)]
    pub user: Signer<'info>,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// Removed legacy StakeAlignmentTokens context

/// Account constraints for staking temporary alignment tokens for a specific topic
#[derive(Accounts)]
pub struct StakeTopicSpecificTokens<'info> {
    #[account(seeds = [b"state"], bump)]
    pub state: Account<'info, State>,
    
    /// The topic for which tokens are being staked
    pub topic: Account<'info, Topic>,
    
    /// The user's profile that must already exist
    #[account(
        mut,
        seeds = [b"user_profile", user.key().as_ref()],
        bump,
        constraint = user_profile.user == user.key()
    )]
    pub user_profile: Account<'info, UserProfile>,
    
    /// The temporary alignment token mint (source tokens to burn)
    #[account(
        mut,
        constraint = *temp_align_mint.to_account_info().key == state.temp_align_mint
    )]
    pub temp_align_mint: Account<'info, Mint>,
    
    /// The temporary reputation token mint (target tokens to mint)
    #[account(
        mut,
        constraint = *temp_rep_mint.to_account_info().key == state.temp_rep_mint
    )]
    pub temp_rep_mint: Account<'info, Mint>,
    
    /// The protocol-owned tempAlign token account for this user (source)
    #[account(
        mut,
        seeds = [b"user_temp_align", user.key().as_ref()],
        bump,
        constraint = user_temp_align_account.mint == state.temp_align_mint,
        constraint = user_temp_align_account.owner == state.key()
    )]
    pub user_temp_align_account: Account<'info, TokenAccount>,
    
    /// The protocol-owned tempRep token account for this user (target)
    #[account(
        mut,
        seeds = [b"user_temp_rep", user.key().as_ref()],
        bump,
        constraint = user_temp_rep_account.mint == state.temp_rep_mint,
        constraint = user_temp_rep_account.owner == state.key()
    )]
    pub user_temp_rep_account: Account<'info, TokenAccount>,
    
    /// The user associated with these tokens (not the token account owner)
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}