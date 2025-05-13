use crate::data::*;
use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

// Removed legacy context structures

/// Account constraints for creating a new topic
#[derive(Accounts)]
pub struct CreateTopic<'info> {
    /// Global protocol state (mutable for topic_count increment). Any signer can now create topics,
    /// so we only require mut access without restricting to the stored authority.
    #[account(mut, seeds = [b"state"], bump)]
    pub state: Account<'info, State>,

    /// The Topic PDA to be initialized. Rent is paid by a dedicated `payer`
    /// account so that the logical topic creator can be separated from the
    /// wallet that covers the transaction fees.  This allows a relayer or
    /// backend service to subsidise account creation while still recording
    /// the end-user as the topic `creator` on-chain.
    #[account(
        init,
        // Use the standalone `payer` account to fund the new Topic PDA.
        payer = payer,
        seeds = [
            b"topic",
            state.topic_count.to_le_bytes().as_ref(),
        ],
        bump,
        space = 8 + // discriminator
                4 + MAX_TOPIC_NAME_LENGTH + // name (string)
                4 + MAX_TOPIC_DESCRIPTION_LENGTH + // description (string)
                32 + // authority (creator)
                8 + // submission_count
                8 + // commit_phase_duration
                8 + // reveal_phase_duration
                1 + // is_active
                1   // bump
    )]
    pub topic: Account<'info, Topic>,

    /// Wallet that funds the account creation. Must sign and be mutable
    /// because its lamports balance decreases.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The logical creator of the topic. Stored on-chain for attribution
    /// but does not need to sign or pay.
    pub creator: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Account constraints for updating an existing topic's configuration
#[derive(Accounts)]
pub struct UpdateTopic<'info> {
    /// Global protocol state (read‑only except for verifying authority)
    pub state: Account<'info, State>,

    /// The topic to update
    #[account(mut)]
    pub topic: Account<'info, Topic>,

    /// Signer must be either the global protocol authority or the topic creator
    pub authority: Signer<'info>,
}

/// Account constraints for submitting data to a specific topic
#[derive(Accounts)]
#[instruction(data_reference: String, current_submission_index: u64)]
pub struct SubmitDataToTopic<'info> {
    #[account(seeds = [b"state"], bump)]
    pub state: Account<'info, State>,

    #[account(mut, constraint = topic.is_active @ ErrorCode::TopicInactive)]
    pub topic: Box<Account<'info, Topic>>,

    /// The temporary alignment token mint, must be mutable for minting
    #[account(
        mut,
        constraint = temp_align_mint.key() == state.temp_align_mint @ ErrorCode::TokenMintMismatch
    )]
    pub temp_align_mint: Account<'info, Mint>,

    /// The signer covering the rent for new PDAs (payer)
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The protocol-owned tempAlign token account for this contributor
    #[account(
        mut,
        seeds = [b"user_temp_align", contributor.key().as_ref()],
        bump,
        constraint = contributor_temp_align_account.mint == temp_align_mint.key() @ ErrorCode::TokenMintMismatch,
        constraint = contributor_temp_align_account.owner == state.key() @ ErrorCode::InvalidTokenAccount
    )]
    pub contributor_temp_align_account: Account<'info, TokenAccount>,

    /// The new Submission account - Seeds now use user key + user counter index
    #[account(
        init,
        payer = payer,
        seeds = [
            b"submission",
            contributor.key().as_ref(),
            current_submission_index.to_le_bytes().as_ref(),
        ],
        bump,
        // Discriminator + contributor pubkey + timestamp + data field + submission PDA bump
        space = 8 + 32 + 8 + (4 + MAX_DATA_REFERENCE_LENGTH) + 1
    )]
    pub submission: Account<'info, Submission>,

    /// The link between submission and topic - Seeds use the derived submission key
    #[account(
        init,
        payer = payer,
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

    /// The contributor's user profile (must exist)
    #[account(
        mut, // Keep mut for incrementing user_submission_count
        seeds = [b"user_profile", contributor.key().as_ref()],
        bump = contributor_profile.bump,
        constraint = contributor_profile.user == contributor.key() @ ErrorCode::UserAccountMismatch
    )]
    pub contributor_profile: Box<Account<'info, UserProfile>>,

    /// The UserTopicBalance account for this contributor and topic.
    /// MUST be initialized separately via `initialize_user_topic_balance` first.
    #[account(
        mut,
        seeds = [b"user_topic_balance", contributor.key().as_ref(), topic.key().as_ref()],
        bump = user_topic_balance.bump,
        constraint = user_topic_balance.user == contributor.key() @ ErrorCode::UserAccountMismatch,
        constraint = user_topic_balance.topic == topic.key() @ ErrorCode::InvalidTopic
    )]
    pub user_topic_balance: Account<'info, UserTopicBalance>,

    /// The user whose submission this is (does not need to sign; the payer covers fees)
    pub contributor: SystemAccount<'info>,

    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

