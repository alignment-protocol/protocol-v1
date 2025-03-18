use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use anyhow::{anyhow, Result};
use dirs::home_dir;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::rc::Rc;

use alignment_protocol::{accounts as AccountsAll, instruction as InstructionAll};

use crate::commands::admin::init::is_state_initialized;
use crate::commands::common::pda::get_state_pda;

/// Update the number of tokens to mint per submission (admin only)
pub fn cmd_admin_update_tokens_to_mint(program: &Program<Rc<Keypair>>, tokens: u64) -> Result<()> {
    // Check if state is initialized
    if !is_state_initialized(program) {
        return Err(anyhow!(
            "Protocol state not initialized. Run 'init state' first."
        ));
    }

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
            new_tokens_to_mint: tokens,
        })
        .send()?;

    println!("Tokens to mint updated successfully (txSig: {})", tx_sig);
    Ok(())
}

/// Get the path to the CLI config file
fn get_config_file_path() -> Result<PathBuf> {
    let mut path = home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    path.push(".alignment-protocol");
    fs::create_dir_all(&path)?;
    path.push("config.txt");
    Ok(path)
}

/// Save cluster configuration to file
fn save_cluster_config(cluster: &str) -> Result<()> {
    let path = get_config_file_path()?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;

    writeln!(file, "cluster={}", cluster)?;

    Ok(())
}

/// Read cluster configuration from file
pub fn read_cluster_config() -> Result<String> {
    let path = get_config_file_path()?;

    // If config file doesn't exist, return default
    if !path.exists() {
        return Ok("http://127.0.0.1:8899".to_string());
    }

    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    for line in contents.lines() {
        if let Some(cluster_str) = line.strip_prefix("cluster=") {
            return Ok(cluster_str.to_string());
        }
    }

    // Default if not found
    Ok("http://127.0.0.1:8899".to_string())
}

/// Resolve cluster name to URL if using predefined networks
fn resolve_cluster(cluster: &str) -> String {
    match cluster.to_lowercase().as_str() {
        "localnet" => "http://127.0.0.1:8899".to_string(),
        "devnet" => "https://api.devnet.solana.com".to_string(),
        "testnet" => "https://api.testnet.solana.com".to_string(),
        "mainnet-beta" | "mainnet" => "https://api.mainnet-beta.solana.com".to_string(),
        custom_url => custom_url.to_string(),
    }
}

/// Set and save the cluster configuration
pub fn cmd_admin_set_cluster(cluster: Option<String>) -> Result<()> {
    // Show available options when no cluster is provided
    if cluster.is_none() {
        println!("Available cluster options:");
        println!("  localnet     - Local Solana validator (http://127.0.0.1:8899)");
        println!("  devnet       - Solana devnet (https://api.devnet.solana.com)");
        println!("  testnet      - Solana testnet (https://api.testnet.solana.com)");
        println!("  mainnet-beta - Solana mainnet (https://api.mainnet-beta.solana.com)");
        println!("  [custom URL] - Any custom RPC endpoint");
        println!("\nUsage: align config set-cluster <CLUSTER>");
        println!("Example: align config set-cluster devnet");
        return Ok(());
    }

    let cluster_str = cluster.unwrap();
    let resolved_cluster = resolve_cluster(&cluster_str);
    save_cluster_config(&resolved_cluster)?;

    println!("Cluster configuration set to: {}", cluster_str);
    if resolved_cluster != cluster_str {
        println!("Resolved to URL: {}", resolved_cluster);
    }
    println!("Configuration saved. This setting will persist between CLI runs.");
    println!("The CLI will automatically use this cluster setting.");
    println!("You can override it with --cluster if needed.");

    Ok(())
}

/// Get the current cluster configuration
pub fn cmd_admin_get_cluster() -> Result<()> {
    let cluster = read_cluster_config()?;

    println!("Current saved cluster configuration: {}", cluster);
    println!("The CLI will automatically use this cluster setting.");
    println!("You can override it with --cluster if needed.");
    println!("\nTip: Run 'align config set-cluster' (without arguments) to see available cluster options");

    Ok(())
}
