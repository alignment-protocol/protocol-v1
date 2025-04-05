use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::{
    solana_sdk::{pubkey::Pubkey, system_program, sysvar},
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
    get_user_profile_pda, get_user_temp_token_account_pda, get_user_topic_balance_pda,
};

/// Submit data to a topic
pub fn cmd_submit_data_to_topic(
    program: &Program<Rc<Keypair>>,
    topic_id: u64,
    data_reference: String,
) -> Result<()> {
    let contributor = program.payer();
    let (state_pda, _) = get_state_pda(program);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (user_topic_balance_pda, _) = get_user_topic_balance_pda(program, &contributor, &topic_pda);

    match program.rpc().get_account(&user_topic_balance_pda) {
        Ok(_) => {
            println!(
                "UserTopicBalance account found ({}). Proceeding with submission.",
                user_topic_balance_pda
            );
        }
        Err(e) => {
            eprintln!(
                "\nError: User balance account for this topic (PDA: {}) not found.",
                user_topic_balance_pda
            );
            eprintln!(
                "This usually means the user hasn't interacted with this specific topic yet."
            );
            eprintln!("Please run 'alignment-protocol-cli user initialize-topic-balance --topic-id {}' first.", topic_id);
            return Err(anyhow::anyhow!(
                "UserTopicBalance account not initialized: {}",
                e
            ));
        }
    }

    let state_data: StateAccount = program.account(state_pda)?;

    let (contributor_profile_pda, _) = get_user_profile_pda(program, &contributor);
    let contributor_profile_data: alignment_protocol::UserProfile =
        match program.account(contributor_profile_pda) {
            Ok(profile) => profile,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Could not fetch user profile {}: {}. Has the user run 'create-profile'?",
                    contributor_profile_pda,
                    e
                ));
            }
        };
    let current_submission_index = contributor_profile_data.user_submission_count;

    let (submission_pda, _) = Pubkey::find_program_address(
        &[
            b"submission",
            contributor.as_ref(),
            &current_submission_index.to_le_bytes(),
        ],
        &program.id(),
    );
    let submission_id_for_print = current_submission_index;

    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);

    let temp_align_mint = state_data.temp_align_mint;

    let (contributor_temp_align_account_pda, _) =
        get_user_temp_token_account_pda(program, &contributor, "user_temp_align");

    let temp_align_account_exists = program
        .rpc()
        .get_account(&contributor_temp_align_account_pda)
        .is_ok();

    if !temp_align_account_exists {
        return Err(anyhow::anyhow!(
            "Temp align token account not found for '{contributor}'. Has the user run 'create-profile'?"
        ));
    }

    println!(
        "Submitting data to topic #{} using user submission index {}",
        topic_id, current_submission_index
    );
    println!("Data reference: {}", data_reference);

    let accounts = AccountsAll::SubmitDataToTopic {
        contributor,
        topic: topic_pda,
        submission: submission_pda,
        submission_topic_link: submission_topic_link_pda,
        state: state_pda,
        temp_align_mint,
        contributor_temp_align_account: contributor_temp_align_account_pda,
        contributor_profile: contributor_profile_pda,
        user_topic_balance: user_topic_balance_pda,
        token_program: anchor_spl::token::ID,
        system_program: system_program::ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::SubmitDataToTopic {
            data_reference,
            current_submission_index,
        })
        .send()?;

    println!("Data submitted successfully (txSig: {})", tx_sig);
    println!(
        "User-Specific Submission Index Used: {}",
        submission_id_for_print
    );
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
    let contributor = program.payer();
    let (state_pda, _) = get_state_pda(program);
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);

    let submission_data: SubmissionAccount = program.account(submission_pda)?;
    let original_contributor = submission_data.contributor;

    let (contributor_profile_pda, _) = get_user_profile_pda(program, &original_contributor);

    let (user_topic_balance_pda, _) =
        get_user_topic_balance_pda(program, &original_contributor, &topic_pda);

    let state_data: StateAccount = program.account(state_pda)?;
    let temp_align_mint = state_data.temp_align_mint;
    let align_mint = state_data.align_mint;

    let (contributor_temp_align_account_pda, _) =
        get_user_temp_token_account_pda(program, &original_contributor, "user_temp_align");
    let contributor_align_ata = get_token_ata(&original_contributor, &align_mint);

    println!(
        "Finalizing submission #{} in topic #{}",
        submission_id, topic_id
    );

    let accounts = AccountsAll::FinalizeSubmission {
        authority: contributor,
        state: state_pda,
        topic: topic_pda,
        submission: submission_pda,
        submission_topic_link: submission_topic_link_pda,
        contributor_profile: contributor_profile_pda,
        contributor_temp_align_account: contributor_temp_align_account_pda,
        contributor_align_ata,
        user_topic_balance: user_topic_balance_pda,
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
