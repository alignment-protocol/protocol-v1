use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::{
    solana_sdk::{system_program, sysvar},
    Program,
};
use anyhow::Result;
use std::rc::Rc;

use alignment_protocol::{
    accounts as AccountsAll, instruction as InstructionAll, State as StateAccount,
    Submission as SubmissionAccount,
};

use crate::commands::common::pda::{
    get_state_pda, get_submission_pda, get_submission_topic_link_pda, get_token_ata, get_topic_pda,
    get_user_profile_pda, get_user_temp_token_account_pda,
};

/// Submit data to a topic
pub fn cmd_submit_data_to_topic(
    program: &Program<Rc<Keypair>>,
    topic_id: u64,
    data_reference: String,
) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let (topic_pda, _) = get_topic_pda(program, topic_id);

    // Get the state to determine the next submission ID
    let state_data: StateAccount = program.account(state_pda)?;
    let submission_id = state_data.submission_count;

    let contributor = program.payer();
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);

    // Get temp align mint
    let temp_align_mint = state_data.temp_align_mint;

    // Get user's temp align account
    let (contributor_temp_align_account_pda, _) =
        get_user_temp_token_account_pda(program, &contributor, "temp_align_account");

    // Get user profile PDA
    let (contributor_profile_pda, _) = get_user_profile_pda(program, &contributor);

    // Check if user profile exists
    let profile_exists = program.rpc().get_account(&contributor_profile_pda).is_ok();

    // Check if temp align account exists
    let temp_align_account_exists = program
        .rpc()
        .get_account(&contributor_temp_align_account_pda)
        .is_ok();

    if !profile_exists || !temp_align_account_exists {
        return Err(anyhow::anyhow!(
            "User profile or token accounts not set up. Please run 'alignment-protocol-cli user create-profile' first."
        ));
    }

    println!("Submitting data to topic #{}", topic_id);
    println!("Data reference: {}", data_reference);

    let accounts = if profile_exists {
        AccountsAll::SubmitDataToTopic {
            contributor,
            topic: topic_pda,
            submission: submission_pda,
            submission_topic_link: submission_topic_link_pda,
            state: state_pda,
            temp_align_mint,
            contributor_temp_align_account: contributor_temp_align_account_pda,
            contributor_profile: Some(contributor_profile_pda),
            token_program: anchor_spl::token::ID,
            system_program: system_program::ID,
            rent: sysvar::rent::ID,
        }
    } else {
        AccountsAll::SubmitDataToTopic {
            contributor,
            topic: topic_pda,
            submission: submission_pda,
            submission_topic_link: submission_topic_link_pda,
            state: state_pda,
            temp_align_mint,
            contributor_temp_align_account: contributor_temp_align_account_pda,
            contributor_profile: None,
            token_program: anchor_spl::token::ID,
            system_program: system_program::ID,
            rent: sysvar::rent::ID,
        }
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::SubmitDataToTopic { data_reference })
        .send()?;

    println!("Data submitted successfully (txSig: {})", tx_sig);
    println!("Submission ID: {}", submission_id);
    println!("Submission PDA: {}", submission_pda);
    println!("Submission-Topic Link PDA: {}", submission_topic_link_pda);
    Ok(())
}

/// Link an existing submission to another topic
pub fn cmd_link_submission_to_topic(
    program: &Program<Rc<Keypair>>,
    submission_id: u64,
    topic_id: u64,
) -> Result<()> {
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);

    println!(
        "Linking submission #{} to topic #{}",
        submission_id, topic_id
    );

    let (state_pda, _) = get_state_pda(program);

    let accounts = AccountsAll::LinkSubmissionToTopic {
        authority: program.payer(),
        state: state_pda,
        submission: submission_pda,
        topic: topic_pda,
        submission_topic_link: submission_topic_link_pda,
        system_program: system_program::ID,
        rent: sysvar::rent::ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::LinkSubmissionToTopic {})
        .send()?;

    println!("Submission linked successfully (txSig: {})", tx_sig);
    println!("Submission-Topic Link PDA: {}", submission_topic_link_pda);
    Ok(())
}

/// Finalize a submission after voting
pub fn cmd_finalize_submission(
    program: &Program<Rc<Keypair>>,
    submission_id: u64,
    topic_id: u64,
) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);

    // Get submission data
    let submission_data: SubmissionAccount = program.account(submission_pda)?;
    let contributor = submission_data.contributor;

    // Get user profile PDA
    let (contributor_profile_pda, _) = get_user_profile_pda(program, &contributor);

    // Get mint addresses from state
    let state_data: StateAccount = program.account(state_pda)?;
    let temp_align_mint = state_data.temp_align_mint;
    let align_mint = state_data.align_mint;

    // Get token accounts
    let (contributor_temp_align_account_pda, _) =
        get_user_temp_token_account_pda(program, &contributor, "temp_align_account");
    let contributor_align_ata = get_token_ata(&contributor, &align_mint);

    println!(
        "Finalizing submission #{} in topic #{}",
        submission_id, topic_id
    );

    let accounts = AccountsAll::FinalizeSubmission {
        authority: program.payer(),
        state: state_pda,
        topic: topic_pda,
        submission: submission_pda,
        submission_topic_link: submission_topic_link_pda,
        contributor_profile: contributor_profile_pda,
        contributor_temp_align_account: contributor_temp_align_account_pda,
        contributor_align_ata,
        temp_align_mint,
        align_mint,
        token_program: anchor_spl::token::ID,
        system_program: system_program::ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::FinalizeSubmission {})
        .send()?;

    println!("Submission finalized successfully (txSig: {})", tx_sig);
    Ok(())
}
