use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        hash::Hash,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair},
        system_program,
        sysvar::{clock::ID as CLOCK_ID, rent::ID as RENT_ID},
    },
    Client, Cluster, Program,
};
use anchor_spl::associated_token::get_associated_token_address;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::rc::Rc;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Digest, Sha256};

// Import Alignment Protocol accounts and instructions
use alignment_protocol::{
    State as StateAccount, 
    Topic as TopicAccount,
    Submission as SubmissionAccount,
    SubmissionTopicLink as SubmissionTopicLinkAccount,
    UserProfile as UserProfileAccount,
    VoteCommit as VoteCommitAccount,
    VoteChoice,
    accounts as AccountsAll,
    instruction as InstructionAll,
};

// Define CLI application structure
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
/// Alignment Protocol CLI: Tool to interact with the Alignment Protocol on Solana
struct Cli {
    /// Path to the user's keypair file
    #[arg(long, default_value = "~/.config/solana/id.json")]
    keypair: String,

    /// Choose the Solana cluster (devnet, mainnet, custom URL, etc.)
    #[arg(long, default_value = "devnet")]
    cluster: String,

    /// Program ID for the Alignment Protocol
    #[arg(long, default_value = "BMYn8rtstaZhzFZtgMVMY9io1zhnqacr3yANZrgkv7DF")]
    program_id: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize protocol components
    Init {
        #[command(subcommand)]
        subcommand: InitCommands,
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
enum InitCommands {
    /// Initialize the protocol state
    State,

    /// Initialize temporary alignment token mint
    TempAlignMint,

    /// Initialize permanent alignment token mint
    AlignMint,

    /// Initialize temporary reputation token mint
    TempRepMint,

    /// Initialize permanent reputation token mint
    RepMint,

    /// Update tokens to mint per submission
    UpdateTokensToMint {
        /// New amount of tokens to mint per submission
        #[arg(index = 1)]
        tokens: u64,
    },
}

#[derive(Subcommand)]
enum TopicCommands {
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
enum UserCommands {
    /// Create a user profile
    CreateProfile,

    /// Create associated token account for a specific token type
    CreateAta {
        /// Token type (temp-align, align, temp-rep, rep)
        #[arg(index = 1)]
        token_type: String,
    },

    /// Create temporary token account (protocol-owned)
    CreateTempAccount {
        /// Token type (temp-align, temp-rep)
        #[arg(index = 1)]
        token_type: String,
    },

    /// View user profile information
    Profile {
        /// User public key (defaults to the CLI payer if not provided)
        #[arg(index = 1)]
        user: Option<String>,
    },
}

#[derive(Subcommand)]
enum SubmissionCommands {
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
enum VoteCommands {
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
enum TokenCommands {
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
enum QueryCommands {
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
enum DebugCommands {
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load keypair and connect to cluster
    let keypair_path = shellexpand::tilde(&cli.keypair).to_string();
    let payer = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair file: {}", e))?;

    let cluster = match cli.cluster.as_str() {
        "devnet" => Cluster::Devnet,
        "mainnet" => Cluster::Mainnet,
        url => Cluster::Custom(url.to_string(), url.to_string()),
    };

    let client = Client::new_with_options(cluster, Rc::new(payer), CommitmentConfig::confirmed());
    let program_id = Pubkey::from_str(&cli.program_id)?;
    let program = client.program(program_id).expect("Failed to load program");

    // Handle commands
    match cli.command {
        Commands::Init { subcommand } => match subcommand {
            InitCommands::State => cmd_init_state(&program)?,
            InitCommands::TempAlignMint => cmd_init_temp_align_mint(&program)?,
            InitCommands::AlignMint => cmd_init_align_mint(&program)?,
            InitCommands::TempRepMint => cmd_init_temp_rep_mint(&program)?,
            InitCommands::RepMint => cmd_init_rep_mint(&program)?,
            InitCommands::UpdateTokensToMint { tokens } => {
                cmd_update_tokens_to_mint(&program, tokens)?
            }
        },
        Commands::Topic { subcommand } => match subcommand {
            TopicCommands::Create {
                name,
                description,
                commit_duration,
                reveal_duration,
            } => cmd_create_topic(&program, name, description, commit_duration, reveal_duration)?,
            TopicCommands::List => cmd_list_topics(&program)?,
            TopicCommands::View { id } => cmd_view_topic(&program, id)?,
        },
        Commands::User { subcommand } => match subcommand {
            UserCommands::CreateProfile => cmd_create_user_profile(&program)?,
            UserCommands::CreateAta { token_type } => cmd_create_user_ata(&program, token_type)?,
            UserCommands::CreateTempAccount { token_type } => {
                cmd_create_user_temp_account(&program, token_type)?
            }
            UserCommands::Profile { user } => cmd_view_user_profile(&program, user)?,
        },
        Commands::Submission { subcommand } => match subcommand {
            SubmissionCommands::Submit {
                topic_id,
                data_reference,
            } => cmd_submit_data_to_topic(&program, topic_id, data_reference)?,
            SubmissionCommands::Link {
                submission_id,
                topic_id,
            } => cmd_link_submission_to_topic(&program, submission_id, topic_id)?,
            SubmissionCommands::Finalize {
                submission_id,
                topic_id,
            } => cmd_finalize_submission(&program, submission_id, topic_id)?,
        },
        Commands::Vote { subcommand } => match subcommand {
            VoteCommands::Commit {
                submission_id,
                topic_id,
                choice,
                amount,
                nonce,
                permanent,
            } => cmd_commit_vote(
                &program,
                submission_id,
                topic_id,
                choice,
                amount,
                nonce,
                permanent,
            )?,
            VoteCommands::Reveal {
                submission_id,
                topic_id,
                choice,
                nonce,
            } => cmd_reveal_vote(&program, submission_id, topic_id, choice, nonce)?,
            VoteCommands::Finalize {
                submission_id,
                topic_id,
            } => cmd_finalize_vote(&program, submission_id, topic_id)?,
            VoteCommands::SetPhases {
                submission_id,
                topic_id,
                commit_start,
                commit_end,
                reveal_start,
                reveal_end,
            } => cmd_set_voting_phases(
                &program,
                submission_id,
                topic_id,
                commit_start,
                commit_end,
                reveal_start,
                reveal_end,
            )?,
        },
        Commands::Token { subcommand } => match subcommand {
            TokenCommands::Stake { topic_id, amount } => {
                cmd_stake_topic_specific_tokens(&program, topic_id, amount)?
            }
        },
        Commands::Query { subcommand } => match subcommand {
            QueryCommands::State => cmd_query_state(&program)?,
            QueryCommands::Submission { id } => cmd_query_submission(&program, id)?,
            QueryCommands::Submissions { by, topic } => cmd_query_submissions(&program, by, topic)?,
            QueryCommands::SubmissionTopic {
                submission_id,
                topic_id,
            } => cmd_query_submission_topic(&program, submission_id, topic_id)?,
            QueryCommands::Vote {
                submission_id,
                topic_id,
                validator,
            } => cmd_query_vote(&program, submission_id, topic_id, validator)?,
        },
        Commands::Debug { subcommand } => match subcommand {
            DebugCommands::TokenAccount { token_type, user } => {
                cmd_debug_token_account(&program, token_type, user)?
            }
            DebugCommands::Tx { signature } => cmd_get_tx_logs(&program, signature)?,
        },
    }

    Ok(())
}

// ---------------------------------------------------
// Helper Functions
// ---------------------------------------------------

/// Get the program-derived address (PDA) for the state account
fn get_state_pda(program: &Program<Rc<Keypair>>) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"state"], &program.id())
}

/// Get the PDA for a specific mint
fn get_mint_pda(program: &Program<Rc<Keypair>>, mint_type: &str) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[mint_type.as_bytes()], &program.id())
}

/// Get the PDA for a topic account
fn get_topic_pda(program: &Program<Rc<Keypair>>, topic_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"topic", &topic_id.to_le_bytes()], &program.id())
}

