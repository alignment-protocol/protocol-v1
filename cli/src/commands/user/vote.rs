use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::{
    solana_sdk::{system_program, sysvar},
    Program,
};
use anyhow::Result;
use hex;
use rand::RngCore;
use std::rc::Rc;
use std::str::FromStr;

use alignment_protocol::{
    accounts as AccountsAll, instruction as InstructionAll, State as StateAccount,
};

use crate::commands::common::pda::{
    get_state_pda, get_submission_topic_link_pda, get_token_ata, get_topic_pda,
    get_user_profile_pda, get_user_temp_token_account_pda, get_user_topic_balance_pda,
    get_vote_commit_pda,
};
use crate::commands::common::vote::{generate_vote_hash, parse_vote_choice};

/// Commit a vote (first phase)
pub fn cmd_commit_vote(
    program: &Program<Rc<Keypair>>,
    submission_pda_str: String,
    topic_index: u64,
    choice_str: String,
    amount: u64,
    nonce_opt: Option<String>,
    permanent: bool,
) -> Result<()> {
    let submission_pda = Pubkey::from_str(&submission_pda_str)
        .map_err(|e| anyhow::anyhow!("Invalid Submission PDA format: {}", e))?;

    let validator = program.payer();
    let (user_profile_pda, _) = get_user_profile_pda(program, &validator);
    let (topic_pda, _) = get_topic_pda(program, topic_index);
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    let (vote_commit_pda, _) = get_vote_commit_pda(program, &submission_topic_link_pda, &validator);
    let (user_topic_balance_pda, _) = get_user_topic_balance_pda(program, &validator, &topic_pda);
    let (state_pda, _) = get_state_pda(program);

    let state_data: StateAccount = program.account(state_pda)?;
    let rep_mint = state_data.rep_mint;
    let validator_rep_ata = get_token_ata(&validator, &rep_mint);

    // Determine nonce: use provided or generate a new one
    let (actual_nonce, was_generated) = match nonce_opt {
        Some(n) => {
            println!("Using provided nonce: {}", n);
            (n, false)
        }
        None => {
            let mut nonce_bytes = [0u8; 16]; // 16 bytes for 128-bit entropy
            rand::rng().fill_bytes(&mut nonce_bytes);
            let generated_nonce = hex::encode(nonce_bytes);
            println!(
                "Generated nonce: {} (Please SAVE THIS for the reveal phase!)\n",
                generated_nonce
            );
            (generated_nonce, true)
        }
    };

    // Parse vote choice
    let vote_choice = parse_vote_choice(&choice_str)?;

    // Generate vote hash using the actual_nonce
    let vote_hash = generate_vote_hash(
        &validator,
        &submission_topic_link_pda,
        &vote_choice,
        &actual_nonce,
    );

    // Check if user profile exists
    let profile_exists = program.rpc().get_account(&user_profile_pda).is_ok();

    if !profile_exists {
        return Err(anyhow::anyhow!(
            "User profile not set up. Please run 'alignment-protocol-cli user create-profile' first."
        ));
    }

    println!(
        "Committing {} vote on submission {} in topic #{}",
        choice_str, submission_pda, topic_index
    );
    println!("Vote amount: {}", amount);
    println!(
        "Using {} reputation",
        if permanent { "permanent" } else { "temporary" }
    );
    // println!("Nonce: {}", actual_nonce);
    println!("Generated hash: {:?}", vote_hash);

    let accounts = AccountsAll::CommitVote {
        validator,
        payer: validator,
        user_profile: user_profile_pda,
        submission_topic_link: submission_topic_link_pda,
        submission: submission_pda,
        state: state_pda,
        topic: topic_pda,
        vote_commit: vote_commit_pda,
        user_topic_balance: user_topic_balance_pda,
        validator_rep_ata,
        system_program: system_program::ID,
        rent: sysvar::rent::ID,
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
    if was_generated {
        println!("Remember to save your generated nonce: {}", actual_nonce);
    } else {
        // If it was provided, actual_nonce holds the user's original string.
        println!(
            "Save your vote choice and provided nonce ({}) for the reveal phase!",
            actual_nonce
        );
    }
    Ok(())
}

/// Reveal a vote (second phase)
pub fn cmd_reveal_vote(
    program: &Program<Rc<Keypair>>,
    submission_pda_str: String,
    topic_index: u64,
    choice_str: String,
    nonce: String,
) -> Result<()> {
    let submission_pda = Pubkey::from_str(&submission_pda_str)
        .map_err(|e| anyhow::anyhow!("Invalid Submission PDA format: {}", e))?;

    let validator = program.payer();
    let (topic_pda, _) = get_topic_pda(program, topic_index);
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    let (vote_commit_pda, _) = get_vote_commit_pda(program, &submission_topic_link_pda, &validator);

    // Parse vote choice
    let vote_choice = parse_vote_choice(&choice_str)?;

    // Check if vote commit account exists
    let vote_commit_exists = program.rpc().get_account(&vote_commit_pda).is_ok();

    if !vote_commit_exists {
        return Err(anyhow::anyhow!(
            "Vote commit not found. Make sure you have committed a vote first."
        ));
    }

    println!(
        "Revealing {} vote on submission {} in topic #{}",
        choice_str, submission_pda, topic_index
    );
    println!("Nonce: {}", nonce);

    let (state_pda, _) = get_state_pda(program);

    let accounts = AccountsAll::RevealVote {
        validator,
        payer: validator,
        submission_topic_link: submission_topic_link_pda,
        submission: submission_pda,
        state: state_pda,
        topic: topic_pda,
        vote_commit: vote_commit_pda,
        system_program: system_program::ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::RevealVote { vote_choice, nonce })
        .send()?;

    println!("Vote revealed successfully (txSig: {})", tx_sig);
    Ok(())
}

/// Finalize a vote
pub fn cmd_finalize_vote(
    program: &Program<Rc<Keypair>>,
    submission_pda_str: String,
    topic_index: u64,
) -> Result<()> {
    let submission_pda = Pubkey::from_str(&submission_pda_str)
        .map_err(|e| anyhow::anyhow!("Invalid Submission PDA format: {}", e))?;

    let validator = program.payer();
    let (state_pda, _) = get_state_pda(program);
    let (validator_profile_pda, _) = get_user_profile_pda(program, &validator);
    let (topic_pda, _) = get_topic_pda(program, topic_index);
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    let (vote_commit_pda, _) = get_vote_commit_pda(program, &submission_topic_link_pda, &validator);
    let (user_topic_balance_pda, _) = get_user_topic_balance_pda(program, &validator, &topic_pda);

    let state_data: StateAccount = program.account(state_pda)?;
    let temp_rep_mint = state_data.temp_rep_mint;
    let rep_mint = state_data.rep_mint;

    let (validator_temp_rep_account_pda, _) =
        get_user_temp_token_account_pda(program, &validator, "user_temp_rep");
    let validator_rep_ata = get_token_ata(&validator, &rep_mint);

    println!(
        "Finalizing vote on submission {} in topic #{}",
        submission_pda, topic_index
    );

    let accounts = AccountsAll::FinalizeVote {
        authority: program.payer(),
        state: state_pda,
        submission: submission_pda,
        submission_topic_link: submission_topic_link_pda,
        topic: topic_pda,
        vote_commit: vote_commit_pda,
        validator_profile: validator_profile_pda,
        user_topic_balance: user_topic_balance_pda,
        validator_temp_rep_account: validator_temp_rep_account_pda,
        validator_rep_ata,
        temp_rep_mint,
        rep_mint,
        token_program: anchor_spl::token::ID,
        system_program: system_program::ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::FinalizeVote {})
        .send()?;

    println!("Vote finalized successfully (txSig: {})", tx_sig);
    Ok(())
}
