use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::{
    solana_sdk::{system_program, sysvar},
    Program,
};
use anyhow::Result;
use std::rc::Rc;

use alignment_protocol::{
    accounts as AccountsAll, instruction as InstructionAll, State as StateAccount,
    Topic as TopicAccount,
};

use crate::utils::pda::{get_state_pda, get_topic_pda};

/// Create a new topic
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
        authority: program.payer(),
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

/// List all topics
pub fn cmd_list_topics(program: &Program<Rc<Keypair>>) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);

    // Get the state to determine the number of topics
    let state_data: StateAccount = program.account(state_pda)?;
    let topic_count = state_data.topic_count;

    if topic_count == 0 {
        println!("No topics found");
        return Ok(());
    }

    println!("Found {} topics:", topic_count);

    for i in 0..topic_count {
        let (topic_pda, _) = get_topic_pda(program, i);

        match program.account::<TopicAccount>(topic_pda) {
            Ok(topic) => {
                println!("\nTopic #{}", i);
                println!("Name: {}", topic.name);
                println!("Description: {}", topic.description);
                println!("Active: {}", topic.is_active);
                println!("Submissions: {}", topic.submission_count);
            }
            Err(e) => {
                println!("Error fetching topic #{}: {}", i, e);
            }
        }
    }

    Ok(())
}

/// View a specific topic
pub fn cmd_view_topic(program: &Program<Rc<Keypair>>, id: u64) -> Result<()> {
    let (topic_pda, _) = get_topic_pda(program, id);

    match program.account::<TopicAccount>(topic_pda) {
        Ok(topic) => {
            println!("Topic #{} ({})", id, topic_pda);
            println!("Name: {}", topic.name);
            println!("Description: {}", topic.description);
            println!("Authority: {}", topic.authority);
            println!("Active: {}", topic.is_active);
            println!("Submissions: {}", topic.submission_count);
            println!(
                "Commit phase duration: {} seconds",
                topic.commit_phase_duration
            );
            println!(
                "Reveal phase duration: {} seconds",
                topic.reveal_phase_duration
            );
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("Topic not found: {}", e)),
    }
}