/// Get the PDA for a submission account
fn get_submission_pda(program: &Program<Rc<Keypair>>, submission_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"submission", &submission_id.to_le_bytes()],
        &program.id(),
    )
}

/// Get the PDA for a user profile account
fn get_user_profile_pda(program: &Program<Rc<Keypair>>, user: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"user_profile", user.as_ref()], &program.id())
}

/// Get the PDA for a submission-topic link account
fn get_submission_topic_link_pda(
    program: &Program<Rc<Keypair>>,
    submission: &Pubkey,
    topic: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"submission_topic_link", submission.as_ref(), topic.as_ref()],
        &program.id(),
    )
}

/// Get the PDA for a vote commit account
fn get_vote_commit_pda(
    program: &Program<Rc<Keypair>>,
    submission_topic_link: &Pubkey,
    validator: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"vote_commit",
            submission_topic_link.as_ref(),
            validator.as_ref(),
        ],
        &program.id(),
    )
}

/// Get the PDA for a user's temporary token account
fn get_user_temp_token_account_pda(
    program: &Program<Rc<Keypair>>,
    user: &Pubkey,
    token_type: &str,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[token_type.as_bytes(), user.as_ref()],
        &program.id(),
    )
}

/// Get the current Unix timestamp
fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

/// Generate a vote hash for commitment phase
fn generate_vote_hash(
    validator: &Pubkey,
    submission_topic_link: &Pubkey,
    vote_choice: &VoteChoice,
    nonce: &str,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(validator.to_bytes());
    hasher.update(submission_topic_link.to_bytes());
    hasher.update(match vote_choice {
        VoteChoice::Yes => b"yes",
        VoteChoice::No => b"no",
    });
    hasher.update(nonce.as_bytes());
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result[..]);
    hash
}

/// Parse vote choice from string
fn parse_vote_choice(choice: &str) -> Result<VoteChoice> {
    match choice.to_lowercase().as_str() {
        "yes" | "y" | "true" | "1" => Ok(VoteChoice::Yes),
        "no" | "n" | "false" | "0" => Ok(VoteChoice::No),
        _ => Err(anyhow::anyhow!("Invalid vote choice. Use 'yes' or 'no'")),
    }
}

// ---------------------------------------------------
// Initialization Commands
// ---------------------------------------------------

/// Initialize the protocol state account
fn cmd_init_state(program: &Program<Rc<Keypair>>) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    
    println!("Initializing protocol state at {}", state_pda);
    
    let accounts = AccountsAll::InitializeState {
        authority: program.payer(),
        state: state_pda,
        system_program: system_program::ID,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeState {})
        .send()?;
    
    println!("State initialized successfully (txSig: {})", tx_sig);
    Ok(())
}

/// Initialize the temporary alignment token mint
fn cmd_init_temp_align_mint(program: &Program<Rc<Keypair>>) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let (temp_align_mint_pda, _) = get_mint_pda(program, "temp_align_mint");
    
    println!("Initializing temporary alignment token mint at {}", temp_align_mint_pda);
    
    let accounts = AccountsAll::InitializeTempAlignMint {
        state: state_pda,
        temp_align_mint: temp_align_mint_pda,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeTempAlignMint {})
        .send()?;
    
    println!("Temporary alignment token mint initialized successfully (txSig: {})", tx_sig);
    Ok(())
}

/// Initialize the permanent alignment token mint
fn cmd_init_align_mint(program: &Program<Rc<Keypair>>) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let (align_mint_pda, _) = get_mint_pda(program, "align_mint");
    
    println!("Initializing permanent alignment token mint at {}", align_mint_pda);
    
    let accounts = AccountsAll::InitializeAlignMint {
        state: state_pda,
        align_mint: align_mint_pda,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeAlignMint {})
        .send()?;
    
    println!("Permanent alignment token mint initialized successfully (txSig: {})", tx_sig);
    Ok(())
}

