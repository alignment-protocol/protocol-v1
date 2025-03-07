use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::{
    solana_sdk::{system_program, sysvar},
    Program,
};
use anyhow::Result;
use std::rc::Rc;

use alignment_protocol::{
    accounts as AccountsAll, instruction as InstructionAll, State as StateAccount,
};

use crate::utils::pda::{
    get_state_pda, get_submission_pda, get_submission_topic_link_pda, get_token_ata, get_topic_pda,
    get_user_profile_pda, get_user_temp_token_account_pda, get_vote_commit_pda,
};
use crate::utils::vote::{generate_vote_hash, parse_vote_choice};

/// Commit a vote (first phase)
pub fn cmd_commit_vote(
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
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    let (vote_commit_pda, _) = get_vote_commit_pda(program, &submission_topic_link_pda, &validator);

    // Parse vote choice
    let vote_choice = parse_vote_choice(&choice_str)?;

    // Generate vote hash
    let vote_hash =
        generate_vote_hash(&validator, &submission_topic_link_pda, &vote_choice, &nonce);

    println!(
        "Committing {} vote on submission #{} in topic #{}",
        choice_str, submission_id, topic_id
    );
    println!("Vote amount: {}", amount);
    println!(
        "Using {} reputation",
        if permanent { "permanent" } else { "temporary" }
    );
    println!("Nonce: {}", nonce);
    println!("Generated hash: {:?}", vote_hash);

    let (state_pda, _) = get_state_pda(program);

    let accounts = AccountsAll::CommitVote {
        validator,
        user_profile: user_profile_pda,
        submission_topic_link: submission_topic_link_pda,
        submission: submission_pda,
        state: state_pda,
        topic: topic_pda,
        vote_commit: vote_commit_pda,
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
    println!("Save your vote choice and nonce for the reveal phase!");
    Ok(())
}

/// Reveal a vote (second phase)
pub fn cmd_reveal_vote(
    program: &Program<Rc<Keypair>>,
    submission_id: u64,
    topic_id: u64,
    choice_str: String,
    nonce: String,
) -> Result<()> {
    let validator = program.payer();
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    let (vote_commit_pda, _) = get_vote_commit_pda(program, &submission_topic_link_pda, &validator);

    // Parse vote choice
    let vote_choice = parse_vote_choice(&choice_str)?;

    println!(
        "Revealing {} vote on submission #{} in topic #{}",
        choice_str, submission_id, topic_id
    );
    println!("Nonce: {}", nonce);

    let (state_pda, _) = get_state_pda(program);
    let (user_profile_pda, _) = get_user_profile_pda(program, &validator);

    let accounts = AccountsAll::RevealVote {
        validator,
        user_profile: user_profile_pda,
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
    submission_id: u64,
    topic_id: u64,
) -> Result<()> {
    let validator = program.payer();
    let (state_pda, _) = get_state_pda(program);
    let (validator_profile_pda, _) = get_user_profile_pda(program, &validator);
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    let (vote_commit_pda, _) = get_vote_commit_pda(program, &submission_topic_link_pda, &validator);

    // Get mint addresses from state
    let state_data: StateAccount = program.account(state_pda)?;
    let temp_rep_mint = state_data.temp_rep_mint;
    let rep_mint = state_data.rep_mint;

    // Get token accounts
    let (validator_temp_rep_account_pda, _) =
        get_user_temp_token_account_pda(program, &validator, "temp_rep_account");
    let validator_rep_ata = get_token_ata(&validator, &rep_mint);

    println!(
        "Finalizing vote on submission #{} in topic #{}",
        submission_id, topic_id
    );

    let accounts = AccountsAll::FinalizeVote {
        authority: program.payer(),
        state: state_pda,
        submission: submission_pda,
        submission_topic_link: submission_topic_link_pda,
        topic: topic_pda,
        vote_commit: vote_commit_pda,
        validator_profile: validator_profile_pda,
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

/// Set arbitrary timestamps for voting phases (admin function)
pub fn cmd_set_voting_phases(
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
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);

    println!(
        "Setting voting phases for submission #{} in topic #{}",
        submission_id, topic_id
    );
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
        submission: submission_pda,
        submission_topic_link: submission_topic_link_pda,
        topic: topic_pda,
        system_program: system_program::ID,
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
