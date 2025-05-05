use clap::{Parser, Subcommand};

/// Define CLI application structure
#[derive(Parser)]
#[command(
    author,
    version,
    about,
    before_help = "
     ▗▄▖ ▗▖   ▗▄▄▄▖ ▗▄▄▖▗▖  ▗▖▗▖  ▗▖▗▄▄▄▖▗▖  ▗▖▗▄▄▄▖
    ▐▌ ▐▌▐▌     █  ▐▌   ▐▛▚▖▐▌▐▛▚▞▜▌▐▌   ▐▛▚▖▐▌  █
    ▐▛▀▜▌▐▌     █  ▐▌▝▜▌▐▌ ▝▜▌▐▌  ▐▌▐▛▀▀▘▐▌ ▝▜▌  █
    ▐▌ ▐▌▐▙▄▄▖▗▄█▄▖▝▚▄▞▘▐▌  ▐▌▐▌  ▐▌▐▙▄▄▖▐▌  ▐▌  █

    ▗▄▄▖ ▗▄▄▖  ▗▄▖▗▄▄▄▖▗▄▖  ▗▄▄▖ ▗▄▖ ▗▖
    ▐▌ ▐▌▐▌ ▐▌▐▌ ▐▌ █ ▐▌ ▐▌▐▌   ▐▌ ▐▌▐▌
    ▐▛▀▘ ▐▛▀▚▖▐▌ ▐▌ █ ▐▌ ▐▌▐▌   ▐▌ ▐▌▐▌
    ▐▌   ▐▌ ▐▌▝▚▄▞▘ █ ▝▚▄▞▘▝▚▄▄▖▝▚▄▞▘▐▙▄▄▖
",
    long_about = "The Alignment Protocol CLI provides tools to interact with the Alignment Protocol on Solana. \
    It supports user operations like creating profiles, submitting data, voting, and staking tokens, \
    as well as admin functions for protocol management."
)]
/// Alignment Protocol CLI: Tool to interact with the Alignment Protocol on Solana
pub struct Cli {
    /// Path to the user's keypair file
    #[arg(long, default_value = "~/.config/solana/id.json")]
    pub keypair: String,

    /// Choose the Solana cluster (localnet, devnet, testnet, mainnet-beta, or custom URL)
    /// Overrides any saved cluster configuration
    #[arg(long)]
    pub cluster: Option<String>,

    /// Program ID for the Alignment Protocol
    #[arg(long, default_value = "ArVxFdoxzCsMDb1K3jXsQTrDP4mbfHMxKiZLjZpznB5c")]
    pub program_id: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Topic-related commands
    Topic {
        #[command(subcommand)]
        subcommand: TopicCommands,
    },

    /// User profile and token account setup
    User {
        #[command(subcommand)]
        subcommand: UserCommands,
    },

    /// Submission-related commands
    Submission {
        #[command(subcommand)]
        subcommand: SubmissionCommands,
    },

    /// Voting-related commands
    Vote {
        #[command(subcommand)]
        subcommand: VoteCommands,
    },

    /// Token and staking operations
    Token {
        #[command(subcommand)]
        subcommand: TokenCommands,
    },

    /// Query and explore protocol data
    Query {
        #[command(subcommand)]
        subcommand: QueryCommands,
    },

    /// Debug operations
    Debug {
        #[command(subcommand)]
        subcommand: DebugCommands,
    },

    /// [ADMIN] Protocol initialization functions
    Init {
        #[command(subcommand)]
        subcommand: InitCommands,
    },

    /// [ADMIN] Protocol configuration
    Config {
        #[command(subcommand)]
        subcommand: ConfigCommands,
    },
}

#[derive(Subcommand)]
pub enum TopicCommands {
    /// List all topics
    List,

    /// View a specific topic
    View {
        /// Topic index (sequential number)
        #[arg(index = 1, value_name = "TOPIC_INDEX")]
        topic_index: u64,
    },

    /// Create a new topic
    Create {
        /// Topic name
        #[arg(index = 1)]
        name: String,

        /// Topic description
        #[arg(index = 2)]
        description: String,

        /// Commit phase duration in seconds (optional)
        #[arg(long)]
        commit_duration: Option<u64>,

        /// Reveal phase duration in seconds (optional)
        #[arg(long)]
        reveal_duration: Option<u64>,
    },