/// Initialize the temporary reputation token mint
fn cmd_init_temp_rep_mint(program: &Program<Rc<Keypair>>) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let (temp_rep_mint_pda, _) = get_mint_pda(program, "temp_rep_mint");
    
    println!("Initializing temporary reputation token mint at {}", temp_rep_mint_pda);
    
    let accounts = AccountsAll::InitializeTempRepMint {
        state: state_pda,
        temp_rep_mint: temp_rep_mint_pda,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeTempRepMint {})
        .send()?;
    
    println!("Temporary reputation token mint initialized successfully (txSig: {})", tx_sig);
    Ok(())
}

/// Initialize the permanent reputation token mint
fn cmd_init_rep_mint(program: &Program<Rc<Keypair>>) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let (rep_mint_pda, _) = get_mint_pda(program, "rep_mint");
    
    println!("Initializing permanent reputation token mint at {}", rep_mint_pda);
    
    let accounts = AccountsAll::InitializeRepMint {
        state: state_pda,
        rep_mint: rep_mint_pda,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeRepMint {})
        .send()?;
    
    println!("Permanent reputation token mint initialized successfully (txSig: {})", tx_sig);
    Ok(())
}

/// Update the number of tokens to mint per submission
fn cmd_update_tokens_to_mint(program: &Program<Rc<Keypair>>, tokens: u64) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    
    println!("Updating tokens to mint to {}", tokens);
    
    let accounts = AccountsAll::UpdateTokensToMint {
        authority: program.payer(),
        state: state_pda,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::UpdateTokensToMint { 
            new_tokens_to_mint: tokens 
        })
        .send()?;
    
    println!("Tokens to mint updated successfully (txSig: {})", tx_sig);
    Ok(())
}

// ---------------------------------------------------
// Topic Commands
// ---------------------------------------------------

/// Create a new topic
fn cmd_create_topic(
    program: &Program<Rc<Keypair>>,
    name: String,
    description: String,
    commit_duration: Option<u64>,
    reveal_duration: Option<u64>,
) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    
    // Get the state to determine the next topic ID
    let state_data: StateAccount = program.account(state_pda)?;
    let topic_id = state_data.topic_count;
    
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    
    println!("Creating new topic with ID {}", topic_id);
    println!("Name: {}", name);
    println!("Description: {}", description);
    
    let accounts = AccountsAll::CreateTopic {
        authority: program.payer(),
        state: state_pda,
        topic: topic_pda,
        system_program: system_program::ID,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::CreateTopic { 
            name, 
            description,
            commit_phase_duration: commit_duration,
            reveal_phase_duration: reveal_duration,
        })
        .send()?;
    
    println!("Topic created successfully (txSig: {})", tx_sig);
    println!("Topic ID: {}", topic_id);
    println!("Topic PDA: {}", topic_pda);
    Ok(())
}

/// List all topics
fn cmd_list_topics(program: &Program<Rc<Keypair>>) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    
    // Get the state to determine the number of topics
    let state_data: StateAccount = program.account(state_pda)?;
    let topic_count = state_data.topic_count;
    
    if topic_count == 0 {
        println!("No topics found");
        return Ok(());
    }
    
    println!("Found {} topics:", topic_count);
    
    for i in 0..topic_count {
        let (topic_pda, _) = get_topic_pda(program, i);
        
        match program.account::<TopicAccount>(topic_pda) {
            Ok(topic) => {
                println!("\nTopic #{}", i);
                println!("Name: {}", topic.name);
                println!("Description: {}", topic.description);
                println!("Active: {}", topic.is_active);
                println!("Submissions: {}", topic.submission_count);
            },
            Err(e) => {
                println!("Error fetching topic #{}: {}", i, e);
            }
        }
    }
    
    Ok(())
}

/// View a specific topic
fn cmd_view_topic(program: &Program<Rc<Keypair>>, id: u64) -> Result<()> {
    let (topic_pda, _) = get_topic_pda(program, id);
    
    match program.account::<TopicAccount>(topic_pda) {
        Ok(topic) => {
            println!("Topic #{} ({})", id, topic_pda);
            println!("Name: {}", topic.name);
            println!("Description: {}", topic.description);
            println!("Authority: {}", topic.authority);
            println!("Active: {}", topic.is_active);
            println!("Submissions: {}", topic.submission_count);
            println!("Commit phase duration: {} seconds", topic.commit_phase_duration);
            println!("Reveal phase duration: {} seconds", topic.reveal_phase_duration);
            Ok(())
        },
        Err(e) => {
            Err(anyhow::anyhow!("Topic not found: {}", e))
        }
    }
}

// ---------------------------------------------------
// User Commands
// ---------------------------------------------------

/// Create a user profile
fn cmd_create_user_profile(program: &Program<Rc<Keypair>>) -> Result<()> {
    let user = program.payer();
    let (user_profile_pda, _) = get_user_profile_pda(program, &user);
    
    println!("Creating user profile for {}", user);
    
    let accounts = AccountsAll::CreateUserProfile {
        user,
        user_profile: user_profile_pda,
        system_program: system_program::ID,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::CreateUserProfile {})
        .send()?;
    
    println!("User profile created successfully (txSig: {})", tx_sig);
    println!("User profile PDA: {}", user_profile_pda);
    Ok(())
}

/// Create associated token account for a user
fn cmd_create_user_ata(program: &Program<Rc<Keypair>>, token_type: String) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let user = program.payer();
    
    // Get the mint address based on token type
    let state_data: StateAccount = program.account(state_pda)?;
    let mint = match token_type.to_lowercase().as_str() {
        "temp-align" => state_data.temp_align_mint,
        "align" => state_data.align_mint,
        "temp-rep" => state_data.temp_rep_mint,
        "rep" => state_data.rep_mint,
        _ => return Err(anyhow::anyhow!("Invalid token type. Use temp-align, align, temp-rep, or rep")),
    };
    
    let ata = get_associated_token_address(&user, &mint);
    
    println!("Creating {} associated token account for {}", token_type, user);
    println!("Mint: {}", mint);
    println!("ATA: {}", ata);
    
    let accounts = AccountsAll::CreateUserAta {
        payer: user,
        user,
        mint,
        user_ata: ata,
        system_program: system_program::ID,
        token_program: anchor_spl::token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
        rent: RENT_ID,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::CreateUserAta {})
        .send()?;
    
    println!("Associated token account created successfully (txSig: {})", tx_sig);
    Ok(())
}

