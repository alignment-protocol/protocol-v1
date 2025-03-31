use anchor_lang::prelude::*;

// Maximum lengths for strings
pub const MAX_TOPIC_NAME_LENGTH: usize = 64;
pub const MAX_TOPIC_DESCRIPTION_LENGTH: usize = 256;
pub const MAX_DATA_REFERENCE_LENGTH: usize = 128; // For IPFS/Arweave hashes or transaction references

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
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SubmissionStatus {
    /// Submission is pending voting/finalization
    Pending,

    /// Submission has been accepted by voters
    Accepted,

    /// Submission has been rejected by voters
    Rejected,
}

/// User profile account to track reputation and submissions
#[account]
pub struct UserProfile {
    /// The user's wallet public key
    pub user: Pubkey,

    /// A local submission counter for unique seeds
    pub user_submission_count: u64,

    /// References to the user's protocol-owned temporary token accounts
    pub user_temp_align_account: Pubkey,
    pub user_temp_rep_account: Pubkey,

    /// (Optional) References to the user's permanent token Associated Token Accounts (ATAs)
    /// These might need separate instructions to initialize/update if used.
    pub user_align_ata: Pubkey,
    pub user_rep_ata: Pubkey,

    // REMOVE pub permanent_rep_amount: u64,
    /// Bump seed for the user profile PDA
    pub bump: u8,
    // REMOVE pub topic_tokens: Vec<TopicTokenPair>, - Done
}

/// Account to store user's token balances for a specific topic
#[account]
pub struct UserTopicBalance {
    /// The user wallet this balance belongs to
    pub user: Pubkey,

    /// The topic this balance is associated with
    pub topic: Pubkey,

    /// Amount of temporary alignment tokens for this topic
    pub temp_align_amount: u64,

    /// Amount of temporary reputation tokens staked for this topic (available for voting)
    pub temp_rep_amount: u64,

    /// Amount of temporary reputation tokens locked in active votes for this topic
    pub locked_temp_rep_amount: u64,

    /// Bump seed for the PDA
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
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
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
