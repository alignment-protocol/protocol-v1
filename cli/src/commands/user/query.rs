use anchor_client::solana_sdk::{pubkey::Pubkey, signature::Keypair};
use anchor_client::Program;
use anyhow::Result;
use std::rc::Rc;
use std::str::FromStr;

use alignment_protocol::{
    State as StateAccount, Submission as SubmissionAccount,
    SubmissionTopicLink as SubmissionTopicLinkAccount, VoteCommit as VoteCommitAccount,
};

use crate::commands::common::pda::{
    get_state_pda, get_submission_pda, get_submission_topic_link_pda, get_topic_pda,
    get_vote_commit_pda,
};
use crate::commands::common::time::get_current_timestamp;

/// Query state account
pub fn cmd_query_state(program: &Program<Rc<Keypair>>) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);

    match program.account::<StateAccount>(state_pda) {
        Ok(state) => {
            println!("State Account ({})", state_pda);
            println!("Authority: {}", state.authority);
            println!("Temp Align Mint: {}", state.temp_align_mint);
            println!("Align Mint: {}", state.align_mint);
            println!("Temp Rep Mint: {}", state.temp_rep_mint);
            println!("Rep Mint: {}", state.rep_mint);
            println!("Submission Count: {}", state.submission_count);
            println!("Topic Count: {}", state.topic_count);
            println!("Tokens to Mint: {}", state.tokens_to_mint);
            println!(
                "Default Commit Phase Duration: {} seconds",
                state.default_commit_phase_duration
            );
            println!(
                "Default Reveal Phase Duration: {} seconds",
                state.default_reveal_phase_duration
            );
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!(
            "State account not found: {}. Initialize it first.",
            e
        )),
    }
}

/// Query a specific submission
pub fn cmd_query_submission(program: &Program<Rc<Keypair>>, id: u64) -> Result<()> {
    let (submission_pda, _) = get_submission_pda(program, id);

    match program.account::<SubmissionAccount>(submission_pda) {
        Ok(submission) => {
            println!("Submission #{} ({})", id, submission_pda);
            println!("Contributor: {}", submission.contributor);
            println!("Timestamp: {}", submission.timestamp);
            println!("Data Reference: {}", submission.data_reference);
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("Submission not found: {}", e)),
    }
}

/// Query all submissions
pub fn cmd_query_submissions(
    program: &Program<Rc<Keypair>>,
    by: Option<String>,
    topic: Option<u64>,
) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);

    // Get state data to find total submission count
    let state_data: StateAccount = program.account(state_pda)?;
    let submission_count = state_data.submission_count;

    if submission_count == 0 {
        println!("No submissions found.");
        return Ok(());
    }

    println!("Total submissions in protocol: {}", submission_count);

    // Parse contributor pubkey if provided
    let contributor_filter = match by {
        Some(pubkey_str) => Some(Pubkey::from_str(&pubkey_str)?),
        None => None,
    };

    // Get topic PDA if topic filter is provided
    let topic_pda_filter = match topic {
        Some(id) => {
            let (topic_pda, _) = get_topic_pda(program, id);
            Some(topic_pda)
        }
        None => None,
    };

    // Print filter info
    if let Some(pubkey) = contributor_filter {
        println!("Filtering for contributor: {}", pubkey);
    }

    if let Some(id) = topic {
        println!("Filtering for topic: {}", id);
    }

    let mut matched_count = 0;

    // Iterate through all submission indices
    for i in 0..submission_count {
        let (submission_pda, _) = get_submission_pda(program, i);

        // Fetch the submission account data
        let submission_data: SubmissionAccount = match program.account(submission_pda) {
            Ok(data) => data,
            Err(e) => {
                println!("Warning: Failed to fetch submission #{}: {}", i, e);
                continue;
            }
        };

        // If contributor filter is specified, skip non-matching submissions
        if let Some(pubkey) = contributor_filter {
            if submission_data.contributor != pubkey {
                continue;
            }
        }

        // If topic filter is specified, check submission-topic links
        if let Some(topic_pda) = topic_pda_filter {
            let (submission_topic_link_pda, _) =
                get_submission_topic_link_pda(program, &submission_pda, &topic_pda);

            // Skip if the submission-topic link doesn't exist
            if program
                .rpc()
                .get_account(&submission_topic_link_pda)
                .is_err()
            {
                continue;
            }
        }

        matched_count += 1;

        println!("\nSubmission #{}", i);
        println!("PDA: {}", submission_pda);
        println!("Contributor: {}", submission_data.contributor);
        println!("Timestamp: {}", submission_data.timestamp);
        println!("Data Reference: {}", submission_data.data_reference);
    }

    println!(
        "\nDisplayed {} submissions matching the criteria",
        matched_count
    );

    Ok(())
}