/// Create temporary token account (protocol-owned)
fn cmd_create_user_temp_account(program: &Program<Rc<Keypair>>, token_type: String) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let user = program.payer();
    
    // Get the mint address based on token type
    let state_data: StateAccount = program.account(state_pda)?;
    
    match token_type.to_lowercase().as_str() {
        "temp-align" => {
            let mint = state_data.temp_align_mint;
            let (temp_account_pda, _) = get_user_temp_token_account_pda(program, &user, "temp_align_account");
            
            println!("Creating temp-align account for {}", user);
            println!("Mint: {}", mint);
            println!("Account PDA: {}", temp_account_pda);
            
            let accounts = AccountsAll::CreateUserTempAlignAccount {
                payer: user,
                user,
                state: state_pda,
                user_temp_align_account: temp_account_pda,
                system_program: system_program::ID,
                token_program: anchor_spl::token::ID,
            };
            
            let tx_sig = program
                .request()
                .accounts(accounts)
                .args(InstructionAll::CreateUserTempAlignAccount {})
                .send()?;
            
            println!("Temporary alignment token account created successfully (txSig: {})", tx_sig);
        },
        "temp-rep" => {
            let mint = state_data.temp_rep_mint;
            let (temp_account_pda, _) = get_user_temp_token_account_pda(program, &user, "temp_rep_account");
            
            println!("Creating temp-rep account for {}", user);
            println!("Mint: {}", mint);
            println!("Account PDA: {}", temp_account_pda);
            
            let accounts = AccountsAll::CreateUserTempRepAccount {
                payer: user,
                user,
                state: state_pda,
                user_temp_rep_account: temp_account_pda,
                system_program: system_program::ID,
                token_program: anchor_spl::token::ID,
            };
            
            let tx_sig = program
                .request()
                .accounts(accounts)
                .args(InstructionAll::CreateUserTempRepAccount {})
                .send()?;
            
            println!("Temporary reputation token account created successfully (txSig: {})", tx_sig);
        },
        _ => return Err(anyhow::anyhow!("Invalid token type. Use temp-align or temp-rep")),
    }
    
    Ok(())
}

/// View user profile information
fn cmd_view_user_profile(program: &Program<Rc<Keypair>>, user_str: Option<String>) -> Result<()> {
    let user = match user_str {
        Some(pubkey_str) => Pubkey::from_str(&pubkey_str)?,
        None => program.payer(),
    };
    
    let (user_profile_pda, _) = get_user_profile_pda(program, &user);
    
    match program.account::<UserProfileAccount>(user_profile_pda) {
        Ok(profile) => {
            println!("User Profile for {}", user);
            println!("Profile PDA: {}", user_profile_pda);
            println!("Permanent reputation: {}", profile.permanent_rep_amount);
            
            if profile.topic_tokens.is_empty() {
                println!("No topic-specific tokens");
            } else {
                println!("\nTopic-specific tokens:");
                for token_pair in profile.topic_tokens {
                    println!("  Topic #{}", token_pair.topic_id);
                    println!("    Temp Align: {}", token_pair.token.temp_align_amount);
                    println!("    Temp Rep: {}", token_pair.token.temp_rep_amount);
                }
            }
            
            Ok(())
        },
        Err(e) => {
            println!("User profile not found: {}", e);
            println!("Create a profile with 'user create-profile'");
            Ok(())
        }
    }
}

// ---------------------------------------------------
// Submission Commands
// ---------------------------------------------------

/// Submit data to a topic
fn cmd_submit_data_to_topic(
    program: &Program<Rc<Keypair>>,
    topic_id: u64,
    data_reference: String,
) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    
    // Get the state to determine the next submission ID
    let state_data: StateAccount = program.account(state_pda)?;
    let submission_id = state_data.submission_count;
    
    let contributor = program.payer();
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (submission_topic_link_pda, _) = get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    
    // Get temp align mint
    let temp_align_mint = state_data.temp_align_mint;
    
    // Get user's temp align account
    let (contributor_temp_align_account_pda, _) = get_user_temp_token_account_pda(
        program, 
        &contributor, 
        "temp_align_account"
    );
    
    // Get user profile PDA
    let (contributor_profile_pda, _) = get_user_profile_pda(program, &contributor);
    
    // Check if temp align account exists
    let temp_align_account_exists = program.rpc().get_account(&contributor_temp_align_account_pda).is_ok();
    
    if !temp_align_account_exists {
        println!("Creating temp align account first...");
        cmd_create_user_temp_account(program, "temp-align".to_string())?;
    }
    
    // Check if user profile exists
    let profile_exists = program.rpc().get_account(&contributor_profile_pda).is_ok();
    
    println!("Submitting data to topic #{}", topic_id);
    println!("Data reference: {}", data_reference);
    
    let accounts = if profile_exists {
        AccountsAll::SubmitDataToTopic {
            contributor,
            topic: topic_pda,
            submission: submission_pda,
            submission_topic_link: submission_topic_link_pda,
            state: state_pda,
            temp_align_mint,
            contributor_temp_align_account: contributor_temp_align_account_pda,
            contributor_profile: Some(contributor_profile_pda),
            token_program: anchor_spl::token::ID,
            system_program: system_program::ID,
        }
    } else {
        AccountsAll::SubmitDataToTopic {
            contributor,
            topic: topic_pda,
            submission: submission_pda,
            submission_topic_link: submission_topic_link_pda,
            state: state_pda,
            temp_align_mint,
            contributor_temp_align_account: contributor_temp_align_account_pda,
            contributor_profile: None,
            token_program: anchor_spl::token::ID,
            system_program: system_program::ID,
        }
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::SubmitDataToTopic { data_reference })
        .send()?;
    
    println!("Data submitted successfully (txSig: {})", tx_sig);
    println!("Submission ID: {}", submission_id);
    println!("Submission PDA: {}", submission_pda);
    println!("Submission-Topic Link PDA: {}", submission_topic_link_pda);
    Ok(())
}

