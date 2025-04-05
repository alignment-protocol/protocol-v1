use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::{
    solana_sdk::{pubkey::Pubkey, system_program, sysvar::rent::ID as RENT_ID},
    Program,
};
use anyhow::Result;
use std::rc::Rc;
use std::str::FromStr;

use alignment_protocol::{
    accounts as AccountsAll, instruction as InstructionAll, State as StateAccount,
    UserProfile as UserProfileAccount, UserTopicBalance as UserTopicBalanceAccount,
};

use crate::commands::common::pda::{
    get_state_pda, get_token_ata, get_user_profile_pda, get_user_temp_token_account_pda,
};

/// Create a complete user profile with all necessary token accounts
pub fn cmd_create_user_profile(program: &Program<Rc<Keypair>>) -> Result<()> {
    let user = program.payer();
    let (user_profile_pda, _) = get_user_profile_pda(program, &user);
    let (state_pda, _) = get_state_pda(program);

    println!("Creating complete user profile for {}", user);
    println!("User profile PDA: {}", user_profile_pda);

    // Step 1: Create user profile
    println!("Step 1: Creating user profile...");
    let accounts = AccountsAll::CreateUserProfile {
        user,
        state: state_pda,
        user_profile: user_profile_pda,
        system_program: system_program::ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::CreateUserProfile {})
        .send()?;

    println!("User profile created successfully (txSig: {})", tx_sig);

    // Get state account data to access token mint addresses
    let state_data: StateAccount = program.account(state_pda)?;

    // Step 2: Create ATA for permanent alignment token
    println!("Step 2: Creating permanent alignment token account...");
    let align_mint = state_data.align_mint;
    let align_ata = get_token_ata(&user, &align_mint);

    let accounts = AccountsAll::CreateUserAta {
        payer: user,
        user,
        user_profile: user_profile_pda,
        state: state_pda,
        mint: align_mint,
        user_ata: align_ata,
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

    println!("Permanent alignment token account created: {}", align_ata);
    println!("Transaction signature: {}", tx_sig);

    // Step 3: Create ATA for permanent reputation token
    println!("Step 3: Creating permanent reputation token account...");
    let rep_mint = state_data.rep_mint;
    let rep_ata = get_token_ata(&user, &rep_mint);

    let accounts = AccountsAll::CreateUserAta {
        payer: user,
        user,
        user_profile: user_profile_pda,
        state: state_pda,
        mint: rep_mint,
        user_ata: rep_ata,
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

    println!("Permanent reputation token account created: {}", rep_ata);
    println!("Transaction signature: {}", tx_sig);

    // Step 4: Create protocol-owned temporary alignment token account
    println!("Step 4: Creating protocol-owned temporary alignment token account...");
    let temp_align_mint = state_data.temp_align_mint;
    let (temp_align_account_pda, _) =
        get_user_temp_token_account_pda(program, &user, "user_temp_align");

    let accounts = AccountsAll::CreateUserTempAlignAccount {
        payer: user,
        user,
        user_profile: user_profile_pda,
        state: state_pda,
        mint: temp_align_mint,
        token_account: temp_align_account_pda,
        system_program: system_program::ID,
        token_program: anchor_spl::token::ID,
        rent: RENT_ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::CreateUserTempAlignAccount {})
        .send()?;

    println!(
        "Temporary alignment token account created: {}",
        temp_align_account_pda
    );
    println!("Transaction signature: {}", tx_sig);

    // Step 5: Create protocol-owned temporary reputation token account
    println!("Step 5: Creating protocol-owned temporary reputation token account...");
    let temp_rep_mint = state_data.temp_rep_mint;
    let (temp_rep_account_pda, _) =
        get_user_temp_token_account_pda(program, &user, "user_temp_rep");

    let accounts = AccountsAll::CreateUserTempRepAccount {
        payer: user,
        user,
        user_profile: user_profile_pda,
        state: state_pda,
        mint: temp_rep_mint,
        token_account: temp_rep_account_pda,
        system_program: system_program::ID,
        token_program: anchor_spl::token::ID,
        rent: RENT_ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::CreateUserTempRepAccount {})
        .send()?;

    println!(
        "Temporary reputation token account created: {}",
        temp_rep_account_pda
    );
    println!("Transaction signature: {}", tx_sig);

    println!("\nUser profile setup completed successfully!");
    println!("Use 'alignment-protocol-cli user profile' to view your profile details");

    Ok(())
}

/// View user profile information
pub fn cmd_view_user_profile(
    program: &Program<Rc<Keypair>>,
    user_str: Option<String>,
) -> Result<()> {
    let user = match user_str {
        Some(pubkey_str) => Pubkey::from_str(&pubkey_str)?,
        None => program.payer(),
    };

    let (user_profile_pda, _) = get_user_profile_pda(program, &user);

    match program.account::<UserProfileAccount>(user_profile_pda) {
        Ok(profile) => {
            println!("User Profile for {}", user);
            println!("Profile PDA: {}", user_profile_pda);
            println!("User Submission Count: {}", profile.user_submission_count);

            // Display optional token account references stored in the profile struct
            println!("\nToken Account References (Stored in Profile):");

            if profile.user_temp_align_account != Pubkey::default() {
                println!(
                    "  User Temp Align Account: {}",
                    profile.user_temp_align_account
                );
            } else {
                println!("  User Temp Align Account: Not Set");
            }
            if profile.user_temp_rep_account != Pubkey::default() {
                println!("  User Temp Rep Account: {}", profile.user_temp_rep_account);
            } else {
                println!("  User Temp Rep Account: Not Set");
            }
            if profile.user_align_ata != Pubkey::default() {
                println!("  User Align ATA: {}", profile.user_align_ata);
            } else {
                println!("  User Align ATA: Not Set");
            }
            if profile.user_rep_ata != Pubkey::default() {
                println!("  User Rep ATA: {}", profile.user_rep_ata);
            } else {
                println!("  User Rep ATA: Not Set");
            }

            // Display UserTopicBalance accounts
            println!("\nTopic Balances:");
            // Fetch ALL UserTopicBalance accounts first
            match program.accounts::<UserTopicBalanceAccount>(vec![]) {
                Ok(all_balance_accounts) => {
                    // Filter client-side
                    let user_balance_accounts: Vec<_> = all_balance_accounts
                        .into_iter()
                        .filter(|(_, balance_account)| balance_account.user == user)
                        .collect();

                    if user_balance_accounts.is_empty() {
                        println!("  No topic-specific balances found for this user.");
                    } else {
                        for (pda, balance_account) in user_balance_accounts {
                            // Iterate over filtered list
                            // Fetching Topic name is skipped for CLI simplicity for now
                            println!("  Topic PDA: {}", balance_account.topic);
                            println!("    Balance PDA: {}", pda); // Show the PDA of the balance account itself
                            println!("    Temp Align: {}", balance_account.temp_align_amount);
                            println!("    Temp Rep: {}", balance_account.temp_rep_amount);
                            println!(
                                "    Locked Temp Rep: {}",
                                balance_account.locked_temp_rep_amount
                            );
                        }
                    }
                }
                Err(e) => {
                    println!("  Error fetching topic balances: {}", e);
                }
            }

            // Existing code to display actual token account addresses and balances
            // This remains useful as it shows the derived/expected ATAs/PDAs and their current balances
            match program.account::<StateAccount>(get_state_pda(program).0) {
                Ok(state_data) => {
                    println!("\nToken Account Details (Derived & Checked):");

                    // Permanent align token account
                    let align_ata = get_token_ata(&user, &state_data.align_mint);
                    let align_balance =
                        match program.account::<anchor_spl::token::TokenAccount>(align_ata) {
                            Ok(token_account) => token_account.amount.to_string(),
                            Err(_) => "Not found/created".to_string(),
                        };
                    println!(
                        "  Permanent Align Token ATA: {} (Balance: {})",
                        align_ata, align_balance
                    );

                    // Permanent rep token account
                    let rep_ata = get_token_ata(&user, &state_data.rep_mint);
                    let rep_balance =
                        match program.account::<anchor_spl::token::TokenAccount>(rep_ata) {
                            Ok(token_account) => token_account.amount.to_string(),
                            Err(_) => "Not found/created".to_string(),
                        };
                    println!(
                        "  Permanent Rep Token ATA: {} (Balance: {})",
                        rep_ata, rep_balance
                    );

                    // Temporary align token account (protocol-owned)
                    let (temp_align_account_pda, _) =
                        get_user_temp_token_account_pda(program, &user, "user_temp_align");
                    let temp_align_balance = match program
                        .account::<anchor_spl::token::TokenAccount>(temp_align_account_pda)
                    {
                        Ok(token_account) => token_account.amount.to_string(),
                        Err(_) => "Not found/created".to_string(),
                    };
                    println!(
                        "  Temp Align Token PDA: {} (Balance: {})",
                        temp_align_account_pda, temp_align_balance
                    );

                    // Temporary rep token account (protocol-owned)
                    let (temp_rep_account_pda, _) =
                        get_user_temp_token_account_pda(program, &user, "user_temp_rep");
                    let temp_rep_balance = match program
                        .account::<anchor_spl::token::TokenAccount>(temp_rep_account_pda)
                    {
                        Ok(token_account) => token_account.amount.to_string(),
                        Err(_) => "Not found/created".to_string(),
                    };
                    println!(
                        "  Temp Rep Token PDA: {} (Balance: {})",
                        temp_rep_account_pda, temp_rep_balance
                    );
                }
                Err(e) => {
                    println!(
                        "\nCould not fetch protocol state to check token accounts: {}",
                        e
                    );
                }
            }

            Ok(())
        }
        Err(e) => {
            println!("\nUser profile {} not found: {}", user_profile_pda, e);
            println!("Hint: Create a profile using `alignment-protocol-cli user create-profile`");
            // Consider returning an error or specific exit code here if desired
            Ok(()) // Keep Ok(()) for now to avoid breaking changes in CLI exit behavior
        }
    }
}
