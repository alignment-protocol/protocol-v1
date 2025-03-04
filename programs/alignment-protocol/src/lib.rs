use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{create, AssociatedToken, Create},
    token::{self, Burn, Mint, MintTo, Token, TokenAccount},
};
use sha2::{Digest, Sha256};

declare_id!("BMYn8rtstaZhzFZtgMVMY9io1zhnqacr3yANZrgkv7DF");

// Maximum lengths for strings
pub const MAX_TOPIC_NAME_LENGTH: usize = 64;
pub const MAX_TOPIC_DESCRIPTION_LENGTH: usize = 256;
pub const MAX_DATA_REFERENCE_LENGTH: usize = 128; // For IPFS/Arweave hashes or transaction references

// ------------------------------
//          Data Structs
// ------------------------------

/// Global state account for this protocol
#[account]
pub struct State {
    /// The temporary alignment token mint (non-transferable until converted)
    pub temp_align_mint: Pubkey,
    
    /// The permanent alignment token mint (transferable)
    pub align_mint: Pubkey,
    
    /// The temporary reputation token mint (non-transferable)
    pub temp_rep_mint: Pubkey,
    
    /// The permanent reputation token mint (non-transferable)
    pub rep_mint: Pubkey,

    /// The protocol authority (admin, DAO, etc.)
    pub authority: Pubkey,

    /// Bump seed for the state PDA
    pub bump: u8,

    /// Counts how many submissions have been made
    pub submission_count: u64,
    
    /// Counts how many topics have been created
    pub topic_count: u64,

    /// The number of tokens to mint for each submission
    pub tokens_to_mint: u64,
    
    /// Default duration for commit phase in seconds (24 hours)
    pub default_commit_phase_duration: u64,
    
    /// Default duration for reveal phase in seconds (24 hours)
    pub default_reveal_phase_duration: u64,
}

/// Each submission entry
#[account]
pub struct Submission {
    /// The user who submitted the data
    pub contributor: Pubkey,

    /// Unix timestamp of when they submitted
    pub timestamp: u64,

    /// Arbitrary string to store data reference (IPFS hash, Arweave ID, etc.)
    pub data_reference: String,
    
    /// Bump seed for the submission PDA
    pub bump: u8,
}

/// Status of a submission
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum SubmissionStatus {
    /// Submission is pending voting/finalization
    Pending,
    
    /// Submission has been accepted by voters
    Accepted,
    
    /// Submission has been rejected by voters
    Rejected,
}

/// User profile account to track reputation
#[account]
pub struct UserProfile {
    /// The user's public key
    pub user: Pubkey,
    
    /// Amount of temporary reputation tokens staked
    pub temp_rep_amount: u64,
    
    /// Amount of permanent reputation tokens earned
    pub permanent_rep_amount: u64,
    
    /// Bump seed for the user profile PDA
    pub bump: u8,
}

/// Topic/Corpus account for organizing submissions
#[account]
pub struct Topic {
    /// Unique identifier for the topic
    pub id: u64,
    
    /// Name of the topic
    pub name: String,
    
    /// Description of the topic
    pub description: String,
    
    /// Creator of the topic (authority or eventually DAO)
    pub authority: Pubkey,
    
    /// Count of submissions in this topic
    pub submission_count: u64,
    
    /// Duration of the commit phase in seconds
    pub commit_phase_duration: u64,
    
    /// Duration of the reveal phase in seconds
    pub reveal_phase_duration: u64,
    
    /// Whether the topic is active and accepting submissions
    pub is_active: bool,
    
    /// Bump seed for the topic PDA
    pub bump: u8,
}

/// Tracks the relationship between a submission and a topic
#[account]
pub struct SubmissionTopicLink {
    /// The submission this link refers to
    pub submission: Pubkey,
    
    /// The topic this link refers to
    pub topic: Pubkey,
    
    /// Status of this submission within this specific topic
    pub status: SubmissionStatus,
    
    /// Start timestamp for the commit phase
    pub commit_phase_start: u64,
    
    /// End timestamp for the commit phase
    pub commit_phase_end: u64,
    
    /// Start timestamp for the reveal phase
    pub reveal_phase_start: u64,
    
    /// End timestamp for the reveal phase
    pub reveal_phase_end: u64,
    
    /// Total yes voting power received (quadratic)
    pub yes_voting_power: u64,
    
    /// Total no voting power received (quadratic)
    pub no_voting_power: u64,
    