/// Link an existing submission to another topic
fn cmd_link_submission_to_topic(
    program: &Program<Rc<Keypair>>,
    submission_id: u64,
    topic_id: u64,
) -> Result<()> {
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) = get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    
    println!("Linking submission #{} to topic #{}", submission_id, topic_id);
    
    let accounts = AccountsAll::LinkSubmissionToTopic {
        submission: submission_pda,
        topic: topic_pda,
        submission_topic_link: submission_topic_link_pda,
        system_program: system_program::ID,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::LinkSubmissionToTopic {})
        .send()?;
    
    println!("Submission linked successfully (txSig: {})", tx_sig);
    println!("Submission-Topic Link PDA: {}", submission_topic_link_pda);
    Ok(())
}

/// Finalize a submission after voting
fn cmd_finalize_submission(
    program: &Program<Rc<Keypair>>,
    submission_id: u64,
    topic_id: u64,
) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) = get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    
    // Get submission data
    let submission_data: SubmissionAccount = program.account(submission_pda)?;
    let contributor = submission_data.contributor;
    
    // Get user profile PDA
    let (contributor_profile_pda, _) = get_user_profile_pda(program, &contributor);
    
    // Get mint addresses from state
    let state_data: StateAccount = program.account(state_pda)?;
    let temp_align_mint = state_data.temp_align_mint;
    let align_mint = state_data.align_mint;
    
    // Get token accounts
    let (contributor_temp_align_account_pda, _) = get_user_temp_token_account_pda(
        program, 
        &contributor, 
        "temp_align_account"
    );
    let contributor_align_ata = get_associated_token_address(&contributor, &align_mint);
    
    println!("Finalizing submission #{} in topic #{}", submission_id, topic_id);
    
    let accounts = AccountsAll::FinalizeSubmission {
        state: state_pda,
        topic: topic_pda,
        submission: submission_pda,
        submission_topic_link: submission_topic_link_pda,
        contributor,
        contributor_profile: contributor_profile_pda,
        temp_align_mint,
        contributor_temp_align_account: contributor_temp_align_account_pda,
        align_mint,
        contributor_align_ata,
        token_program: anchor_spl::token::ID,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::FinalizeSubmission {})
        .send()?;
    
    println!("Submission finalized successfully (txSig: {})", tx_sig);
    Ok(())
}

// ---------------------------------------------------
// Vote Commands
// ---------------------------------------------------

/// Commit a vote (first phase)
fn cmd_commit_vote(
    program: &Program<Rc<Keypair>>,
    submission_id: u64,
    topic_id: u64,
    choice_str: String,
    amount: u64,
    nonce: String,
    permanent: bool,
) -> Result<()> {
    let validator = program.payer();
    let (user_profile_pda, _) = get_user_profile_pda(program, &validator);
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) = get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    let (vote_commit_pda, _) = get_vote_commit_pda(program, &submission_topic_link_pda, &validator);
    
    // Parse vote choice
    let vote_choice = parse_vote_choice(&choice_str)?;
    
    // Generate vote hash
    let vote_hash = generate_vote_hash(&validator, &submission_topic_link_pda, &vote_choice, &nonce);
    
    println!("Committing {} vote on submission #{} in topic #{}", choice_str, submission_id, topic_id);
    println!("Vote amount: {}", amount);
    println!("Using {} reputation", if permanent { "permanent" } else { "temporary" });
    println!("Nonce: {}", nonce);
    println!("Generated hash: {:?}", vote_hash);
    
    let accounts = AccountsAll::CommitVote {
        validator,
        user_profile: user_profile_pda,
        submission_topic_link: submission_topic_link_pda,
        topic: topic_pda,
        vote_commit: vote_commit_pda,
        system_program: system_program::ID,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::CommitVote { 
            vote_hash,
            vote_amount: amount,
            is_permanent_rep: permanent,
        })
        .send()?;
    
    println!("Vote committed successfully (txSig: {})", tx_sig);
    println!("Vote Commit PDA: {}", vote_commit_pda);
    println!("Save your vote choice and nonce for the reveal phase!");
    Ok(())
}

/// Reveal a vote (second phase)
fn cmd_reveal_vote(
    program: &Program<Rc<Keypair>>,
    submission_id: u64,
    topic_id: u64,
    choice_str: String,
    nonce: String,
) -> Result<()> {
    let validator = program.payer();
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) = get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    let (vote_commit_pda, _) = get_vote_commit_pda(program, &submission_topic_link_pda, &validator);
    
    // Parse vote choice
    let vote_choice = parse_vote_choice(&choice_str)?;
    
    println!("Revealing {} vote on submission #{} in topic #{}", choice_str, submission_id, topic_id);
    println!("Nonce: {}", nonce);
    
    let accounts = AccountsAll::RevealVote {
        validator,
        submission_topic_link: submission_topic_link_pda,
        topic: topic_pda,
        vote_commit: vote_commit_pda,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::RevealVote { 
            vote_choice,
            nonce,
        })
        .send()?;
    
    println!("Vote revealed successfully (txSig: {})", tx_sig);
    Ok(())
}

