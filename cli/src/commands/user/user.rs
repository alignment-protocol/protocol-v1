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
    UserProfile as UserProfileAccount,
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
        rent: RENT_ID,
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

            // Get state account to access mint addresses
            let (state_pda, _) = get_state_pda(program);
            match program.account::<StateAccount>(state_pda) {
                Ok(state_data) => {
                    // Show token account addresses and balances
                    println!("\nToken Accounts:");

                    // Permanent align token account
                    let align_ata = get_token_ata(&user, &state_data.align_mint);
                    let align_balance =
                        match program.account::<anchor_spl::token::TokenAccount>(align_ata) {
                            Ok(token_account) => token_account.amount,
                            Err(_) => 0,
                        };
                    println!(
                        "  Permanent Align Token ATA: {} (Balance: {})",
                        align_ata, align_balance
                    );

                    // Permanent rep token account
                    let rep_ata = get_token_ata(&user, &state_data.rep_mint);
                    let rep_balance =
                        match program.account::<anchor_spl::token::TokenAccount>(rep_ata) {
                            Ok(token_account) => token_account.amount,
                            Err(_) => 0,
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
                        Ok(token_account) => token_account.amount,
                        Err(_) => 0,
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
                        Ok(token_account) => token_account.amount,
                        Err(_) => 0,
                    };
                    println!(
                        "  Temp Rep Token PDA: {} (Balance: {})",
                        temp_rep_account_pda, temp_rep_balance
                    );
                }
                Err(e) => {
                    println!("Could not fetch protocol state: {}", e);
                }
            }

            Ok(())
        }
        Err(e) => {
            println!("User profile not found: {}", e);
            println!("Create a profile with 'alignment-protocol-cli user create-profile'");
            Ok(())
        }
    }
}
