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

use crate::utils::pda::{
    get_state_pda, get_token_ata, get_user_profile_pda, get_user_temp_token_account_pda,
};

/// Create a user profile
pub fn cmd_create_user_profile(program: &Program<Rc<Keypair>>) -> Result<()> {
    let user = program.payer();
    let (user_profile_pda, _) = get_user_profile_pda(program, &user);

    println!("Creating user profile for {}", user);

    let (state_pda, _) = get_state_pda(program);

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
    println!("User profile PDA: {}", user_profile_pda);
    Ok(())
}

/// Create associated token account for a user
pub fn cmd_create_user_ata(program: &Program<Rc<Keypair>>, token_type: String) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let user = program.payer();

    // Get the mint address based on token type
    let state_data: StateAccount = program.account(state_pda)?;
    let mint = match token_type.to_lowercase().as_str() {
        "temp-align" => state_data.temp_align_mint,
        "align" => state_data.align_mint,
        "temp-rep" => state_data.temp_rep_mint,
        "rep" => state_data.rep_mint,
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid token type. Use temp-align, align, temp-rep, or rep"
            ))
        }
    };

    let ata = get_token_ata(&user, &mint);

    println!(
        "Creating {} associated token account for {}",
        token_type, user
    );
    println!("Mint: {}", mint);
    println!("ATA: {}", ata);

    let accounts = AccountsAll::CreateUserAta {
        payer: user,
        user,
        state: state_pda,
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

    println!(
        "Associated token account created successfully (txSig: {})",
        tx_sig
    );
    Ok(())
}

/// Create temporary token account (protocol-owned)
pub fn cmd_create_user_temp_account(
    program: &Program<Rc<Keypair>>,
    token_type: String,
) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let user = program.payer();

    // Get the mint address based on token type
    let state_data: StateAccount = program.account(state_pda)?;

    match token_type.to_lowercase().as_str() {
        "temp-align" => {
            let mint = state_data.temp_align_mint;
            let (temp_account_pda, _) =
                get_user_temp_token_account_pda(program, &user, "temp_align_account");

            println!("Creating temp-align account for {}", user);
            println!("Mint: {}", mint);
            println!("Account PDA: {}", temp_account_pda);

            let accounts = AccountsAll::CreateUserTempAlignAccount {
                payer: user,
                user,
                state: state_pda,
                mint: mint,
                token_account: temp_account_pda,
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
                "Temporary alignment token account created successfully (txSig: {})",
                tx_sig
            );
        }
        "temp-rep" => {
            let mint = state_data.temp_rep_mint;
            let (temp_account_pda, _) =
                get_user_temp_token_account_pda(program, &user, "temp_rep_account");

            println!("Creating temp-rep account for {}", user);
            println!("Mint: {}", mint);
            println!("Account PDA: {}", temp_account_pda);

            let accounts = AccountsAll::CreateUserTempRepAccount {
                payer: user,
                user,
                state: state_pda,
                mint: mint,
                token_account: temp_account_pda,
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
                "Temporary reputation token account created successfully (txSig: {})",
                tx_sig
            );
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid token type. Use temp-align or temp-rep"
            ))
        }
    }

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

            Ok(())
        }
        Err(e) => {
            println!("User profile not found: {}", e);
            println!("Create a profile with 'user create-profile'");
            Ok(())
        }
    }
}