    /// Total number of committed votes
    pub total_committed_votes: u64,
    
    /// Total number of revealed votes
    pub total_revealed_votes: u64,
    
    /// Bump seed for the link PDA
    pub bump: u8,
}

/// Vote direction (Yes/No)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum VoteChoice {
    Yes,
    No,
}

/// Vote commit account - stores the hash of a user's vote during the commit phase
#[account]
pub struct VoteCommit {
    /// The link between submission and topic being voted on
    pub submission_topic_link: Pubkey,
    
    /// The validator who created this vote commit
    pub validator: Pubkey,
    
    /// The hashed vote: SHA-256(validator pubkey + submission_topic_link pubkey + vote choice + nonce)
    pub vote_hash: [u8; 32],
    
    /// Whether this vote has been revealed yet
    pub revealed: bool,
    
    /// Whether this vote has been finalized (tokens converted or burned)
    pub finalized: bool,
    
    /// The revealed vote choice (only valid after reveal)
    pub vote_choice: Option<VoteChoice>,
    
    /// Commit timestamp
    pub commit_timestamp: u64,
    
    /// The amount of tempRep or Rep tokens committed to this vote
    pub vote_amount: u64,
    
    /// Whether this is using permanent Rep (true) or temporary tempRep (false)
    pub is_permanent_rep: bool,
    
    /// Bump seed for the vote commit PDA
    pub bump: u8,
}

// ------------------------------
//          Error Codes
// ------------------------------
#[error_code]
pub enum ErrorCode {
    #[msg("Invalid authority for this state")]
    InvalidAuthority,

    #[msg("Arithmetic overflow occurred")]
    Overflow,
    
    #[msg("Insufficient token balance for staking")]
    InsufficientTokenBalance,
    
    #[msg("Token mint mismatch")]
    TokenMintMismatch,
    
    #[msg("Invalid token account")]
    InvalidTokenAccount,
    
    #[msg("Invalid user profile")]
    InvalidUserProfile,
    
    #[msg("User profile already initialized")]
    UserProfileAlreadyInitialized,
    
    #[msg("Cannot stake zero tokens")]
    ZeroStakeAmount,
    
    // Topic-related errors
    #[msg("Topic name cannot be empty")]
    EmptyTopicName,
    
    #[msg("Topic name exceeds maximum length")]
    TopicNameTooLong,
    
    #[msg("Topic description exceeds maximum length")]
    TopicDescriptionTooLong,
    
    #[msg("Topic is inactive")]
    TopicInactive,
    
    #[msg("No active topics available for submission")]
    NoActiveTopics,
    
    #[msg("Submission already exists in this topic")]
    SubmissionAlreadyInTopic,
    
    // Voting-related errors
    #[msg("Vote has already been committed")]
    VoteAlreadyCommitted,
    
    #[msg("Vote has already been revealed")]
    VoteAlreadyRevealed,
    
    #[msg("Invalid vote hash")]
    InvalidVoteHash,
    
    #[msg("Validator has no reputation tokens for this topic")]
    NoReputationForTopic,
    
    #[msg("Submission is not in the pending state")]
    SubmissionNotPending,
    
    #[msg("Vote amount exceeds available reputation")]
    InsufficientVotingPower,
    
    #[msg("Vote amount must be greater than zero")]
    ZeroVoteAmount,
    
    #[msg("Commit phase has not started yet")]
    CommitPhaseNotStarted,
    
    #[msg("Commit phase has ended")]
    CommitPhaseEnded,
    
    #[msg("Reveal phase has not started yet")]
    RevealPhaseNotStarted,
    
    #[msg("Reveal phase has ended")]
    RevealPhaseEnded,
    
    #[msg("Vote has already been finalized")]
    VoteAlreadyFinalized,
}

// ------------------------------
//          Instructions
// ------------------------------

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
    
    /// The user making the submission
    #[account(mut)]
    pub contributor: Signer<'info>,
    
    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,
    
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
    
    /// The contributor's user profile
    #[account(
        mut,
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
        space = 8 + 32 + 8 + 8 + 1  // Discriminator + user pubkey + temp_rep_amount + permanent_rep_amount + bump
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

// ------------------------------
//          Helper Functions
// ------------------------------

/// Calculates the square root of a number for quadratic voting power
pub fn calculate_quadratic_voting_power(amount: u64) -> u64 {
    (amount as f64).sqrt() as u64
}

