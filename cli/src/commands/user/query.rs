use anchor_client::solana_sdk::{pubkey::Pubkey, signature::Keypair};
use anchor_client::Program;
use anyhow::Result;
use std::rc::Rc;
use std::str::FromStr;

use alignment_protocol::{
    State as StateAccount, Submission as SubmissionAccount,
    SubmissionTopicLink as SubmissionTopicLinkAccount, UserProfile as UserProfileAccount,
    UserTopicBalance as UserTopicBalanceAccount, VoteCommit as VoteCommitAccount,
};

use crate::commands::common::pda::{
    get_state_pda, get_submission_topic_link_pda, get_topic_pda, get_user_profile_pda,
    get_user_topic_balance_pda, get_vote_commit_pda,
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
pub fn cmd_query_submission(program: &Program<Rc<Keypair>>, pda_str: String) -> Result<()> {
    let submission_pda = Pubkey::from_str(&pda_str)
        .map_err(|e| anyhow::anyhow!("Invalid Submission PDA format: {}", e))?;

    match program.account::<SubmissionAccount>(submission_pda) {
        Ok(submission) => {
            println!("Submission ({})", submission_pda);
            println!("  Contributor: {}", submission.contributor);
            println!("  Timestamp: {}", submission.timestamp);
            println!("  Data Reference: {}", submission.data_reference);
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("Submission not found: {}", e)),
    }
}

/// Query submissions by a contributor, optionally filtered by topic
pub fn cmd_query_submissions(
    program: &Program<Rc<Keypair>>,
    by: String,
    topic: Option<u64>,
) -> Result<()> {
    let contributor_pubkey = Pubkey::from_str(&by)?;
    println!(
        "Querying submissions by Contributor: {}",
        contributor_pubkey
    );

    // Fetch the user profile to get the submission count
    let (contributor_profile_pda, _) = get_user_profile_pda(program, &contributor_pubkey);
    let contributor_profile: UserProfileAccount = match program.account(contributor_profile_pda) {
        Ok(profile) => profile,
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Could not fetch user profile {}: {}. Has the user submitted data?",
                contributor_profile_pda,
                e
            ));
        }
    };

    let submission_count = contributor_profile.user_submission_count;

    if submission_count == 0 {
        println!("No submissions found for this contributor.");
        return Ok(());
    }

    println!(
        "Contributor {} has {} total submissions. Checking filters...",
        contributor_pubkey, submission_count
    );

    // Get topic PDA if topic filter is provided
    let topic_pda_filter = match topic {
        Some(topic_index) => {
            let (topic_pda, _) = get_topic_pda(program, topic_index);
            println!(
                "Filtering for topic index {} (PDA: {})",
                topic_index, topic_pda
            );
            Some(topic_pda)
        }
        None => None,
    };

    let mut matched_count = 0;

    // Iterate through all submission indices for this user
    for i in 0..submission_count {
        // Derive submission PDA using contributor key and index
        let (submission_pda, _) = Pubkey::find_program_address(
            &[b"submission", contributor_pubkey.as_ref(), &i.to_le_bytes()],
            &program.id(),
        );

        // Fetch the submission account data
        let submission_data: SubmissionAccount = match program.account(submission_pda) {
            Ok(data) => data,
            Err(e) => {
                // This might happen if accounts were created out of order or inconsistently
                println!(
                    "Warning: Failed to fetch submission account at index {}: {}. PDA: {}",
                    i, e, submission_pda
                );
                continue;
            }
        };

        // If topic filter is specified, check submission-topic links
        if let Some(topic_pda) = topic_pda_filter {
            let (submission_topic_link_pda, _) =
                get_submission_topic_link_pda(program, &submission_pda, &topic_pda);

            // Check if the submission-topic link exists by attempting to fetch it
            match program.account::<SubmissionTopicLinkAccount>(submission_topic_link_pda) {
                Ok(_) => {
                    // Link exists, continue to print
                }
                Err(_) => {
                    // Link doesn't exist for this topic, skip this submission
                    continue;
                }
            }
        }

        // If we reach here, the submission matches all filters
        matched_count += 1;
        println!("\nSubmission Index: {}", i); // Show the user-specific index
        println!("  PDA: {}", submission_pda);
        println!("  Contributor: {}", submission_data.contributor);
        println!("  Timestamp: {}", submission_data.timestamp);
        println!("  Data Reference: {}", submission_data.data_reference);
    }

    println!(
        "\nDisplayed {} submissions for contributor {} matching the criteria",
        matched_count, contributor_pubkey
    );

    Ok(())
}