/// Account constraints for linking an existing submission to a topic
#[derive(Accounts)]
pub struct LinkSubmissionToTopic<'info> {
    /// Global protocol state PDA. Mutable so we can increment `topic_count`.
    /// Anyone may invoke `create_topic`, therefore we only enforce the PDA
    /// derivation ("state") and do **not** restrict by authority.
    #[account(mut, seeds = [b"state"], bump)]
    pub state: Account<'info, State>,

    #[account(mut, constraint = topic.is_active)]
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
    #[account(seeds = [b"state"], bump)]
    pub state: Account<'info, State>,

    #[account(
        mut,
        seeds = [
            b"submission_topic_link",
            submission.key().as_ref(),
            topic.key().as_ref(),
        ],
        bump = submission_topic_link.bump,
        constraint = submission_topic_link.status == SubmissionStatus::Pending
    )]
    pub submission_topic_link: Account<'info, SubmissionTopicLink>,

    pub topic: Account<'info, Topic>,

    pub submission: Account<'info, Submission>,

    #[account(
        init,
        payer = payer,
        seeds = [
            b"vote_commit",
            submission_topic_link.key().as_ref(),
            validator.key().as_ref(),
        ],
        bump,
        // Discriminator + submission_topic_link pubkey + validator pubkey + vote_hash + revealed + finalized + 
        // vote_choice (option) + commit_timestamp + temp_rep_amount + perm_rep_amount + bump
        space = 8 + 32 + 32 + 32 + 1 + 1 + (1 + 1) + 8 + 8 + 8 + 1
    )]
    pub vote_commit: Account<'info, VoteCommit>,

    /// Validator's profile (needed for constraints and rep_ata check)
    #[account(
        seeds = [b"user_profile", validator.key().as_ref()],
        bump,
        constraint = user_profile.user == validator.key() @ ErrorCode::UserAccountMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,

    /// Validator's topic-specific balance account for this topic.
    /// MUST be initialized first. Only used if is_permanent_rep is false.
    #[account(
        mut,
        seeds = [b"user_topic_balance", validator.key().as_ref(), topic.key().as_ref()],
        bump = user_topic_balance.bump,
        constraint = user_topic_balance.user == validator.key() @ ErrorCode::UserAccountMismatch,
        constraint = user_topic_balance.topic == topic.key() @ ErrorCode::InvalidTopic
    )]
    pub user_topic_balance: Account<'info, UserTopicBalance>,

    /// Validator's permanent Rep ATA (user-owned).
    /// Only needed if is_permanent_rep is true. We pass it regardless for simplicity,
    /// but only read its balance conditionally in the instruction.
    #[account(
        // No mut needed, just reading balance
        constraint = validator_rep_ata.mint == state.rep_mint @ ErrorCode::TokenMintMismatch,
        constraint = validator_rep_ata.owner == validator.key() @ ErrorCode::InvalidTokenAccount,
        // Ensure ATA corresponds to the profile's stored ATA key
        constraint = validator_rep_ata.key() == user_profile.user_rep_ata @ ErrorCode::InvalidTokenAccount
    )]
    pub validator_rep_ata: Account<'info, TokenAccount>,

    /// The account committing the vote (does not pay fees)
    pub validator: SystemAccount<'info>,

    /// The payer covering transaction fees and rent. Signs the transaction.
    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Account constraints for revealing a previously committed vote
