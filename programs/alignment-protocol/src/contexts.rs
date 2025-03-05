use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{AssociatedToken},
    token::{Mint, Token, TokenAccount},
};
use crate::data::*;

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
    #[account(mut)]
    pub state: Account<'info, State>,
    
    #[account(mut, constraint = topic.is_active == true)]
    pub topic: Account<'info, Topic>,
    
    /// The temporary alignment token mint, must be mutable for minting
    #[account(
        mut,
        constraint = *temp_align_mint.to_account_info().key == state.temp_align_mint
    )]
    pub temp_align_mint: Account<'info, Mint>,
    
    /// The user's ATA for temporary alignment tokens
    /// We only mark it mut. We assume it's already created via `create_user_ata`.
    #[account(
        mut,
        constraint = contributor_ata.mint == state.temp_align_mint,
        constraint = contributor_ata.owner == contributor.key()
    )]
    pub contributor_ata: Account<'info, TokenAccount>,
    
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
        constraint = contributor_profile.user == contributor.key(),
        optional
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
    
    #[account(constraint = submission_topic_link.status == SubmissionStatus::Pending)]
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

/// Account constraints for finalizing a submission within a topic after voting
#[derive(Accounts)]
pub struct FinalizeSubmission<'info> {
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
    
    /// The contributor's ATA for temporary alignment tokens
    #[account(
        mut,
        constraint = contributor_temp_align_ata.mint == state.temp_align_mint,
        constraint = contributor_temp_align_ata.owner == submission.contributor
    )]
    pub contributor_temp_align_ata: Account<'info, TokenAccount>,
    
    /// The contributor's ATA for permanent alignment tokens
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
    pub state: Account<'info, State>,
    
    #[account(
        constraint = submission_topic_link.status != SubmissionStatus::Pending,
    )]
    pub submission_topic_link: Account<'info, SubmissionTopicLink>,
    
    pub topic: Account<'info, Topic>,
    
    pub submission: Account<'info, Submission>,
    
    #[account(
        constraint = vote_commit.revealed == true,
        constraint = vote_commit.validator == validator_profile.user,
        constraint = vote_commit.submission_topic_link == submission_topic_link.key()
    )]
    pub vote_commit: Account<'info, VoteCommit>,
    
    /// The validator's user profile
    #[account(mut)]
    pub validator_profile: Account<'info, UserProfile>,
    
    /// The validator's ATA for temporary reputation tokens (for burning)
    #[account(
        mut,
        constraint = validator_temp_rep_ata.mint == state.temp_rep_mint,
        constraint = validator_temp_rep_ata.owner == validator_profile.user
    )]
    pub validator_temp_rep_ata: Account<'info, TokenAccount>,
    
    /// The validator's ATA for permanent reputation tokens (for minting)
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

/// Instruction: Initialize the protocol state + create four token mints
///
/// 1) Creates the `State` account (PDA with seeds=["state"]).
/// 2) Creates the four token mint accounts with different seeds and properties:
///   - temp_align_mint: Non-transferable temporary alignment tokens (seeds=["temp_align_mint"])
///   - align_mint: Transferable permanent alignment tokens (seeds=["align_mint"])
///   - temp_rep_mint: Non-transferable temporary reputation tokens (seeds=["temp_rep_mint"])
///   - rep_mint: Non-transferable permanent reputation tokens (seeds=["rep_mint"])
/// 3) Sets `submission_count = 0`.
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        seeds = [b"state"],
        bump,
        payer = authority,
        space = 8 + 32 + 32 + 32 + 32 + 32 + 1 + 8 + 8 + 8 + 8 + 8 // Discriminator + 4 mints + authority + bump + submission_count + topic_count + tokens_to_mint + 2 phase durations
    )]
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

    /// The mint for which we want the user's ATA (must match one of the four mints in state)
    #[account(mut, constraint = 
        *mint.to_account_info().key == state.temp_align_mint || 
        *mint.to_account_info().key == state.align_mint || 
        *mint.to_account_info().key == state.temp_rep_mint || 
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

/// Instruction: Store data directly in your program's Submission account and mint temporary alignment tokens.
/// Note: This only creates the submission. For voting, the submission must be linked to a topic.
#[derive(Accounts)]
pub struct SubmitData<'info> {
    #[account(mut)]
    pub state: Account<'info, State>,

    /// The temporary alignment token mint, must be mutable for minting
    #[account(
        mut,
        constraint = *temp_align_mint.to_account_info().key == state.temp_align_mint
    )]
    pub temp_align_mint: Account<'info, Mint>,

    /// The user's ATA for temporary alignment tokens
    /// We only mark it mut. We assume it's already created via `create_user_ata`.
    #[account(
        mut,
        constraint = contributor_ata.mint == state.temp_align_mint,
        constraint = contributor_ata.owner == contributor.key()
    )]
    pub contributor_ata: Account<'info, TokenAccount>,

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
        // Discriminator + contributor pubkey + timestamp + data_reference field + bump
        space = 8 + 32 + 8 + (4 + MAX_DATA_REFERENCE_LENGTH) + 1
    )]
    pub submission: Account<'info, Submission>,

    /// The user making the submission
    #[account(mut)]
    pub contributor: Signer<'info>,

    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

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
        space = 8 + 32 + 8 + 8 + 4 + 200 + 1  // Discriminator + user pubkey + temp_rep_amount + permanent_rep_amount + vector length + topic tokens (initially space for ~10 topics) + bump
    )]
    pub user_profile: Account<'info, UserProfile>,
    
    /// The user creating the profile and paying for the account
    #[account(mut)]
    pub user: Signer<'info>,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Account constraints for staking temporary alignment tokens
#[derive(Accounts)]
pub struct StakeAlignmentTokens<'info> {
    #[account(mut)]
    pub state: Account<'info, State>,
    
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
    
    /// The user's ATA for temporary alignment tokens (source)
    #[account(
        mut,
        constraint = user_temp_align_ata.mint == state.temp_align_mint,
        constraint = user_temp_align_ata.owner == user.key()
    )]
    pub user_temp_align_ata: Account<'info, TokenAccount>,
    
    /// The user's ATA for temporary reputation tokens (target)
    #[account(
        mut,
        constraint = user_temp_rep_ata.mint == state.temp_rep_mint,
        constraint = user_temp_rep_ata.owner == user.key()
    )]
    pub user_temp_rep_ata: Account<'info, TokenAccount>,
    
    /// The user performing the stake
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Account constraints for staking temporary alignment tokens for a specific topic
#[derive(Accounts)]
pub struct StakeTopicSpecificTokens<'info> {
    #[account(mut)]
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
    
    /// The user's ATA for temporary alignment tokens (source)
    #[account(
        mut,
        constraint = user_temp_align_ata.mint == state.temp_align_mint,
        constraint = user_temp_align_ata.owner == user.key()
    )]
    pub user_temp_align_ata: Account<'info, TokenAccount>,
    
    /// The user's ATA for temporary reputation tokens (target)
    #[account(
        mut,
        constraint = user_temp_rep_ata.mint == state.temp_rep_mint,
        constraint = user_temp_rep_ata.owner == user.key()
    )]
    pub user_temp_rep_ata: Account<'info, TokenAccount>,
    
    /// The user performing the stake
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}