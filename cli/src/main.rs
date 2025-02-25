use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair},
        system_program,
        sysvar::rent::ID as RENT_ID,
    },
    Client, Cluster, Program,
};
use anchor_spl::associated_token::get_associated_token_address;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::rc::Rc;
use std::str::FromStr;

use alignment_protocol::accounts::CreateUserAta as CreateUserAtaAccounts;
use alignment_protocol::accounts::SubmitData as SubmitDataAccounts;
use alignment_protocol::instruction::CreateUserAta as CreateUserAtaIx;
use alignment_protocol::instruction::SubmitData as SubmitDataIx;
use alignment_protocol::State as StateAccount;
use alignment_protocol::Submission as SubmissionAccount;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
/// alignment-cli: A simple CLI to interact with the alignment-protocol program
struct Cli {
    /// Path to the user's keypair file
    #[arg(long, default_value = "~/.config/solana/id.json")]
    keypair: String,

    /// Choose the Solana cluster (devnet, mainnet, custom URL, etc.)
    #[arg(long, default_value = "devnet")]
    cluster: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Submit data to the protocol
    Submit {
        /// The data string (could be JSON or text)
        #[arg(long)]
        data: String,
    },
    /// Fetch and display a submission account
    GetSubmission {
        /// The submission index (like 0,1,2,...)
        #[arg(long)]
        submission_index: u64,
    },
    /// Fetch and display all submission accounts
    GetAllSubmissions {
        /// Optional contributor public key to filter by
        #[arg(long)]
        contributor: Option<String>,
    },
    /// Debug ATA status for a user
    DebugAta {
        /// Optional user public key (defaults to the CLI payer if not provided)
        #[arg(long)]
        user: Option<String>,
    },
    /// Get detailed transaction logs for debugging
    GetTxLogs {
        /// The transaction signature to fetch logs for
        #[arg(long)]
        signature: String,
    },
}