/// Finalize a vote
fn cmd_finalize_vote(
    program: &Program<Rc<Keypair>>,
    submission_id: u64,
    topic_id: u64,
) -> Result<()> {
    let validator = program.payer();
    let (state_pda, _) = get_state_pda(program);
    let (validator_profile_pda, _) = get_user_profile_pda(program, &validator);
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) = get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    let (vote_commit_pda, _) = get_vote_commit_pda(program, &submission_topic_link_pda, &validator);
    
    // Get mint addresses from state
    let state_data: StateAccount = program.account(state_pda)?;
    let temp_rep_mint = state_data.temp_rep_mint;
    let rep_mint = state_data.rep_mint;
    
    // Get token accounts
    let (validator_temp_rep_account_pda, _) = get_user_temp_token_account_pda(
        program, 
        &validator, 
        "temp_rep_account"
    );
    let validator_rep_ata = get_associated_token_address(&validator, &rep_mint);
    
    println!("Finalizing vote on submission #{} in topic #{}", submission_id, topic_id);
    
    let accounts = AccountsAll::FinalizeVote {
        validator,
        validator_profile: validator_profile_pda,
        topic: topic_pda,
        submission_topic_link: submission_topic_link_pda,
        vote_commit: vote_commit_pda,
        state: state_pda,
        temp_rep_mint,
        validator_temp_rep_account: validator_temp_rep_account_pda,
        rep_mint,
        validator_rep_ata,
        token_program: anchor_spl::token::ID,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::FinalizeVote {})
        .send()?;
    
    println!("Vote finalized successfully (txSig: {})", tx_sig);
    Ok(())
}

/// Set arbitrary timestamps for voting phases (admin function)
fn cmd_set_voting_phases(
    program: &Program<Rc<Keypair>>,
    submission_id: u64,
    topic_id: u64,
    commit_start: Option<u64>,
    commit_end: Option<u64>,
    reveal_start: Option<u64>,
    reveal_end: Option<u64>,
) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) = get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    
    println!("Setting voting phases for submission #{} in topic #{}", submission_id, topic_id);
    if let Some(ts) = commit_start {
        println!("Commit phase start: {}", ts);
    }
    if let Some(ts) = commit_end {
        println!("Commit phase end: {}", ts);
    }
    if let Some(ts) = reveal_start {
        println!("Reveal phase start: {}", ts);
    }
    if let Some(ts) = reveal_end {
        println!("Reveal phase end: {}", ts);
    }
    
    let accounts = AccountsAll::SetVotingPhases {
        authority: program.payer(),
        state: state_pda,
        submission_topic_link: submission_topic_link_pda,
        topic: topic_pda,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::SetVotingPhases { 
            commit_phase_start: commit_start,
            commit_phase_end: commit_end,
            reveal_phase_start: reveal_start,
            reveal_phase_end: reveal_end,
        })
        .send()?;
    
    println!("Voting phases set successfully (txSig: {})", tx_sig);
    Ok(())
}

// ---------------------------------------------------
// Token Commands
// ---------------------------------------------------

/// Stake temporary alignment tokens for a topic to earn reputation
fn cmd_stake_topic_specific_tokens(
    program: &Program<Rc<Keypair>>,
    topic_id: u64,
    amount: u64,
) -> Result<()> {
    let user = program.payer();
    let (user_profile_pda, _) = get_user_profile_pda(program, &user);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (state_pda, _) = get_state_pda(program);
    
    // Get mint addresses from state
    let state_data: StateAccount = program.account(state_pda)?;
    let temp_align_mint = state_data.temp_align_mint;
    let temp_rep_mint = state_data.temp_rep_mint;
    
    // Get token accounts
    let (user_temp_align_account_pda, _) = get_user_temp_token_account_pda(
        program, 
        &user, 
        "temp_align_account"
    );
    let (user_temp_rep_account_pda, _) = get_user_temp_token_account_pda(
        program, 
        &user, 
        "temp_rep_account"
    );
    
    println!("Staking {} temp alignment tokens for topic #{}", amount, topic_id);
    
    let accounts = AccountsAll::StakeTopicSpecificTokens {
        user,
        user_profile: user_profile_pda,
        topic: topic_pda,
        state: state_pda,
        temp_align_mint,
        user_temp_align_account: user_temp_align_account_pda,
        temp_rep_mint,
        user_temp_rep_account: user_temp_rep_account_pda,
        token_program: anchor_spl::token::ID,
    };
    
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::StakeTopicSpecificTokens { amount })
        .send()?;
    
    println!("Tokens staked successfully (txSig: {})", tx_sig);
    Ok(())
}

// ---------------------------------------------------
// Query Commands
// ---------------------------------------------------

/// Query state account
fn cmd_query_state(program: &Program<Rc<Keypair>>) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    
    match program.account::<StateAccount>(state_pda) {
        Ok(state) => {
            println!("State Account ({})", state_pda);
            println!("Authority: {}", state.authority);
            println!("Temp Align Mint: {}", state.temp_align_mint);
            println!("Align Mint: {}", state.align_mint);
            println!("Temp Rep Mint: {}", state.temp_rep_mint);
            println!("Rep Mint: {}", state.rep_mint);
            println!("Submission Count: {}", state.submission_count);
            println!("Topic Count: {}", state.topic_count);
            println!("Tokens to Mint: {}", state.tokens_to_mint);
            println!("Default Commit Phase Duration: {} seconds", state.default_commit_phase_duration);
            println!("Default Reveal Phase Duration: {} seconds", state.default_reveal_phase_duration);
            Ok(())
        },
        Err(e) => {
            Err(anyhow::anyhow!("State account not found: {}. Initialize it first.", e))
        }
    }
}

/// Query a specific submission
fn cmd_query_submission(program: &Program<Rc<Keypair>>, id: u64) -> Result<()> {
    let (submission_pda, _) = get_submission_pda(program, id);
    
    match program.account::<SubmissionAccount>(submission_pda) {
        Ok(submission) => {
            println!("Submission #{} ({})", id, submission_pda);
            println!("Contributor: {}", submission.contributor);
            println!("Timestamp: {}", submission.timestamp);
            println!("Data Reference: {}", submission.data_reference);
            Ok(())
        },
        Err(e) => {
            Err(anyhow::anyhow!("Submission not found: {}", e))
        }
    }
}