#[derive(Accounts)]
pub struct RevealVote<'info> {
    #[account(seeds = [b"state"], bump)]
    pub state: Account<'info, State>,

    #[account(
        mut,
        seeds = [
            b"submission_topic_link",
            submission.key().as_ref(),
            topic.key().as_ref(),
        ],
        bump = submission_topic_link.bump,
        constraint = submission_topic_link.status == SubmissionStatus::Pending
    )]
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
        constraint = !vote_commit.revealed,
    )]
    pub vote_commit: Account<'info, VoteCommit>,

    /// The original voter (readonly, no signature required)
    pub validator: SystemAccount<'info>,

    /// The payer covering transaction fees (signer)
    #[account(mut)]
    pub payer: Signer<'info>,

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
        constraint = submission_topic_link.status == SubmissionStatus::Pending @ ErrorCode::SubmissionNotPending,
        constraint = Clock::get()?.unix_timestamp as u64 > submission_topic_link.reveal_phase_end @ ErrorCode::RevealPhaseNotEnded
    )]
    pub submission_topic_link: Account<'info, SubmissionTopicLink>,

    #[account(
        constraint = topic.key() == submission_topic_link.topic @ ErrorCode::InvalidTopic
    )]
    pub topic: Account<'info, Topic>,

    #[account(
        constraint = submission.key() == submission_topic_link.submission @ ErrorCode::InvalidSubmission
    )]
    pub submission: Account<'info, Submission>,

    /// The contributor's user profile (needed for token account constraints)
    #[account(
        seeds = [b"user_profile", submission.contributor.as_ref()],
        bump = contributor_profile.bump,
        constraint = contributor_profile.user == submission.contributor @ ErrorCode::UserAccountMismatch
    )]
    pub contributor_profile: Account<'info, UserProfile>,

    /// The contributor's topic-specific balance account for this topic.
    /// This holds the tempAlign to potentially convert.
    #[account(
        mut,
        seeds = [b"user_topic_balance", submission.contributor.as_ref(), topic.key().as_ref()],
        bump = user_topic_balance.bump,
        constraint = user_topic_balance.user == submission.contributor @ ErrorCode::UserAccountMismatch,
        constraint = user_topic_balance.topic == topic.key() @ ErrorCode::InvalidTopic
    )]
    pub user_topic_balance: Account<'info, UserTopicBalance>,

    /// The protocol-owned tempAlign token account for the contributor (for burning)
    #[account(
        mut,
        seeds = [b"user_temp_align", submission.contributor.as_ref()],
        bump,
        constraint = contributor_temp_align_account.mint == state.temp_align_mint @ ErrorCode::TokenMintMismatch,
        constraint = contributor_temp_align_account.owner == state.key() @ ErrorCode::InvalidTokenAccount,
        constraint = contributor_temp_align_account.key() == contributor_profile.user_temp_align_account @ ErrorCode::InvalidTokenAccount
    )]
    pub contributor_temp_align_account: Account<'info, TokenAccount>,

    /// The contributor's ATA for permanent alignment tokens (regular user-owned ATA)
    #[account(
        mut,
        constraint = contributor_align_ata.mint == state.align_mint @ ErrorCode::TokenMintMismatch,
        constraint = contributor_align_ata.owner == submission.contributor @ ErrorCode::UserAccountMismatch,
        constraint = contributor_align_ata.key() == contributor_profile.user_align_ata @ ErrorCode::InvalidTokenAccount
    )]
    pub contributor_align_ata: Account<'info, TokenAccount>,

    /// The tempAlign mint (for burning)
    #[account(
        mut,
        constraint = temp_align_mint.key() == state.temp_align_mint @ ErrorCode::TokenMintMismatch
    )]
    pub temp_align_mint: Account<'info, Mint>,

    /// The Align mint (for minting)
    #[account(
        mut,
        constraint = align_mint.key() == state.align_mint @ ErrorCode::TokenMintMismatch
    )]
    pub align_mint: Account<'info, Mint>,

    /// The authority calling this instruction (can be any user, acts as payer)
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
        constraint = vote_commit.revealed,
        constraint = vote_commit.validator == validator_profile.user,
        constraint = vote_commit.submission_topic_link == submission_topic_link.key()
    )]
    pub vote_commit: Account<'info, VoteCommit>,

    /// The validator's user profile (profile whose vote is being finalized)
    #[account(
        mut, // Needs mut to potentially update permanent_rep_amount
        seeds = [b"user_profile", vote_commit.validator.as_ref()], // Use validator key from vote_commit
        bump = validator_profile.bump,
        constraint = validator_profile.user == vote_commit.validator @ ErrorCode::UserAccountMismatch
    )]
    pub validator_profile: Account<'info, UserProfile>,

    /// Validator's topic-specific balance account for this topic.
    /// Used only if is_permanent_rep was false during commit.
    #[account(
        mut, // Needs to be mutable to update locked balance
        seeds = [b"user_topic_balance", validator_profile.user.as_ref(), topic.key().as_ref()], // Use validator key from profile
        bump = user_topic_balance.bump,
        constraint = user_topic_balance.user == validator_profile.user @ ErrorCode::UserAccountMismatch,
        constraint = user_topic_balance.topic == topic.key() @ ErrorCode::InvalidTopic
    )]
    pub user_topic_balance: Account<'info, UserTopicBalance>,

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
/// 3) Sets `topic_count = 0`.
#[derive(Accounts)]
pub struct InitializeState<'info> {
    #[account(
        init,
        seeds = [b"state"],
        bump,
        payer = authority,
        space = 8 + (32 * 6) + 1 + (8 * 4) // Updated space (233 bytes) for 6 pubkeys, 1 bump, 4 u64s
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
    #[account(seeds = [b"state"], bump)]
    pub state: Account<'info, State>,

    /// Signer that covers the rent
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The user for whom we want to create an ATA (read-only, unsigned)
    pub user: SystemAccount<'info>,

    /// The user's profile, needs mut to store the new ATA address
    #[account(
        mut,
        seeds = [b"user_profile", user.key().as_ref()],
        bump = user_profile.bump,
        constraint = user_profile.user == user.key() @ ErrorCode::UserAccountMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,

    /// The mint for which we want the user's ATA (only permanent token mints)
    #[account(mut, constraint =
        *mint.to_account_info().key == state.align_mint ||
        *mint.to_account_info().key == state.rep_mint
            @ ErrorCode::TokenMintMismatch // Added error code for clarity
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

    /// The user for whom we're creating the account (read-only, no signature required)
    pub user: SystemAccount<'info>,

    /// The user's profile, needs mut to store the new token account address
    #[account(
        mut,
        seeds = [b"user_profile", user.key().as_ref()],
        bump = user_profile.bump,
        constraint = user_profile.user == user.key() @ ErrorCode::UserAccountMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,

    /// The mint must be the tempAlign mint
    #[account(mut, constraint =
        *mint.to_account_info().key == state.temp_align_mint
            @ ErrorCode::TokenMintMismatch // Added error code for clarity
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

    /// The user for whom we're creating the account (read-only, unsigned)
    pub user: SystemAccount<'info>,

    /// The user's profile, needs mut to store the new token account address
    #[account(
        mut,
        seeds = [b"user_profile", user.key().as_ref()],
        bump = user_profile.bump,
        constraint = user_profile.user == user.key() @ ErrorCode::UserAccountMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,

    /// The mint must be the tempRep mint
    #[account(mut, constraint =
        *mint.to_account_info().key == state.temp_rep_mint
            @ ErrorCode::TokenMintMismatch // Added error code for clarity
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