    /// Update an existing topic's settings
    Update {
        /// Topic index
        #[arg(index = 1, value_name = "TOPIC_INDEX")]
        topic_index: u64,

        /// Optional new commit phase duration (seconds)
        #[arg(long)]
        commit_duration: Option<u64>,

        /// Optional new reveal phase duration (seconds)
        #[arg(long)]
        reveal_duration: Option<u64>,

        /// Activate or deactivate the topic
        #[arg(long)]
        active: Option<bool>,
    },
}

#[derive(Subcommand)]
pub enum UserCommands {
    /// Create a user profile with all necessary token accounts
    CreateProfile,

    /// View user profile information
    Profile {
        /// User public key (defaults to the CLI payer if not provided)
        #[arg(index = 1)]
        user: Option<String>,
    },

    /// Initialize the UserTopicBalance account for a user and topic
    InitializeTopicBalance {
        /// Topic index to initialize balance for
        #[arg(index = 1, value_name = "TOPIC_INDEX")]
        topic_index: u64,
    },
}

#[derive(Subcommand)]
pub enum SubmissionCommands {
    /// Submit data to a topic
    Submit {
        /// Topic index
        #[arg(index = 1, value_name = "TOPIC_INDEX")]
        topic_index: u64,

        /// Data reference (IPFS hash, Arweave ID, etc.)
        #[arg(index = 2)]
        data_reference: String,
    },

    /// Link an existing submission to another topic
    Link {
        /// Submission PDA (Pubkey as String)
        #[arg(index = 1)]
        submission_pda: String,

        /// Topic index
        #[arg(index = 2, value_name = "TOPIC_INDEX")]
        topic_index: u64,
    },

    /// Finalize a submission after voting
    Finalize {
        /// Submission PDA (Pubkey as String)
        #[arg(index = 1)]
        submission_pda: String,

        /// Topic index
        #[arg(index = 2, value_name = "TOPIC_INDEX")]
        topic_index: u64,
    },

    /// Request AI validation for your submission (costs tempRep)
    RequestAiValidation {
        /// Submission PDA (Pubkey as String)
        #[arg(long)]
        submission_pda: String,

        /// Topic index (mandatory positional)
        #[arg(index = 1, value_name = "TOPIC_INDEX")]
        topic_index: u64,

        /// Amount of tempRep to stake/spend for the validation
        #[arg(long)]
        amount: u64,
    },
}

#[derive(Subcommand)]
pub enum VoteCommands {
    /// Commit a vote (first phase)
    Commit {
        /// Submission PDA (Pubkey as String)
        #[arg(index = 1)]
        submission_pda: String,

        /// Topic index
        #[arg(index = 2, value_name = "TOPIC_INDEX")]
        topic_index: u64,

        /// Vote choice (yes/no)
        #[arg(index = 3)]
        choice: String,

        /// Amount of tokens to vote with
        #[arg(index = 4)]
        amount: u64,

        /// Secret nonce for commitment
        #[arg(index = 5)]
        nonce: String,

        /// Use permanent reputation tokens (default: false)
        #[arg(long)]
        permanent: bool,
    },

    /// Reveal a vote (second phase)
    Reveal {
        /// Submission PDA (Pubkey as String)
        #[arg(index = 1)]
        submission_pda: String,

        /// Topic index
        #[arg(index = 2, value_name = "TOPIC_INDEX")]
        topic_index: u64,

        /// Vote choice (yes/no)
        #[arg(index = 3)]
        choice: String,

        /// Secret nonce used in commit phase
        #[arg(index = 4)]
        nonce: String,
    },

    /// Finalize a vote
    Finalize {
        /// Submission PDA (Pubkey as String)
        #[arg(index = 1)]
        submission_pda: String,

        /// Topic index
        #[arg(index = 2, value_name = "TOPIC_INDEX")]
        topic_index: u64,
    },

    /// [ADMIN] Set arbitrary timestamps for voting phases
    SetPhases {
        /// Submission PDA (Pubkey as String)
        #[arg(index = 1)]
        submission_pda: String,

        /// Topic index
        #[arg(index = 2, value_name = "TOPIC_INDEX")]
        topic_index: u64,

        /// Commit phase start (Unix timestamp)
        #[arg(long)]
        commit_start: Option<u64>,

        /// Commit phase end (Unix timestamp)
        #[arg(long)]
        commit_end: Option<u64>,

        /// Reveal phase start (Unix timestamp)
        #[arg(long)]
        reveal_start: Option<u64>,

        /// Reveal phase end (Unix timestamp)
        #[arg(long)]
        reveal_end: Option<u64>,
    },
}