/// Query details about a submission in a specific topic
pub fn cmd_query_submission_topic(
    program: &Program<Rc<Keypair>>,
    submission_id: u64,
    topic_id: u64,
) -> Result<()> {
    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);

    match program.account::<SubmissionTopicLinkAccount>(submission_topic_link_pda) {
        Ok(link) => {
            println!("Submission #{} in Topic #{}", submission_id, topic_id);
            println!("Link PDA: {}", submission_topic_link_pda);
            println!("Status: {:?}", link.status);
            println!("\nVoting Phases:");
            println!(
                "Commit Phase: {} to {}",
                link.commit_phase_start, link.commit_phase_end
            );
            println!(
                "Reveal Phase: {} to {}",
                link.reveal_phase_start, link.reveal_phase_end
            );

            let current_time = get_current_timestamp();
            if current_time < link.commit_phase_start {
                println!("Voting not started yet");
            } else if current_time < link.commit_phase_end {
                println!("Currently in commit phase");
            } else if current_time < link.reveal_phase_start {
                println!("Commit phase ended, waiting for reveal phase");
            } else if current_time < link.reveal_phase_end {
                println!("Currently in reveal phase");
            } else {
                println!("Voting ended");
            }

            println!("\nVoting Statistics:");
            println!("Yes Voting Power: {}", link.yes_voting_power);
            println!("No Voting Power: {}", link.no_voting_power);
            println!("Total Committed Votes: {}", link.total_committed_votes);
            println!("Total Revealed Votes: {}", link.total_revealed_votes);

            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("Submission-topic link not found: {}", e)),
    }
}

/// Query vote information
pub fn cmd_query_vote(
    program: &Program<Rc<Keypair>>,
    submission_id: u64,
    topic_id: u64,
    validator_str: Option<String>,
) -> Result<()> {
    let validator = match validator_str {
        Some(pubkey_str) => Pubkey::from_str(&pubkey_str)?,
        None => program.payer(),
    };

    let (submission_pda, _) = get_submission_pda(program, submission_id);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    let (vote_commit_pda, _) = get_vote_commit_pda(program, &submission_topic_link_pda, &validator);

    match program.account::<VoteCommitAccount>(vote_commit_pda) {
        Ok(vote) => {
            println!(
                "Vote by {} on Submission #{} in Topic #{}",
                validator, submission_id, topic_id
            );
            println!("Vote Commit PDA: {}", vote_commit_pda);
            println!("Vote Hash: {:?}", vote.vote_hash);
            println!("Commit Timestamp: {}", vote.commit_timestamp);
            println!("Vote Amount: {}", vote.vote_amount);
            println!(
                "Using {} reputation",
                if vote.is_permanent_rep {
                    "permanent"
                } else {
                    "temporary"
                }
            );
            println!("Revealed: {}", vote.revealed);
            println!("Finalized: {}", vote.finalized);

            if vote.revealed {
                println!("Vote Choice: {:?}", vote.vote_choice);
            }

            Ok(())
        }
        Err(e) => {
            println!("Vote not found: {}", e);
            println!("The validator may not have voted on this submission.");
            Ok(())
        }
    }
}