/// Account constraints for creating a user profile.
///
/// Previously the profile PDA was paid for and signed by the end-user, which required
/// their signature on every transaction.  This updated version removes that
/// requirement: any signer may act as the `payer`, while the `user` account is now
/// read-only.  This enables fee subsidisation without a wallet pop-up.
#[derive(Accounts)]
pub struct CreateUserProfile<'info> {
    /// The user profile PDA to be initialised.
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 8 + (32 * 4) + 1, // discriminator + UserProfile fields
        seeds = [b"user_profile", user.key().as_ref()],
        bump
    )]
    pub user_profile: Account<'info, UserProfile>,

    /// The user's public key – no signature required.
    pub user: SystemAccount<'info>,

    /// The payer funding account creation
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

/// Account constraints for initializing a user's balance account for a specific topic
#[derive(Accounts)]
pub struct InitializeUserTopicBalance<'info> {
    /// The user for whom the balance account is being created (does not need to sign)
    pub user: SystemAccount<'info>,

    /// The user's profile (needed for constraint check, maybe not mutation)
    #[account(
        seeds = [b"user_profile", user.key().as_ref()],
        bump,
        constraint = user_profile.user == user.key()
    )]
    pub user_profile: Account<'info, UserProfile>,

    /// The topic this balance is associated with
    pub topic: Account<'info, Topic>,

    /// The signer paying for account creation
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The user's topic-specific balance account to be initialized.
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 8 + 8 + 8 + 1, // Space: Discriminator + user + topic + 3*u64 + bump
        seeds = [b"user_topic_balance", user.key().as_ref(), topic.key().as_ref()],
        bump,
    )]
    pub user_topic_balance: Account<'info, UserTopicBalance>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Account constraints for staking temporary alignment tokens for a specific topic