#[derive(Subcommand)]
pub enum TokenCommands {
    /// Stake temporary alignment tokens for a topic to earn reputation
    Stake {
        /// Topic index
        #[arg(index = 1, value_name = "TOPIC_INDEX")]
        topic_index: u64,

        /// Amount of tokens to stake
        #[arg(index = 2)]
        amount: u64,
    },

    /// [ADMIN] Mint tokens to a user
    Mint {
        /// Token type (temp-align, align, temp-rep, rep)
        #[arg(index = 1)]
        token_type: String,

        /// Receiver's public key
        #[arg(index = 2)]
        to: String,

        /// Amount to mint
        #[arg(index = 3)]
        amount: u64,
    },
}

#[derive(Subcommand)]
pub enum QueryCommands {
    /// Query state account
    State,

    /// Get a specific submission
    Submission {
        /// Submission PDA (Pubkey as String)
        #[arg(index = 1)]
        pda: String,
    },

    /// List submissions by a specific contributor (optionally filtered by topic)
    Submissions {
        /// Contributor public key (Mandatory)
        #[arg(long)]
        by: String,

        /// Filter by topic index (Optional)
        #[arg(long, value_name = "TOPIC_INDEX")]
        topic: Option<u64>,
    },

    /// Get details about submission in a specific topic
    SubmissionTopic {
        /// Submission PDA (Pubkey as String)
        #[arg(index = 1)]
        submission_pda: String,

        /// Topic index
        #[arg(index = 2, value_name = "TOPIC_INDEX")]
        topic_index: u64,
    },

    /// Get information about a vote
    Vote {
        /// Submission PDA (Pubkey as String)
        #[arg(index = 1)]
        submission_pda: String,

        /// Topic index
        #[arg(index = 2, value_name = "TOPIC_INDEX")]
        topic_index: u64,

        /// Validator public key (defaults to the CLI payer if not provided)
        #[arg(index = 3)]
        validator: Option<String>,
    },

    /// Get user balance for a specific topic
    TopicBalance {
        /// Topic index (creation index)
        #[arg(index = 1, value_name = "TOPIC_INDEX")]
        topic_index: u64,

        /// User public key (defaults to the CLI payer if not provided)
        #[arg(index = 2)]
        user: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum DebugCommands {
    /// Debug token account status
    TokenAccount {
        /// Token type (temp-align, align, temp-rep, rep)
        #[arg(index = 1)]
        token_type: String,

        /// User public key (defaults to the CLI payer if not provided)
        #[arg(index = 2)]
        user: Option<String>,
    },

    /// Get transaction logs for debugging
    Tx {
        /// Transaction signature
        #[arg(index = 1)]
        signature: String,
    },
}

#[derive(Subcommand)]
pub enum InitCommands {
    /// [ADMIN] Initialize protocol state account
    State {
        /// Public key of the authorized AI Oracle service
        #[arg(long, required = true)]
        oracle_pubkey: String,
    },

    /// [ADMIN] Initialize temporary alignment token mint
    TempAlignMint,

    /// [ADMIN] Initialize permanent alignment token mint
    AlignMint,

    /// [ADMIN] Initialize temporary reputation token mint
    TempRepMint,

    /// [ADMIN] Initialize permanent reputation token mint
    RepMint,

    /// [ADMIN] Initialize all accounts (state and all token mints)
    All {
        /// Public key of the authorized AI Oracle service
        #[arg(long, required = true)]
        oracle_pubkey: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// [ADMIN] Update tokens to mint per submission
    UpdateTokensToMint {
        /// New amount of tokens to mint per submission
        #[arg(index = 1)]
        tokens: u64,
    },

    /// [ADMIN] Set and save the Solana cluster configuration
    SetCluster {
        /// Cluster name or URL (localnet, devnet, testnet, mainnet-beta, or custom URL)
        #[arg(index = 1)]
        cluster: Option<String>,
    },

    /// [ADMIN] Get current Solana cluster configuration
    GetCluster,
}
