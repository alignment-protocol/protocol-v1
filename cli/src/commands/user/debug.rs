use anchor_client::solana_sdk::{pubkey::Pubkey, signature::Keypair};
use anchor_client::Program;
use anyhow::Result;
use std::process::Command;
use std::rc::Rc;
use std::str::FromStr;

use alignment_protocol::State as StateAccount;

use crate::commands::common::pda::{get_state_pda, get_token_ata, get_user_temp_token_account_pda};

/// Debug token account status
pub fn cmd_debug_token_account(
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
            let (temp_account_pda, _) =
                get_user_temp_token_account_pda(program, &user, "temp_align_account");

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
                        }
                        Err(e) => {
                            println!("❌ Could not deserialize as token account: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Account does not exist: {}", e);
                    println!("You can create it with: alignment-protocol-cli user create-temp-account temp-align");
                }
            }
        }
        "align" => {
            let mint = state_data.align_mint;
            let ata = get_token_ata(&user, &mint);

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
                        }
                        Err(e) => {
                            println!("❌ Could not deserialize as token account: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Account does not exist: {}", e);
                    println!(
                        "You can create it with: alignment-protocol-cli user create-ata align"
                    );
                }
            }
        }
        "temp-rep" => {
            let mint = state_data.temp_rep_mint;
            let (temp_account_pda, _) =
                get_user_temp_token_account_pda(program, &user, "temp_rep_account");

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
                        }
                        Err(e) => {
                            println!("❌ Could not deserialize as token account: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Account does not exist: {}", e);
                    println!("You can create it with: alignment-protocol-cli user create-temp-account temp-rep");
                }
            }
        }
        "rep" => {
            let mint = state_data.rep_mint;
            let ata = get_token_ata(&user, &mint);

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
                        }
                        Err(e) => {
                            println!("❌ Could not deserialize as token account: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Account does not exist: {}", e);
                    println!("You can create it with: alignment-protocol-cli user create-ata rep");
                }
            }
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid token type. Use temp-align, align, temp-rep, or rep"
            ));
        }
    }

    println!("\n==== End of Token Account Debug Information ====");
    Ok(())
}

/// Get transaction logs for debugging
pub fn cmd_get_tx_logs(_program: &Program<Rc<Keypair>>, signature: String) -> Result<()> {
    println!("Fetching logs for transaction: {}", signature);

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
