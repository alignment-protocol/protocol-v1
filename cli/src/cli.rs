use clap::{Parser, Subcommand};

/// Define CLI application structure
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
/// Alignment Protocol CLI: Tool to interact with the Alignment Protocol on Solana
pub struct Cli {
    /// Path to the user's keypair file
    #[arg(long, default_value = "~/.config/solana/id.json")]
    pub keypair: String,

    /// Choose the Solana cluster (devnet, mainnet, custom URL, etc.)
    #[arg(long, default_value = "devnet")]
    pub cluster: String,

    /// Program ID for the Alignment Protocol
    #[arg(long, default_value = "BMYn8rtstaZhzFZtgMVMY9io1zhnqacr3yANZrgkv7DF")]
    pub program_id: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Admin operations (protocol configuration)
    Admin {
        #[command(subcommand)]
        subcommand: AdminCommands,
    },

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
}

#[derive(Subcommand)]
pub enum AdminCommands {
    /// Update tokens to mint per submission
    UpdateTokensToMint {
        /// New amount of tokens to mint per submission
        #[arg(index = 1)]
        tokens: u64,
    },
}

#[derive(Subcommand)]
pub enum TopicCommands {
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

    /// List all topics
    List,

    /// View a specific topic
    View {
        /// Topic ID
        #[arg(index = 1)]
        id: u64,
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
}

#[derive(Subcommand)]
pub enum SubmissionCommands {
    /// Submit data to a topic
    Submit {
        /// Topic ID
        #[arg(index = 1)]
        topic_id: u64,

        /// Data reference (IPFS hash, Arweave ID, etc.)
        #[arg(index = 2)]
        data_reference: String,
    },

    /// Link an existing submission to another topic
    Link {
        /// Submission ID
        #[arg(index = 1)]
        submission_id: u64,

        /// Topic ID
        #[arg(index = 2)]
        topic_id: u64,
    },

    /// Finalize a submission after voting
    Finalize {
        /// Submission ID
        #[arg(index = 1)]
        submission_id: u64,

        /// Topic ID
        #[arg(index = 2)]
        topic_id: u64,
    },
}

#[derive(Subcommand)]
pub enum VoteCommands {
    /// Commit a vote (first phase)
    Commit {
        /// Submission ID
        #[arg(index = 1)]
        submission_id: u64,

        /// Topic ID
        #[arg(index = 2)]
        topic_id: u64,

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
        /// Submission ID
        #[arg(index = 1)]
        submission_id: u64,

        /// Topic ID
        #[arg(index = 2)]
        topic_id: u64,

        /// Vote choice (yes/no)
        #[arg(index = 3)]
        choice: String,

        /// Secret nonce used in commit phase
        #[arg(index = 4)]
        nonce: String,
    },

    /// Finalize a vote
    Finalize {
        /// Submission ID
        #[arg(index = 1)]
        submission_id: u64,

        /// Topic ID
        #[arg(index = 2)]
        topic_id: u64,
    },

    /// Set arbitrary timestamps for voting phases (admin function)
    SetPhases {
        /// Submission ID
        #[arg(index = 1)]
        submission_id: u64,

        /// Topic ID
        #[arg(index = 2)]
        topic_id: u64,

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
        /// Topic ID
        #[arg(index = 1)]
        topic_id: u64,

        /// Amount of tokens to stake
        #[arg(index = 2)]
        amount: u64,
    },
}

#[derive(Subcommand)]
pub enum QueryCommands {
    /// Query state account
    State,

    /// Get a specific submission
    Submission {
        /// Submission ID
        #[arg(index = 1)]
        id: u64,
    },

    /// List all submissions
    Submissions {
        /// Filter by contributor public key
        #[arg(long)]
        by: Option<String>,

        /// Filter by topic ID
        #[arg(long)]
        topic: Option<u64>,
    },

    /// Get details about submission in a specific topic
    SubmissionTopic {
        /// Submission ID
        #[arg(index = 1)]
        submission_id: u64,

        /// Topic ID
        #[arg(index = 2)]
        topic_id: u64,
    },

    /// Get information about a vote
    Vote {
        /// Submission ID
        #[arg(index = 1)]
        submission_id: u64,

        /// Topic ID
        #[arg(index = 2)]
        topic_id: u64,

        /// Validator public key (defaults to the CLI payer if not provided)
        #[arg(index = 3)]
        validator: Option<String>,
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