#[derive(Accounts)]
pub struct StakeTopicSpecificTokens<'info> {
    #[account(seeds = [b"state"], bump = state.bump)]
    pub state: Account<'info, State>,

    /// The topic for which tokens are being staked
    pub topic: Account<'info, Topic>,

    /// The user's profile, contains references to token accounts
    #[account(
        // No mut needed if just reading keys
        seeds = [b"user_profile", user.key().as_ref()],
        bump = user_profile.bump,
        // Use the new error code for user mismatch
        constraint = user_profile.user == user.key() @ ErrorCode::UserAccountMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,

    /// The user's topic-specific balance account for this topic.
    /// MUST be initialized separately via `initialize_user_topic_balance` first.
    #[account(
        mut,
        seeds = [b"user_topic_balance", user.key().as_ref(), topic.key().as_ref()],
        bump = user_topic_balance.bump,
        // Use the new error code for user mismatch
        constraint = user_topic_balance.user == user.key() @ ErrorCode::UserAccountMismatch,
        // Use the new error code for topic mismatch
        constraint = user_topic_balance.topic == topic.key() @ ErrorCode::InvalidTopic
    )]
    pub user_topic_balance: Account<'info, UserTopicBalance>,

    /// The temporary alignment token mint (source tokens to burn)
    #[account(
        mut,
        seeds = [b"temp_align_mint"],
        bump,
    )]
    pub temp_align_mint: Account<'info, Mint>,

    /// The temporary reputation token mint (target tokens to mint)
    #[account(
        mut,
        seeds = [b"temp_rep_mint"],
        bump,
    )]
    pub temp_rep_mint: Account<'info, Mint>,

    /// The protocol-owned tempAlign token account for this user (source for burn)
    #[account(
        mut,
        seeds = [b"user_temp_align", user.key().as_ref()],
        bump,
        constraint = user_temp_align_account.key() == user_profile.user_temp_align_account @ ErrorCode::InvalidTokenAccount,
        constraint = user_temp_align_account.mint == temp_align_mint.key() @ ErrorCode::TokenMintMismatch,
        constraint = user_temp_align_account.owner == state.key() @ ErrorCode::InvalidTokenAccount
    )]
    pub user_temp_align_account: Account<'info, TokenAccount>,

    /// The protocol-owned tempRep token account for this user (target for mint)
    #[account(
        mut,
        seeds = [b"user_temp_rep", user.key().as_ref()],
        bump,
        constraint = user_temp_rep_account.key() == user_profile.user_temp_rep_account @ ErrorCode::InvalidTokenAccount,
        constraint = user_temp_rep_account.mint == temp_rep_mint.key() @ ErrorCode::TokenMintMismatch,
        constraint = user_temp_rep_account.owner == state.key() @ ErrorCode::InvalidTokenAccount
    )]
    pub user_temp_rep_account: Account<'info, TokenAccount>,

    /// The user for whom tokens are being staked (no signature required).
    pub user: SystemAccount<'info>,

    /// The signer paying for transaction fees.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Token program for CPI calls
    pub token_program: Program<'info, Token>,
}