/// Query all submissions
fn cmd_query_submissions(
    program: &Program<Rc<Keypair>>,
    by: Option<String>,
    topic: Option<u64>,
) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    
    // Get state data to find total submission count
    let state_data: StateAccount = program.account(state_pda)?;
    let submission_count = state_data.submission_count;
    
    if submission_count == 0 {
        println!("No submissions found.");
        return Ok(());
    }
    
    println!("Total submissions in protocol: {}", submission_count);
    
    // Parse contributor pubkey if provided
    let contributor_filter = match by {
        Some(pubkey_str) => Some(Pubkey::from_str(&pubkey_str)?),
        None => None,
    };
    
    // Get topic PDA if topic filter is provided
    let topic_pda_filter = match topic {
        Some(id) => {
            let (topic_pda, _) = get_topic_pda(program, id);
            Some(topic_pda)
        },
        None => None,
    };
    
    // Print filter info
    if let Some(pubkey) = contributor_filter {
        println!("Filtering for contributor: {}", pubkey);
    }
    
    if let Some(id) = topic {
        println!("Filtering for topic: {}", id);
    }
    
    let mut matched_count = 0;
    
    // Iterate through all submission indices
    for i in 0..submission_count {
        let (submission_pda, _) = get_submission_pda(program, i);
        
        // Fetch the submission account data
        let submission_data: SubmissionAccount = match program.account(submission_pda) {
            Ok(data) => data,
            Err(e) => {
                println!("Warning: Failed to fetch submission #{}: {}", i, e);
                continue;
            }
        };
        
        // If contributor filter is specified, skip non-matching submissions
        if let Some(pubkey) = contributor_filter {
            if submission_data.contributor != pubkey {
                continue;
            }
        }
        
        // If topic filter is specified, check submission-topic links
        if let Some(topic_pda) = topic_pda_filter {
            let (submission_topic_link_pda, _) = get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
            
            // Skip if the submission-topic link doesn't exist
            if program.rpc().get_account(&submission_topic_link_pda).is_err() {
                continue;
            }
        }
        
        matched_count += 1;
        
        println!("\nSubmission #{}", i);
        println!("PDA: {}", submission_pda);
        println!("Contributor: {}", submission_data.contributor);
        println!("Timestamp: {}", submission_data.timestamp);
        println!("Data Reference: {}", submission_data.data_reference);
    }
    
    println!("\nDisplayed {} submissions matching the criteria", matched_count);
    
    Ok(())
}

/// Query details about a submission in a specific topic
fn cmd_query_submission_topic(
    program: &Program<Rc<Keypair>>,
    submission_id: u64,
    topic_id: u64,
) -> Result<()> {
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) = get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    
    match program.account::<SubmissionTopicLinkAccount>(submission_topic_link_pda) {
        Ok(link) => {
            println!("Submission #{} in Topic #{}", submission_id, topic_id);
            println!("Link PDA: {}", submission_topic_link_pda);
            println!("Status: {:?}", link.status);
            println!("\nVoting Phases:");
            println!("Commit Phase: {} to {}", link.commit_phase_start, link.commit_phase_end);
            println!("Reveal Phase: {} to {}", link.reveal_phase_start, link.reveal_phase_end);
            
            let current_time = get_current_timestamp();
            if current_time < link.commit_phase_start {
                println!("Voting not started yet");
            } else if current_time < link.commit_phase_end {
                println!("Currently in commit phase");
            } else if current_time < link.reveal_phase_start {
                println!("Commit phase ended, waiting for reveal phase");
            } else if current_time < link.reveal_phase_end {
                println!("Currently in reveal phase");
            } else {
                println!("Voting ended");
            }
            
            println!("\nVoting Statistics:");
            println!("Yes Voting Power: {}", link.yes_voting_power);
            println!("No Voting Power: {}", link.no_voting_power);
            println!("Total Committed Votes: {}", link.total_committed_votes);
            println!("Total Revealed Votes: {}", link.total_revealed_votes);
            
            Ok(())
        },
        Err(e) => {
            Err(anyhow::anyhow!("Submission-topic link not found: {}", e))
        }
    }
}

/// Query vote information
fn cmd_query_vote(
    program: &Program<Rc<Keypair>>,
    submission_id: u64,
    topic_id: u64,
    validator_str: Option<String>,
) -> Result<()> {
    let validator = match validator_str {
        Some(pubkey_str) => Pubkey::from_str(&pubkey_str)?,
        None => program.payer(),
    };
    
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) = get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    let (vote_commit_pda, _) = get_vote_commit_pda(program, &submission_topic_link_pda, &validator);
    
    match program.account::<VoteCommitAccount>(vote_commit_pda) {
        Ok(vote) => {
            println!("Vote by {} on Submission #{} in Topic #{}", validator, submission_id, topic_id);
            println!("Vote Commit PDA: {}", vote_commit_pda);
            println!("Vote Hash: {:?}", vote.vote_hash);
            println!("Commit Timestamp: {}", vote.commit_timestamp);
            println!("Vote Amount: {}", vote.vote_amount);
            println!("Using {} reputation", if vote.is_permanent_rep { "permanent" } else { "temporary" });
            println!("Revealed: {}", vote.revealed);
            println!("Finalized: {}", vote.finalized);
            
            if vote.revealed {
                println!("Vote Choice: {:?}", vote.vote_choice);
            }
            
            Ok(())
        },
        Err(e) => {
            println!("Vote not found: {}", e);
            println!("The validator may not have voted on this submission.");
            Ok(())
        }
    }
}

// ---------------------------------------------------
// Debug Commands
// ---------------------------------------------------

