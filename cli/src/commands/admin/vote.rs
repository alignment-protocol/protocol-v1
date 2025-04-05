use anchor_client::solana_sdk::{pubkey::Pubkey, signature::Keypair};
use anchor_client::{solana_sdk::system_program, Program};
use anyhow::Result;
use std::rc::Rc;
use std::str::FromStr;

use alignment_protocol::{accounts as AccountsAll, instruction as InstructionAll};

use crate::commands::common::pda::{get_state_pda, get_submission_topic_link_pda, get_topic_pda};

/// Set arbitrary timestamps for voting phases (admin function)
pub fn cmd_set_voting_phases(
    program: &Program<Rc<Keypair>>,
    submission_pda_str: String,
    topic_id: u64,
    commit_start: Option<u64>,
    commit_end: Option<u64>,
    reveal_start: Option<u64>,
    reveal_end: Option<u64>,
) -> Result<()> {
    let submission_pda = Pubkey::from_str(&submission_pda_str)
        .map_err(|e| anyhow::anyhow!("Invalid Submission PDA format: {}", e))?;

    let (state_pda, _) = get_state_pda(program);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);

    println!(
        "Setting voting phases for submission {} in topic #{}",
        submission_pda, topic_id
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