// --- NEW CONTEXTS FOR AI VALIDATION ---

/// Account constraints for requesting AI validation for a submission
#[derive(Accounts)]
#[instruction(temp_rep_to_stake: u64, expected_ai_request_index: u64)]
pub struct RequestAiValidation<'info> {
    #[account(mut)]
    pub requester: Signer<'info>,

    /// The submission made by the requester
    #[account()]
    pub submission: Account<'info, Submission>,

    /// The topic the submission belongs to (needed for UserTopicBalance PDA derivation)
    #[account()]
    pub topic: Account<'info, Topic>,

    /// The link between the submission and the topic
    #[account(
        mut,
        seeds = [b"submission_topic_link", submission.key().as_ref(), topic.key().as_ref()],
        bump = submission_topic_link.bump,
    )]
    pub submission_topic_link: Account<'info, SubmissionTopicLink>,

    /// User's balance account for this specific topic (to deduct tempRep)
    #[account(
        mut,
        seeds = [b"user_topic_balance", requester.key().as_ref(), topic.key().as_ref()],
        bump = user_topic_balance.bump,
        constraint = user_topic_balance.user == requester.key() @ ErrorCode::UserAccountMismatch,
        constraint = user_topic_balance.topic == topic.key() @ ErrorCode::InvalidTopic,
    )]
    pub user_topic_balance: Account<'info, UserTopicBalance>,

    /// The new AI Validation Request account to be created
    #[account(
        init,
        payer = requester,
        space = 8 + // Discriminator
                32 + // submission_topic_link: Pubkey
                32 + // requester: Pubkey
                 8 + // temp_rep_staked: u64
                 8 + // request_timestamp: u64
                 1 + // status: AiValidationStatus (enum discriminator)
                 2 + // ai_decision: Option<VoteChoice> (option + enum discriminators)
                 8 + // ai_voting_power: u64
                 8 + // request_index: u64 (the index used for PDA derivation)
                 1 , // bump: u8
                // TOTAL = 108 bytes
        seeds = [
            b"ai_request",
            submission_topic_link.key().as_ref(),
            expected_ai_request_index.to_le_bytes().as_ref()
        ],
        bump
    )]
    pub ai_validation_request: Account<'info, AiValidationRequest>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(ai_request_index: u64)]
pub struct SubmitAiVote<'info> {
    #[account(mut)]
    pub oracle: Signer<'info>, // The off-chain service's keypair

    #[account(
        seeds = [b"state"], bump = state.bump,
        // Constraint checked in instruction logic using oracle_pubkey field
    )]
    pub state: Account<'info, State>, // Needed to verify the oracle's public key

    /// The AI Request being fulfilled.
    #[account(
        mut, // Needs to be mutable to update status
        seeds = [
            b"ai_request",
            submission_topic_link.key().as_ref(),
            ai_request_index.to_le_bytes().as_ref() // Use the passed index
        ],
        bump, // Specify bump for Anchor to derive the PDA address using canonical bump
        // Constraint: Ensure it belongs to the link (checked in instruction logic)
    )]
    pub ai_validation_request: Account<'info, AiValidationRequest>,

    /// The SubmissionTopicLink being voted on.
    #[account(
        mut,
        // Constraint: Ensure link matches request (checked in instruction logic)
    )]
    pub submission_topic_link: Account<'info, SubmissionTopicLink>,
    // Optional: Include Topic if needed for context or validation rules in future
    // #[account(constraint = submission_topic_link.topic == topic.key())]
    // pub topic: Account<'info, Topic>,
}
// --- END OF NEW CONTEXTS ---