// ------------------------------
//          Program Logic
// ------------------------------
#[program]
pub mod alignment_protocol {
    use super::*;

    /// Instruction handler: initialize the protocol with four token mints
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

    /// Instruction handler: update the number of tokens to mint for each submission
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

    /// Instruction handler: explicitly create user's ATA
    ///
    /// This does NOT use `init_if_needed`. Instead, it does a CPI to the associated_token::create method.
    /// If the ATA already exists, this transaction will fail (unless you do extra checks).
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

    /// Instruction handler: Create a user profile for tracking reputation
    /// 
    /// This creates a new PDA account to store the user's reputation metrics
    pub fn create_user_profile(ctx: Context<CreateUserProfile>) -> Result<()> {
        // Initialize the user profile fields
        let user_profile = &mut ctx.accounts.user_profile;
        user_profile.user = ctx.accounts.user.key();
        user_profile.temp_rep_amount = 0;
        user_profile.permanent_rep_amount = 0;
        user_profile.bump = ctx.bumps.user_profile;
        
        msg!("Created user profile for {}", ctx.accounts.user.key());
        Ok(())
    }
    
    /// Instruction handler: Create a new topic
    ///
    /// This creates a new topic that submissions can be added to.
    /// Only the protocol authority can create topics.
    pub fn create_topic(
        ctx: Context<CreateTopic>,
        name: String,
        description: String,
        commit_phase_duration: Option<u64>,
        reveal_phase_duration: Option<u64>,
    ) -> Result<()> {
        // Validate inputs
        if name.is_empty() {
            return Err(ErrorCode::EmptyTopicName.into());
        }
        
        if name.len() > MAX_TOPIC_NAME_LENGTH {
            return Err(ErrorCode::TopicNameTooLong.into());
        }
        
        if description.len() > MAX_TOPIC_DESCRIPTION_LENGTH {
            return Err(ErrorCode::TopicDescriptionTooLong.into());
        }
        
        // Initialize the topic
        let topic = &mut ctx.accounts.topic;
        let state = &mut ctx.accounts.state;
        
        topic.id = state.topic_count;
        topic.name = name.clone();
        topic.description = description.clone();
        topic.authority = ctx.accounts.authority.key();
        topic.submission_count = 0;
        topic.is_active = true;
        topic.bump = ctx.bumps.topic;
        
        // Set the commit and reveal phase durations - use provided values or defaults from state
        topic.commit_phase_duration = commit_phase_duration.unwrap_or(state.default_commit_phase_duration);
        topic.reveal_phase_duration = reveal_phase_duration.unwrap_or(state.default_reveal_phase_duration);
        
        // Increment the topic count
        state.topic_count = state.topic_count.checked_add(1).ok_or(ErrorCode::Overflow)?;
        
        msg!("Created new topic: {} (ID: {})", name, topic.id);
        msg!("Description: {}", description);
        msg!("Commit phase duration: {} seconds", topic.commit_phase_duration);
        msg!("Reveal phase duration: {} seconds", topic.reveal_phase_duration);
        
        Ok(())
    }
    
    /// Instruction handler: Submit data to a specific topic
    ///
    /// This creates a submission and links it to a topic, setting up the voting phases.
    pub fn submit_data_to_topic(
        ctx: Context<SubmitDataToTopic>,
        data_reference: String,
    ) -> Result<()> {
        // Validate inputs
        if data_reference.len() > MAX_DATA_REFERENCE_LENGTH {
            return Err(error!(ErrorCode::TopicDescriptionTooLong));
        }
        
        // Get current time
        let current_time = Clock::get()?.unix_timestamp as u64;
        
        // Fill out the Submission account
        let submission = &mut ctx.accounts.submission;
        submission.contributor = ctx.accounts.contributor.key();
        submission.timestamp = current_time;
        submission.data_reference = data_reference.clone();
        submission.bump = ctx.bumps.submission;
        
        // Fill out the SubmissionTopicLink account
        let link = &mut ctx.accounts.submission_topic_link;
        let topic = &mut ctx.accounts.topic;
        
        link.submission = ctx.accounts.submission.key();
        link.topic = ctx.accounts.topic.key();
        link.status = SubmissionStatus::Pending;
        link.bump = ctx.bumps.submission_topic_link;
        
        // Set up voting phases based on topic durations
        link.commit_phase_start = current_time;
        link.commit_phase_end = current_time.checked_add(topic.commit_phase_duration).ok_or(ErrorCode::Overflow)?;
        link.reveal_phase_start = link.commit_phase_end;
        link.reveal_phase_end = link.reveal_phase_start.checked_add(topic.reveal_phase_duration).ok_or(ErrorCode::Overflow)?;
        
        // Initialize vote counts
        link.yes_voting_power = 0;
        link.no_voting_power = 0;
        link.total_committed_votes = 0;
        link.total_revealed_votes = 0;
        
        // Mint temporary alignment tokens to the contributor if configured
        if ctx.accounts.state.tokens_to_mint > 0 {
            let state_bump = ctx.accounts.state.bump;
            let seeds = &[b"state".as_ref(), &[state_bump]];
            let signer = &[&seeds[..]];
            
            // CPI to the Token Program's 'mint_to'
            let cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.temp_align_mint.to_account_info(),
                    to: ctx.accounts.contributor_ata.to_account_info(),
                    authority: ctx.accounts.state.to_account_info(),
                },
            )
            .with_signer(signer);
            
