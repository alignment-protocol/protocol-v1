use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair},
    },
    Client, Cluster, Program,
};
use anyhow::Result;
use std::rc::Rc;
use std::str::FromStr;

/// Setup the program client
pub fn setup_client(
    keypair_path: &str,
    cluster: &str,
    program_id: &str,
) -> Result<Program<Rc<Keypair>>> {
    // Load keypair and connect to cluster
    let keypair_path = shellexpand::tilde(keypair_path).to_string();
    let payer = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair file: {}", e))?;

    let cluster = match cluster {
        "devnet" => Cluster::Devnet,
        "mainnet" => Cluster::Mainnet,
        url => Cluster::Custom(url.to_string(), url.to_string()),
    };

    let client = Client::new_with_options(cluster, Rc::new(payer), CommitmentConfig::confirmed());
    let program_id = Pubkey::from_str(program_id)?;
    let program = client.program(program_id).expect("Failed to load program");

    Ok(program)
}