/// Debug token account status
fn cmd_debug_token_account(
    program: &Program<Rc<Keypair>>,
    token_type: String,
    user_str: Option<String>,
) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    
    // Get the user pubkey
    let user = match user_str {
        Some(pubkey_str) => Pubkey::from_str(&pubkey_str)?,
        None => program.payer(),
    };
    
    // Get state data to get mint addresses
    let state_data: StateAccount = program.account(state_pda)?;
    
    println!("==== Token Account Debug Information ====");
    println!("User: {}", user);
    
    match token_type.to_lowercase().as_str() {
        "temp-align" => {
            let mint = state_data.temp_align_mint;
            let (temp_account_pda, _) = get_user_temp_token_account_pda(program, &user, "temp_align_account");
            
            println!("Token Type: Temporary Alignment (TempAlign)");
            println!("Mint: {}", mint);
            println!("Account PDA: {}", temp_account_pda);
            
            // Check if the account exists
            match program.rpc().get_account(&temp_account_pda) {
                Ok(account) => {
                    println!("✅ Account exists with the following details:");
                    println!("   - Owner: {}", account.owner);
                    println!("   - Lamports: {}", account.lamports);
                    println!("   - Data length: {} bytes", account.data.len());
                    
                    // Try to get token account info
                    match program.account::<anchor_spl::token::TokenAccount>(temp_account_pda) {
                        Ok(token_account) => {
                            println!("   - Token account data:");
                            println!("     * Mint: {}", token_account.mint);
                            println!("     * Owner: {}", token_account.owner);
                            println!("     * Amount: {}", token_account.amount);
                        },
                        Err(e) => {
                            println!("❌ Could not deserialize as token account: {}", e);
                        }
                    }
                },
                Err(e) => {
                    println!("❌ Account does not exist: {}", e);
                    println!("You can create it with: alignment-protocol-cli user create-temp-account temp-align");
                }
            }
        },
        "align" => {
            let mint = state_data.align_mint;
            let ata = get_associated_token_address(&user, &mint);
            
            println!("Token Type: Permanent Alignment (Align)");
            println!("Mint: {}", mint);
            println!("ATA: {}", ata);
            
            // Check if the account exists
            match program.rpc().get_account(&ata) {
                Ok(account) => {
                    println!("✅ Account exists with the following details:");
                    println!("   - Owner: {}", account.owner);
                    println!("   - Lamports: {}", account.lamports);
                    println!("   - Data length: {} bytes", account.data.len());
                    
                    // Try to get token account info
                    match program.account::<anchor_spl::token::TokenAccount>(ata) {
                        Ok(token_account) => {
                            println!("   - Token account data:");
                            println!("     * Mint: {}", token_account.mint);
                            println!("     * Owner: {}", token_account.owner);
                            println!("     * Amount: {}", token_account.amount);
                        },
                        Err(e) => {
                            println!("❌ Could not deserialize as token account: {}", e);
                        }
                    }
                },
                Err(e) => {
                    println!("❌ Account does not exist: {}", e);
                    println!("You can create it with: alignment-protocol-cli user create-ata align");
                }
            }
        },
        "temp-rep" => {
            let mint = state_data.temp_rep_mint;
            let (temp_account_pda, _) = get_user_temp_token_account_pda(program, &user, "temp_rep_account");
            
            println!("Token Type: Temporary Reputation (TempRep)");
            println!("Mint: {}", mint);
            println!("Account PDA: {}", temp_account_pda);
            
            // Check if the account exists
            match program.rpc().get_account(&temp_account_pda) {
                Ok(account) => {
                    println!("✅ Account exists with the following details:");
                    println!("   - Owner: {}", account.owner);
                    println!("   - Lamports: {}", account.lamports);
                    println!("   - Data length: {} bytes", account.data.len());
                    
                    // Try to get token account info
                    match program.account::<anchor_spl::token::TokenAccount>(temp_account_pda) {
                        Ok(token_account) => {
                            println!("   - Token account data:");
                            println!("     * Mint: {}", token_account.mint);
                            println!("     * Owner: {}", token_account.owner);
                            println!("     * Amount: {}", token_account.amount);
                        },
                        Err(e) => {
                            println!("❌ Could not deserialize as token account: {}", e);
                        }
                    }
                },
                Err(e) => {
                    println!("❌ Account does not exist: {}", e);
                    println!("You can create it with: alignment-protocol-cli user create-temp-account temp-rep");
                }
            }
        },
        "rep" => {
            let mint = state_data.rep_mint;
            let ata = get_associated_token_address(&user, &mint);
            
            println!("Token Type: Permanent Reputation (Rep)");
            println!("Mint: {}", mint);
            println!("ATA: {}", ata);
            
            // Check if the account exists
            match program.rpc().get_account(&ata) {
                Ok(account) => {
                    println!("✅ Account exists with the following details:");
                    println!("   - Owner: {}", account.owner);
                    println!("   - Lamports: {}", account.lamports);
                    println!("   - Data length: {} bytes", account.data.len());
                    
                    // Try to get token account info
                    match program.account::<anchor_spl::token::TokenAccount>(ata) {
                        Ok(token_account) => {
                            println!("   - Token account data:");
                            println!("     * Mint: {}", token_account.mint);
                            println!("     * Owner: {}", token_account.owner);
                            println!("     * Amount: {}", token_account.amount);
                        },
                        Err(e) => {
                            println!("❌ Could not deserialize as token account: {}", e);
                        }
                    }
                },
                Err(e) => {
                    println!("❌ Account does not exist: {}", e);
                    println!("You can create it with: alignment-protocol-cli user create-ata rep");
                }
            }
        },
        _ => {
            return Err(anyhow::anyhow!("Invalid token type. Use temp-align, align, temp-rep, or rep"));
        }
    }
    
    println!("\n==== End of Token Account Debug Information ====");
    Ok(())
}

/// Get transaction logs for debugging
fn cmd_get_tx_logs(_program: &Program<Rc<Keypair>>, signature: String) -> Result<()> {
    println!("Fetching logs for transaction: {}", signature);
    
    // Use std::process::Command to call the Solana CLI
    use std::process::Command;
    
    // Build the solana CLI command
    let cmd_str = format!("solana confirm -v {}", signature);
    println!("Running: {}", cmd_str);
    
    // Execute the command
    let output = Command::new("sh").arg("-c").arg(cmd_str).output();
    
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            println!("\n=== Transaction Status ===");
            if output.status.success() {
                println!("Command executed successfully");
            } else {
                println!("Command failed with exit code: {:?}", output.status.code());
            }
            
            if !stdout.is_empty() {
                println!("\n=== Standard Output ===");
                println!("{}", stdout);
            }
            
            if !stderr.is_empty() {
                println!("\n=== Standard Error ===");
                println!("{}", stderr);
            }
        }
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Failed to execute solana CLI command: {}",
                e
            ));
        }
    }
    
    Ok(())
}