            token::mint_to(cpi_ctx, ctx.accounts.state.tokens_to_mint)?;
            
            msg!(
                "Minted {} tempAlign tokens to {}",
                ctx.accounts.state.tokens_to_mint,
                ctx.accounts.contributor_ata.key()
            );
        }
        
        // Increment submission counts
        let state = &mut ctx.accounts.state;
        state.submission_count = state.submission_count.checked_add(1).ok_or(ErrorCode::Overflow)?;
        
        topic.submission_count = topic.submission_count.checked_add(1).ok_or(ErrorCode::Overflow)?;
        
        msg!("New submission added to topic '{}'", topic.name);
        msg!("Data reference: {}", data_reference);
        msg!("Commit phase: {} to {}", link.commit_phase_start, link.commit_phase_end);
        msg!("Reveal phase: {} to {}", link.reveal_phase_start, link.reveal_phase_end);
        
        Ok(())
    }
    
    /// Instruction handler: Stake temporary alignment tokens to get temporary reputation tokens
    ///
    /// Burns tempAlign tokens and mints an equal amount of tempRep tokens
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
    
    /// Instruction handler: Submit data directly on-chain
    /// 1) Creates new `Submission` account with the given data.
    /// 2) Mints a fixed number of temporary alignment tokens to the user's ATA.
    /// 3) Increments the state's submission_count.
    pub fn submit_data(ctx: Context<SubmitData>, data_reference: String) -> Result<()> {
        // Validate inputs
        if data_reference.len() > MAX_DATA_REFERENCE_LENGTH {
            return Err(error!(ErrorCode::TopicDescriptionTooLong));
        }
        
        // 1) Fill out the Submission account
        let submission = &mut ctx.accounts.submission;
        submission.contributor = ctx.accounts.contributor.key();
        submission.timestamp = Clock::get()?.unix_timestamp as u64;
        submission.data_reference = data_reference.clone(); // store the reference (hash, tx ID, etc.)
        submission.bump = ctx.bumps.submission;

        // 2) Mint temporary alignment tokens to the contributor
        if ctx.accounts.state.tokens_to_mint > 0 {
            let state_bump = ctx.accounts.state.bump;
            let seeds = &[b"state".as_ref(), &[state_bump]];
            let signer = &[&seeds[..]];

            // CPI to the Token Program's 'mint_to'
            let cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.temp_align_mint.to_account_info(),
                    to: ctx.accounts.contributor_ata.to_account_info(),
                    authority: ctx.accounts.state.to_account_info(),
                },
            )
            .with_signer(signer);

            token::mint_to(cpi_ctx, ctx.accounts.state.tokens_to_mint)?;
            msg!(
                "Minted {} tempAlign tokens to {}",
                ctx.accounts.state.tokens_to_mint,
                ctx.accounts.contributor_ata.key()
            );
        }

        // 3) Increment submission_count
        let state_acc = &mut ctx.accounts.state;
        state_acc.submission_count = state_acc
            .submission_count
            .checked_add(1)
            .ok_or(ErrorCode::Overflow)?;

        msg!("New submission on-chain (reference): {}", data_reference);
        msg!("NOTE: This submission is not linked to any topic yet. Use submit_data_to_topic instead.");
        Ok(())
    }
    
    /// Instruction handler: Commit a vote on a submission within a topic
    ///
    /// This creates a vote commitment without revealing the actual vote choice.
    /// The actual vote is hashed with a nonce for privacy during the commit phase.
    pub fn commit_vote(
        ctx: Context<CommitVote>,
        vote_hash: [u8; 32],
        vote_amount: u64,
        is_permanent_rep: bool,
    ) -> Result<()> {
        // Get current time to validate voting window
        let current_time = Clock::get()?.unix_timestamp as u64;
        let link = &ctx.accounts.submission_topic_link;
        
        // Check if commit phase is active
        if current_time < link.commit_phase_start {
            return Err(ErrorCode::CommitPhaseNotStarted.into());
        }
        
        if current_time > link.commit_phase_end {
            return Err(ErrorCode::CommitPhaseEnded.into());
        }
        
        // Validate the vote amount based on token type
        if vote_amount == 0 {
            return Err(ErrorCode::ZeroVoteAmount.into());
        }
        
        // Check if user has enough Rep (either temp or permanent)
        if is_permanent_rep {
            // Voting with permanent Rep - can vote across any topic
            if ctx.accounts.user_profile.permanent_rep_amount < vote_amount {
                return Err(ErrorCode::InsufficientVotingPower.into());
            }
        } else {
            // Voting with tempRep - can only vote within the topic it was gained for
            if ctx.accounts.user_profile.temp_rep_amount < vote_amount {
                return Err(ErrorCode::InsufficientVotingPower.into());
            }
            
            // For MVP, we allow any tempRep to be used for any topic
            // In the future, we'll track tempRep by topic and only allow voting within that topic
        }
        
        // Initialize the vote commit
        let vote_commit = &mut ctx.accounts.vote_commit;
        vote_commit.submission_topic_link = ctx.accounts.submission_topic_link.key();
        vote_commit.validator = ctx.accounts.validator.key();
        vote_commit.vote_hash = vote_hash;
        vote_commit.revealed = false;
        vote_commit.finalized = false;
        vote_commit.vote_choice = None;
        vote_commit.commit_timestamp = current_time;
        vote_commit.vote_amount = vote_amount;
        vote_commit.is_permanent_rep = is_permanent_rep;
        vote_commit.bump = ctx.bumps.vote_commit;
        
        // Increment the submission-topic link's committed votes counter
        let link = &mut ctx.accounts.submission_topic_link;
        link.total_committed_votes = link.total_committed_votes
            .checked_add(1)
            .ok_or(ErrorCode::Overflow)?;
        
        msg!("Vote committed for submission in topic '{}'", ctx.accounts.topic.name);
        msg!("Vote amount: {}", vote_amount);
        msg!("Using {} Rep", if is_permanent_rep { "permanent" } else { "temporary" });
        
        Ok(())
    }
    
    /// Instruction handler: Reveal a previously committed vote
    ///
    /// This reveals the actual vote choice and verifies it matches the previously committed hash.
    /// If valid, it adds the voter's voting power to the appropriate yes/no counter.
    pub fn reveal_vote(
        ctx: Context<RevealVote>,
        vote_choice: VoteChoice,
        nonce: String,
    ) -> Result<()> {
        // Get current time to validate voting window
        let current_time = Clock::get()?.unix_timestamp as u64;
        let link = &ctx.accounts.submission_topic_link;
        
        // Check if reveal phase is active
        if current_time < link.reveal_phase_start {
            return Err(ErrorCode::RevealPhaseNotStarted.into());
        }
        
        if current_time > link.reveal_phase_end {
            return Err(ErrorCode::RevealPhaseEnded.into());
        }
        
        // Reconstruct the hash from the reveal data and verify it matches the commit
        let vote_commit = &mut ctx.accounts.vote_commit;
        
        // Create the pre-image for the hash
        // Format: validator pubkey + submission_topic_link pubkey + vote choice (0 for Yes, 1 for No) + nonce
        let mut hasher = Sha256::new();
        hasher.update(ctx.accounts.validator.key().as_ref());
        hasher.update(ctx.accounts.submission_topic_link.key().as_ref());
        hasher.update(&[vote_choice as u8]);
        hasher.update(nonce.as_bytes());
        
        let reconstructed_hash: [u8; 32] = hasher.finalize().into();
        
        // Verify that the reconstructed hash matches the stored hash
        if reconstructed_hash != vote_commit.vote_hash {
            return Err(ErrorCode::InvalidVoteHash.into());
        }
        
        // Mark the vote as revealed and store the vote choice
        vote_commit.revealed = true;
        vote_commit.vote_choice = Some(vote_choice);
        
        // Calculate voting power (quadratic)
        let voting_power = calculate_quadratic_voting_power(vote_commit.vote_amount);
        
        // Add the voting power to the appropriate counter
        let link = &mut ctx.accounts.submission_topic_link;
        match vote_choice {
            VoteChoice::Yes => {
                link.yes_voting_power = link.yes_voting_power
                    .checked_add(voting_power)
                    .ok_or(ErrorCode::Overflow)?;
            },
            VoteChoice::No => {
                link.no_voting_power = link.no_voting_power
                    .checked_add(voting_power)
                    .ok_or(ErrorCode::Overflow)?;
            },
        }
        
        // Increment the revealed votes counter
        link.total_revealed_votes = link.total_revealed_votes
            .checked_add(1)
            .ok_or(ErrorCode::Overflow)?;
        
        msg!("Vote revealed for submission in topic '{}'", ctx.accounts.topic.name);
        msg!("Vote choice: {:?}", vote_choice);
        msg!("Voting power (quadratic): {}", voting_power);
        
        Ok(())
    }
    
    /// Instruction handler: Finalize a submission within a topic after voting
    ///
    /// This determines if a submission is accepted or rejected based on voting results.
    /// For accepted submissions, it converts contributor's tempAlign tokens to permanent Align tokens.
    pub fn finalize_submission(
        ctx: Context<FinalizeSubmission>,
    ) -> Result<()> {
        // Calculate final vote tallies
        let link = &mut ctx.accounts.submission_topic_link;
        
        // Determine if the submission is accepted or rejected
        let is_accepted = link.yes_voting_power > link.no_voting_power;
        
        // Update submission status
        if is_accepted {
            link.status = SubmissionStatus::Accepted;
            
            // Convert contributor's tempAlign tokens to permanent Align tokens
            // For simplicity, we assume a 1:1 conversion rate in the MVP
            
            // Get conversion amount (tempAlign to burn and Align to mint)
            // In a real implementation, this might be a function of the submission quality
            let conversion_amount = ctx.accounts.state.tokens_to_mint;
            
            // Check if the contributor has enough tempAlign tokens
            if ctx.accounts.contributor_temp_align_ata.amount < conversion_amount {
                return Err(ErrorCode::InsufficientTokenBalance.into());
            }
            
            // 1. Burn tempAlign tokens from contributor
            let burn_cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.temp_align_mint.to_account_info(),
                    from: ctx.accounts.contributor_temp_align_ata.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            );
            
            token::burn(burn_cpi_ctx, conversion_amount)?;
            
            // 2. Mint permanent Align tokens to contributor
            let state_bump = ctx.accounts.state.bump;
            let seeds = &[b"state".as_ref(), &[state_bump]];
            let signer = &[&seeds[..]];
            
            let mint_cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.align_mint.to_account_info(),
                    to: ctx.accounts.contributor_align_ata.to_account_info(),
                    authority: ctx.accounts.state.to_account_info(),
                },
            )
            .with_signer(signer);
            
            token::mint_to(mint_cpi_ctx, conversion_amount)?;
            
            msg!("Submission accepted! Converted {} tempAlign to {} Align for contributor",
                conversion_amount, conversion_amount);
        } else {
            // If rejected, no token conversion happens
            link.status = SubmissionStatus::Rejected;
            msg!("Submission rejected. No token conversion performed.");
        }
        
        // Log the voting results
        msg!("Finalized submission in topic '{}' with status: {:?}", 
            ctx.accounts.topic.name, 
            link.status);
        msg!("Final vote tally: {} YES vs {} NO", 
            link.yes_voting_power, 
            link.no_voting_power);
        
        Ok(())
    }
    
    /// Instruction handler: Finalize a validator's vote after submission has been finalized
    ///
    /// This processes the token rewards or penalties for a validator based on their vote:
    /// - For correct votes: Burn tempRep tokens and mint permanent Rep tokens
    /// - For incorrect votes: Just burn tempRep tokens with no replacement
    /// - No penalty for permanent Rep tokens used for voting
    pub fn finalize_vote(
        ctx: Context<FinalizeVote>,
    ) -> Result<()> {
        // Check if the vote has already been finalized
        if ctx.accounts.vote_commit.finalized {
            return Err(ErrorCode::VoteAlreadyFinalized.into());
        }
        
        // Get the consensus outcome (accepted/rejected)
        let consensus_is_yes = ctx.accounts.submission_topic_link.status == SubmissionStatus::Accepted;
        
        // Get the validator's vote choice
        let vote_choice = ctx.accounts.vote_commit.vote_choice.ok_or(ErrorCode::InvalidVoteHash)?;
        let voted_yes = vote_choice == VoteChoice::Yes;
        
        // Check if the validator voted with the consensus
        let voted_with_consensus = (consensus_is_yes && voted_yes) || (!consensus_is_yes && !voted_yes);
        
        // Only process token conversions for temporary reputation
        // With permanent reputation, we don't burn or reward tokens for now
        if !ctx.accounts.vote_commit.is_permanent_rep {
            let vote_amount = ctx.accounts.vote_commit.vote_amount;
            
            if voted_with_consensus {
                // Validator voted correctly - convert tempRep to permanent Rep
                
                // Check if the validator has enough tempRep tokens
                if ctx.accounts.validator_temp_rep_ata.amount < vote_amount {
                    return Err(ErrorCode::InsufficientTokenBalance.into());
                }
                
                // 1. Burn tempRep tokens
                let burn_cpi_ctx = CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Burn {
                        mint: ctx.accounts.temp_rep_mint.to_account_info(),
                        from: ctx.accounts.validator_temp_rep_ata.to_account_info(),
                        authority: ctx.accounts.authority.to_account_info(),
                    },
                );
                
                token::burn(burn_cpi_ctx, vote_amount)?;
                
                // 2. Mint permanent Rep tokens
                let state_bump = ctx.accounts.state.bump;
                let seeds = &[b"state".as_ref(), &[state_bump]];
                let signer = &[&seeds[..]];
                
                let mint_cpi_ctx = CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    MintTo {
                        mint: ctx.accounts.rep_mint.to_account_info(),
                        to: ctx.accounts.validator_rep_ata.to_account_info(),
                        authority: ctx.accounts.state.to_account_info(),
                    },
                )
                .with_signer(signer);
                
                token::mint_to(mint_cpi_ctx, vote_amount)?;
                
                // Update validator profile
                let validator_profile = &mut ctx.accounts.validator_profile;
                validator_profile.temp_rep_amount = validator_profile.temp_rep_amount
                    .checked_sub(vote_amount)
                    .ok_or(ErrorCode::Overflow)?;
                    
                validator_profile.permanent_rep_amount = validator_profile.permanent_rep_amount
                    .checked_add(vote_amount)
                    .ok_or(ErrorCode::Overflow)?;
                
                msg!(
                    "Validator voted correctly! Converted {} tempRep to {} permanent Rep",
                    vote_amount,
                    vote_amount
                );
            } else {
                // Validator voted incorrectly - burn tempRep tokens with no replacement
                
                // Check if the validator has enough tempRep tokens
                if ctx.accounts.validator_temp_rep_ata.amount < vote_amount {
                    return Err(ErrorCode::InsufficientTokenBalance.into());
                }
                
                // Burn tempRep tokens
                let burn_cpi_ctx = CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Burn {
                        mint: ctx.accounts.temp_rep_mint.to_account_info(),
                        from: ctx.accounts.validator_temp_rep_ata.to_account_info(),
                        authority: ctx.accounts.authority.to_account_info(),
                    },
                );
                
                token::burn(burn_cpi_ctx, vote_amount)?;
                
                // Update validator profile
                let validator_profile = &mut ctx.accounts.validator_profile;
                validator_profile.temp_rep_amount = validator_profile.temp_rep_amount
                    .checked_sub(vote_amount)
                    .ok_or(ErrorCode::Overflow)?;
                
                msg!(
                    "Validator voted incorrectly. Burned {} tempRep tokens with no replacement",
                    vote_amount
                );
            }
        } else {
            // Using permanent Rep tokens
            // For MVP we don't apply penalties to permanent Rep
            msg!("Vote was made with permanent Rep tokens. No token conversion applied.");
        }
        
        // Mark the vote as finalized
        let vote_commit = &mut ctx.accounts.vote_commit;
        vote_commit.finalized = true;
        
        msg!(
            "Finalized vote for validator {} on submission in topic '{}'",
            ctx.accounts.validator_profile.user,
            ctx.accounts.topic.name
        );
        
        Ok(())
    }
}
