use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{read_keypair_file, EncodableKey},
        signer::keypair::Keypair,
        system_program,
    },
    Client, Cluster, Program,
};
use anchor_spl::associated_token::get_associated_token_address;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::rc::Rc;
use std::str::FromStr;

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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 1) Load the keypair
    let keypair_path = shellexpand::tilde(&cli.keypair).to_string();
    let payer = read_keypair_file(&keypair_path).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    // 2) Connect to cluster using anchor-client
    let cluster = match cli.cluster.as_str() {
        "devnet" => Cluster::Devnet,
        "mainnet" => Cluster::Mainnet,
        // Could also parse "localnet" or a custom URL
        url => Cluster::Custom(url.to_string(), url.to_string()),
    };
    let client = Client::new_with_options(cluster, Rc::new(payer), CommitmentConfig::confirmed());

    // 3) The program ID of your alignment_protocol
    //    Replace with your actual ID (the one you used in declare_id!)
    let program_id = Pubkey::from_str("BMYn8rtstaZhzFZtgMVMY9io1zhnqacr3yANZrgkv7DF")?;
    let program = client.program(program_id).expect("Failed to get program");

    match cli.command {
        Commands::Submit { data } => {
            cmd_submit(&program, data).await?;
        }
        Commands::GetSubmission { submission_index } => {
            cmd_get_submission(&program, submission_index).await?;
        }
    }

    Ok(())
}

// ---------------------------------------------------
// Command Handlers
// ---------------------------------------------------

async fn cmd_submit(program: &Program<Rc<Keypair>>, data: String) -> Result<()> {
    // We fetch state to get the current submission_count or just do ephemeral
    let (state_pda, _) = Pubkey::find_program_address(&[b"state"], &program.id());
    let (mint_pda, _) = Pubkey::find_program_address(&[b"mint"], &program.id());

    // If we want to use the CLI payer as the "contributor" too:
    let contributor_pubkey = program.payer();
    // Derive contributor's ATA
    let ata_pubkey = get_associated_token_address(&contributor_pubkey, &mint_pda);

    // We want the next submission PDA. We can do that by fetching the state account from chain or
    // letting the on-chain code do the seeds. But let's assume we want a read to get submission_count:
    // If your IDL is known, you can do:
    // let state: State = program.account(state_pda)?;
    // let next_count = state.submission_count;
    // etc. Then derive submission. Or let the program do it automatically if it seeds with that count.

    // We'll just build the instruction to call 'submitData'
    // We'll pass data + tokens_to_mint
    let tx_sig = program
        .request()
        .accounts([
            ("state", state_pda),
            ("mint", mint_pda),
            ("contributorAta", ata_pubkey),
            ("contributor", contributor_pubkey),
            ("tokenProgram", anchor_client::solana_sdk::system_program::id()),
            ("systemProgram", system_program::id()),
            ("rent", anchor_client::solana_sdk::sysvar::rent::id()),
        ])
        .args((data, tokens_to_mint))
        .signer(&*program.payer())
        .send()?;

    println!("submitData txSig: {}", tx_sig);
    Ok(())
}

async fn cmd_get_submission(program: &Program<Rc<Keypair>>, submission_index: u64) -> Result<()> {
    // We'll derive submission PDA from [b"submission", submission_index],
    // or fetch them all from the program. For a single example:
    let (submission_pda, _) = Pubkey::find_program_address(
        &[b"submission", &submission_index.to_le_bytes()],
        &program.id(),
    );
    let submission_data: serde_json::Value = program.account(submission_pda)?;
    // If you have your IDL typed, you can do let submission: Submission = program.account(submission_pda)?;

    println!("Submission PDA = {}", submission_pda);
    println!("Submission data: {:#}", submission_data);
    Ok(())
}