// Changed to a regular function, no more tokio::main attribute
fn main() -> Result<()> {
    let cli = Cli::parse();

    // 1) Load the keypair
    let keypair_path = shellexpand::tilde(&cli.keypair).to_string();
    let payer = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair file: {}", e))?;

    // 2) Connect to cluster using anchor-client
    let cluster = match cli.cluster.as_str() {
        "devnet" => Cluster::Devnet,
        "mainnet" => Cluster::Mainnet,
        url => Cluster::Custom(url.to_string(), url.to_string()),
    };
    let client = Client::new_with_options(cluster, Rc::new(payer), CommitmentConfig::confirmed());

    // 3) The program ID of your alignment_protocol
    //    Replace with your actual ID (the one you used in declare_id!)
    let program_id = Pubkey::from_str("BMYn8rtstaZhzFZtgMVMY9io1zhnqacr3yANZrgkv7DF")?;
    let program = client.program(program_id).expect("Failed to load program");

    // Execute the appropriate command based on user input
    match cli.command {
        Commands::Submit { data } => {
            cmd_submit(&program, data)?;
        }
        Commands::GetSubmission { submission_index } => {
            cmd_get_submission(&program, submission_index)?;
        }
        Commands::GetAllSubmissions { contributor } => {
            cmd_get_all_submissions(&program, contributor)?;
        }
        Commands::DebugAta { user } => {
            cmd_debug_ata(&program, user)?;
        }
        Commands::GetTxLogs { signature } => {
            cmd_get_tx_logs(&program, signature)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------
// Command Handlers
// ---------------------------------------------------

/// Submits data on-chain by calling `submit_data`.
fn cmd_submit(program: &Program<Rc<Keypair>>, data: String) -> Result<()> {
    // Derive the PDAs from seeds
    let (state_pda, _state_bump) = Pubkey::find_program_address(&[b"state"], &program.id());
    let (mint_pda, _mint_bump) = Pubkey::find_program_address(&[b"mint"], &program.id());

    // If we want the contributor to be the CLI payer:
    let contributor_pubkey = program.payer(); // Public key from the wallet loaded above
    let ata_pubkey = get_associated_token_address(&contributor_pubkey, &mint_pda);

    // Check if the ATA exists first using get_account_info
    println!("Checking if ATA exists at {}...", ata_pubkey);

    // Check if the ATA already exists by attempting to get its account info
    let ata_exists = match program.rpc().get_account(&ata_pubkey) {
        Ok(_) => {
            println!("ATA already exists, skipping creation step");
            true
        }
        Err(_) => {
            println!("ATA does not exist, will create it");
            false
        }
    };

    // Only try to create the ATA if it doesn't exist
    if !ata_exists {
        println!("Creating ATA at {}...", ata_pubkey);

        // Try to create the ATA
        let create_ata_result = program
            .request()
            .accounts(CreateUserAtaAccounts {
                payer: contributor_pubkey,
                user: contributor_pubkey,
                mint: mint_pda,
                user_ata: ata_pubkey,
                system_program: system_program::ID,
                token_program: anchor_spl::token::ID,
                associated_token_program: anchor_spl::associated_token::ID,
                rent: RENT_ID,
            })
            .args(CreateUserAtaIx {})
            .send();

        match create_ata_result {
            Ok(sig) => println!("Successfully created new ATA (txSig: {})", sig),
            Err(e) => {
                // Print detailed error information
                println!("ATA creation failed with error: {}", e);
                println!("Error type: {:?}", e.to_string());

                // Continue anyway since the submission might still work
                println!("Continuing with submission despite ATA creation failure...");
            }
        }
    }

    // If we need the next submission index, we can fetch the current State from chain
    let state_data: StateAccount = program.account(state_pda)?;
    let next_index = state_data.submission_count;

    // Derive the next submission PDA (the program seeds with [b"submission", submission_count])
    let (submission_pda, _sub_bump) =
        Pubkey::find_program_address(&[b"submission", &next_index.to_le_bytes()], &program.id());

    // Now build the typed `SubmitData` accounts struct (as declared in your Anchor program)
    let accounts = SubmitDataAccounts {
        state: state_pda,
        mint: mint_pda,
        contributor_ata: ata_pubkey,
        submission: submission_pda,
        contributor: contributor_pubkey,
        token_program: anchor_spl::token::ID,
        system_program: system_program::ID,
        rent: RENT_ID,
    };

    // Build the instruction data struct
    let ix_data = SubmitDataIx { data_str: data };

    // Send the transaction
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(ix_data) // Anchor automatically serializes the instruction data
        .send()?;

    println!("submitData txSig: {}", tx_sig);
    Ok(())
}

/// Fetch a given submission by its index.
fn cmd_get_submission(program: &Program<Rc<Keypair>>, submission_index: u64) -> Result<()> {
    // Derive the same PDA the program uses for that submission index
    let (submission_pda, _) = Pubkey::find_program_address(
        &[b"submission", &submission_index.to_le_bytes()],
        &program.id(),
    );

    // If we want the typed submission data:
    let submission_data: SubmissionAccount = program.account(submission_pda)?;

    println!("Submission PDA = {}", submission_pda);
    println!("Contributor = {}", submission_data.contributor);
    println!("Timestamp = {}", submission_data.timestamp);
    println!("Data = {}", submission_data.data);

    Ok(())
}

/// Fetch and display all submission accounts
fn cmd_get_all_submissions(
    program: &Program<Rc<Keypair>>,
    contributor: Option<String>,
) -> Result<()> {
    // Derive the PDAs from seeds
    let (state_pda, _state_bump) = Pubkey::find_program_address(&[b"state"], &program.id());

    // Get state data to find total submission count
    let state_data: StateAccount = program.account(state_pda)?;
    let submission_count = state_data.submission_count;

    if submission_count == 0 {
        println!("No submissions found.");
        return Ok(());
    }

    println!("Total submissions in protocol: {}", submission_count);

    // Parse contributor pubkey if provided
    let contributor_pubkey = match contributor {
        Some(pubkey_str) => match Pubkey::from_str(&pubkey_str) {
            Ok(pubkey) => Some(pubkey),
            Err(e) => {
                return Err(anyhow::anyhow!("Invalid contributor pubkey: {}", e));
            }
        },
        None => None,
    };

    // Print filter info if contributor filter is active
    if let Some(pubkey) = contributor_pubkey {
        println!("Filtering for contributor: {}", pubkey);
    }

    let mut matched_count = 0;

    // Iterate through all submission indices
    for i in 0..submission_count {
        let (submission_pda, _) =
            Pubkey::find_program_address(&[b"submission", &i.to_le_bytes()], &program.id());

        // Fetch the submission account data
        let submission_data: SubmissionAccount =
            match program.account::<SubmissionAccount>(submission_pda) {
                Ok(data) => data,
                Err(e) => {
                    println!("Warning: Failed to fetch submission #{}: {}", i, e);
                    continue;
                }
            };

        // If contributor filter is specified, skip non-matching submissions
        if let Some(pubkey) = contributor_pubkey {
            if submission_data.contributor != pubkey {
                continue;
            }
        }

        matched_count += 1;

        println!("\nSubmission #{}", i);
        println!("PDA = {}", submission_pda);
        println!("Contributor = {}", submission_data.contributor);
        println!("Timestamp = {}", submission_data.timestamp);
        println!("Data = {}", submission_data.data);
    }

    println!(
        "\nDisplayed {} submissions matching the criteria",
        matched_count
    );

    Ok(())
}

/// Debug ATA status for a user
fn cmd_debug_ata(program: &Program<Rc<Keypair>>, user: Option<String>) -> Result<()> {
    // Derive the PDAs from seeds
    let (state_pda, _state_bump) = Pubkey::find_program_address(&[b"state"], &program.id());
    let (mint_pda, _mint_bump) = Pubkey::find_program_address(&[b"mint"], &program.id());

    // If no user is provided, use the CLI payer
    let user_pubkey = match user {
        Some(pubkey_str) => Pubkey::from_str(&pubkey_str)?,
        None => program.payer(),
    };

    let ata_pubkey = get_associated_token_address(&user_pubkey, &mint_pda);

    println!("==== ATA Debug Information ====");
    println!("User pubkey: {}", user_pubkey);
    println!("Mint pubkey: {}", mint_pda);
    println!("ATA pubkey: {}", ata_pubkey);

    // Check if the ATA exists by attempting to get its account info
    println!("\nChecking if ATA exists...");

    match program.rpc().get_account(&ata_pubkey) {
        Ok(account) => {
            println!("✅ ATA exists with the following details:");
            println!("   - Owner: {}", account.owner);
            println!("   - Lamports: {}", account.lamports);
            println!("   - Data length: {} bytes", account.data.len());

            // Try to deserialize as a token account
            match program.account::<anchor_spl::token::TokenAccount>(ata_pubkey) {
                Ok(token_account) => {
                    println!("   - Token account data:");
                    println!("     * Mint: {}", token_account.mint);
                    println!("     * Owner: {}", token_account.owner);
                    println!("     * Amount: {}", token_account.amount);

                    // Check if the mint matches the expected mint
                    if token_account.mint == mint_pda {
                        println!("✅ ATA mint matches the protocol mint");
                    } else {
                        println!("❌ ATA mint does NOT match the protocol mint!");
                    }

                    // Check if the owner matches the user
                    if token_account.owner == user_pubkey {
                        println!("✅ ATA owner matches the user");
                    } else {
                        println!("❌ ATA owner does NOT match the user!");
                    }
                }
                Err(e) => {
                    println!("❌ Could not deserialize as token account: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ ATA does not exist: {}", e);

            println!("\nAttempting to create ATA...");
            // Try to create the ATA
            let create_ata_result = program
                .request()
                .accounts(CreateUserAtaAccounts {
                    payer: program.payer(),
                    user: user_pubkey,
                    mint: mint_pda,
                    user_ata: ata_pubkey,
                    system_program: system_program::ID,
                    token_program: anchor_spl::token::ID,
                    associated_token_program: anchor_spl::associated_token::ID,
                    rent: RENT_ID,
                })
                .args(CreateUserAtaIx {})
                .send();

            match create_ata_result {
                Ok(sig) => println!("✅ Successfully created new ATA (txSig: {})", sig),
                Err(e) => {
                    println!("❌ ATA creation failed with error: {}", e);
                    println!("   Error details: {:?}", e.to_string());
                }
            }
        }
    }

    // Get state data to check tokens_to_mint
    match program.account::<StateAccount>(state_pda) {
        Ok(state_data) => {
            println!("\nState account information:");
            println!(
                "   - Tokens to mint per submission: {}",
                state_data.tokens_to_mint
            );
            println!("   - Total submissions: {}", state_data.submission_count);
        }
        Err(e) => {
            println!("\n❌ Could not fetch state account: {}", e);
        }
    }

    println!("\n==== End of ATA Debug Information ====");
    Ok(())
}

/// Get detailed transaction logs for debugging
fn cmd_get_tx_logs(_program: &Program<Rc<Keypair>>, signature: String) -> Result<()> {
    println!("Fetching logs for transaction: {}", signature);

    // Use std::process::Command to call the Solana CLI instead
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
