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

use crate::commands::common::pda::{get_state_pda, get_topic_pda};

/// Create a new topic (admin only)
pub fn cmd_create_topic(
    program: &Program<Rc<Keypair>>,
    name: String,
    description: String,
    commit_duration: Option<u64>,
    reveal_duration: Option<u64>,
) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);

    // Get the state to determine the next topic ID
    let state_data: StateAccount = program.account(state_pda)?;
    let topic_id = state_data.topic_count;

    let (topic_pda, _) = get_topic_pda(program, topic_id);

    println!("Creating new topic with ID {}", topic_id);
    println!("Name: {}", name);
    println!("Description: {}", description);

    let accounts = AccountsAll::CreateTopic {
        creator: program.payer(),
        state: state_pda,
        topic: topic_pda,
        system_program: system_program::ID,
        rent: sysvar::rent::ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::CreateTopic {
            name,
            description,
            commit_phase_duration: commit_duration,
            reveal_phase_duration: reveal_duration,
        })
        .send()?;

    println!("Topic created successfully (txSig: {})", tx_sig);
    println!("Topic ID: {}", topic_id);
    println!("Topic PDA: {}", topic_pda);
    Ok(())
}
