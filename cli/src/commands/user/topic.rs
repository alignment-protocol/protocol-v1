use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use anyhow::Result;
use std::rc::Rc;

use alignment_protocol::{
    accounts as AccountsAll, instruction as InstructionAll, State as StateAccount,
    Topic as TopicAccount,
};

use crate::commands::common::pda::{
    get_state_pda, get_topic_pda, get_user_profile_pda, get_user_topic_balance_pda,
};

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
                println!("  PDA: {}", topic_pda);
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
pub fn cmd_view_topic(program: &Program<Rc<Keypair>>, topic_index: u64) -> Result<()> {
    let (topic_pda, _) = get_topic_pda(program, topic_index);

    match program.account::<TopicAccount>(topic_pda) {
        Ok(topic) => {
            println!("Topic #{} ({})", topic_index, topic_pda);
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

/// Initialize UserTopicBalance account for the payer and a specific topic
pub fn cmd_initialize_user_topic_balance(
    program: &Program<Rc<Keypair>>,
    topic_index: u64,
) -> Result<()> {
    let user = program.payer();
    let (user_profile_pda, _) = get_user_profile_pda(program, &user);
    let (topic_pda, _) = get_topic_pda(program, topic_index);
    let (user_topic_balance_pda, _) = get_user_topic_balance_pda(program, &user, &topic_pda);

    // Check if user profile exists first (optional but good UX)
    if program.rpc().get_account(&user_profile_pda).is_err() {
        return Err(anyhow::anyhow!(
            "User profile {} not found. Run 'user create-profile' first.",
            user_profile_pda
        ));
    }

    // Check if topic exists (optional but good UX)
    if program.rpc().get_account(&topic_pda).is_err() {
        return Err(anyhow::anyhow!(
            "Topic with index {} (PDA: {}) not found.",
            topic_index,
            topic_pda
        ));
    }

    // Check if balance account *already* exists (optional but good UX)
    if program.rpc().get_account(&user_topic_balance_pda).is_ok() {
        println!(
            "UserTopicBalance account {} already exists for topic {}. No action needed.",
            user_topic_balance_pda, topic_index
        );
        return Ok(());
    }

    println!(
        "Initializing UserTopicBalance for user {} and topic index {}",
        user, topic_index
    );
    println!("  User Profile PDA: {}", user_profile_pda);
    println!("  Topic PDA: {}", topic_pda);
    println!("  UserTopicBalance PDA: {}", user_topic_balance_pda);

    let accounts = AccountsAll::InitializeUserTopicBalance {
        user,
        user_profile: user_profile_pda,
        topic: topic_pda,
        user_topic_balance: user_topic_balance_pda,
        system_program: anchor_client::solana_sdk::system_program::ID, // Use fully qualified path
        rent: anchor_client::solana_sdk::sysvar::rent::ID,             // Use fully qualified path
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeUserTopicBalance {})
        .send()?;

    println!(
        "UserTopicBalance initialized successfully (txSig: {})",
        tx_sig
    );

    Ok(())
}

/// Create a new topic (open to any wallet / fee‑payer)
pub fn cmd_create_topic(
    program: &Program<Rc<Keypair>>,
    name: String,
    description: String,
    commit_duration: Option<u64>,
    reveal_duration: Option<u64>,
) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);

    // Fetch current state to determine the next
    let state_data: StateAccount = program.account(state_pda)?;
    let topic_index = state_data.topic_count;

    // Derive the PDA for the new topic
    let (topic_pda, _) = get_topic_pda(program, topic_index);

    println!("Creating new topic with index {}", topic_index);
    println!("Name: {}", name);
    println!("Description: {}", description);

    // Build account metas
    let accounts = AccountsAll::CreateTopic {
        creator: program.payer(), // the CLI payer is the creator & fee‑payer
        state: state_pda,
        topic: topic_pda,
        system_program: anchor_client::solana_sdk::system_program::ID,
        rent: anchor_client::solana_sdk::sysvar::rent::ID,
    };

    // Send transaction
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
    println!("Topic index: {}", topic_index);
    println!("Topic PDA: {}", topic_pda);

    Ok(())
}

/// Update an existing topic's settings (durations / active flag)
pub fn cmd_update_topic(
    program: &Program<Rc<Keypair>>,
    topic_index: u64,
    commit_duration: Option<u64>,
    reveal_duration: Option<u64>,
    active: Option<bool>,
) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let (topic_pda, _) = get_topic_pda(program, topic_index);

    println!("Updating topic #{} (PDA: {})", topic_index, topic_pda);

    if commit_duration.is_none() && reveal_duration.is_none() && active.is_none() {
        println!("Nothing to update – provide at least one --commit-duration, --reveal-duration or --active flag");
        return Ok(());
    }

    let accounts = AccountsAll::UpdateTopic {
        authority: program.payer(),
        state: state_pda,
        topic: topic_pda,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::UpdateTopic {
            commit_phase_duration: commit_duration,
            reveal_phase_duration: reveal_duration,
            is_active: active,
        })
        .send()?;

    println!("Topic updated successfully (txSig: {})", tx_sig);
    Ok(())
}