/// Query details about a submission in a specific topic
pub fn cmd_query_submission_topic(
    program: &Program<Rc<Keypair>>,
    submission_pda_str: String,
    topic_index: u64,
) -> Result<()> {
    let submission_pda = Pubkey::from_str(&submission_pda_str)
        .map_err(|e| anyhow::anyhow!("Invalid Submission PDA format: {}", e))?;
    let (topic_pda, _) = get_topic_pda(program, topic_index);
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);

    match program.account::<SubmissionTopicLinkAccount>(submission_topic_link_pda) {
        Ok(link) => {
            println!("Submission {} in Topic #{}", submission_pda, topic_index);
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
    submission_pda_str: String,
    topic_index: u64,
    validator_str: Option<String>,
) -> Result<()> {
    let submission_pda = Pubkey::from_str(&submission_pda_str)
        .map_err(|e| anyhow::anyhow!("Invalid Submission PDA format: {}", e))?;
    let validator = match validator_str {
        Some(pubkey_str) => Pubkey::from_str(&pubkey_str)?,
        None => program.payer(),
    };

    let (topic_pda, _) = get_topic_pda(program, topic_index);
    let (submission_topic_link_pda, _) =
        get_submission_topic_link_pda(program, &submission_pda, &topic_pda);
    let (vote_commit_pda, _) = get_vote_commit_pda(program, &submission_topic_link_pda, &validator);

    match program.account::<VoteCommitAccount>(vote_commit_pda) {
        Ok(vote) => {
            println!(
                "Vote by {} on Submission {} in Topic #{}",
                validator, submission_pda, topic_index
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

/// Query user balance for a specific topic
pub fn cmd_view_user_topic_balance(
    program: &Program<Rc<Keypair>>,
    topic_index: u64,
    user_str: Option<String>,
) -> Result<()> {
    let user = match user_str {
        Some(pubkey_str) => Pubkey::from_str(&pubkey_str)?,
        None => program.payer(),
    };

    let (topic_pda, _) = get_topic_pda(program, topic_index);
    let (user_topic_balance_pda, _) = get_user_topic_balance_pda(program, &user, &topic_pda);

    println!(
        "Querying balance for User: {} on Topic index: {}",
        user, topic_index
    );
    println!("Topic PDA: {}", topic_pda);
    println!("UserTopicBalance PDA: {}", user_topic_balance_pda);

    match program.account::<UserTopicBalanceAccount>(user_topic_balance_pda) {
        Ok(balance) => {
            println!("\nBalance Found:");
            println!("  User: {}", balance.user);
            println!("  Topic: {}", balance.topic);
            println!("  Temp Align Amount: {}", balance.temp_align_amount);
            println!("  Temp Rep Amount: {}", balance.temp_rep_amount);
            println!(
                "  Locked Temp Rep Amount: {}",
                balance.locked_temp_rep_amount
            );
        }
        Err(e) => {
            if e.to_string().contains("AccountNotFound")
                || e.to_string().contains("Could not deserialize account data")
            {
                println!("\nNo balance record found for this user/topic combination.");
                println!("This usually means the user hasn't interacted with this specific topic yet (e.g., submitted, staked, or voted).");
            } else {
                println!("\nError fetching topic balance account: {}", e);
            }
        }
    }

    Ok(())
}
