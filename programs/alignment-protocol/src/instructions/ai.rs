use crate::contexts::{RequestAiValidation, SubmitAiVote};
use crate::data::{AiValidationStatus, SubmissionStatus, VoteChoice};
use crate::error::ErrorCode;
use crate::helpers::calculate_quadratic_voting_power; // Use existing helper
use anchor_lang::prelude::*;

pub fn request_ai_validation(
    ctx: Context<RequestAiValidation>,
    temp_rep_to_stake: u64, // Amount of tempRep user commits
) -> Result<()> {
    let clock = Clock::get()?;
    let current_timestamp_u64 = clock.unix_timestamp as u64; // Cast to u64
    let requester = &ctx.accounts.requester;
    let submission = &ctx.accounts.submission;
    let link = &ctx.accounts.submission_topic_link;
    let user_balance = &mut ctx.accounts.user_topic_balance;
    let ai_request = &mut ctx.accounts.ai_validation_request;

    // Validation Checks:
    // 1. Requester must be the original contributor of the submission
    require_keys_eq!(
        submission.contributor,
        requester.key(),
        ErrorCode::NotSubmissionContributor // Keeping specific error
    );

    // 2. Submission must be in Pending state for this topic link
    require!(
        link.status == SubmissionStatus::Pending,
        ErrorCode::SubmissionNotPending // Using existing error
    );

    // 3. User must have enough *available* tempRep in this topic balance
    require!(
        user_balance.temp_rep_amount >= temp_rep_to_stake,
        ErrorCode::InsufficientTempRepBalance // Keeping specific error
    );

    // 4. Can't request AI validation if voting period is over (reveal phase ended)
    require!(
        current_timestamp_u64 <= link.reveal_phase_end, // Compare u64 with u64
        ErrorCode::RevealPhaseEnded                     // Using existing error
    );

    // 5. Ensure an AI validation hasn't already been requested/completed for this link
    // (The `init` constraint on ai_validation_request account in the context handles this -
    // if the account already exists from a previous request, the transaction will fail)

    // 6. Basic sanity check for stake amount
    require!(temp_rep_to_stake > 0, ErrorCode::ZeroVoteAmount); // Using existing error

    // Logic:
    // 1. Deduct tempRep cost from available balance
    user_balance.temp_rep_amount = user_balance
        .temp_rep_amount
        .checked_sub(temp_rep_to_stake)
        .ok_or(ErrorCode::Overflow)?; // Using existing error

    // 2. Initialize the AiValidationRequest account
    ai_request.submission_topic_link = link.key();
    ai_request.requester = requester.key();
    ai_request.temp_rep_staked = temp_rep_to_stake; // Record the amount staked/spent
    ai_request.request_timestamp = current_timestamp_u64; // Assign u64
    ai_request.status = AiValidationStatus::Pending; // Initial status for the oracle to pick up
    ai_request.ai_decision = None;
    ai_request.ai_voting_power = 0;
    ai_request.bump = ctx.bumps.ai_validation_request;

    msg!(
        "AI Validation requested for link {} by user {}. Staked/Spent {} tempRep.",
        link.key(),
        requester.key(),
        temp_rep_to_stake
    );

    Ok(())
}

pub fn submit_ai_vote(
    ctx: Context<SubmitAiVote>,
    ai_decision: VoteChoice, // The decision from the AI (Yes/No)
) -> Result<()> {
    let clock = Clock::get()?;
    let current_timestamp_u64 = clock.unix_timestamp as u64; // Cast to u64
    let oracle = &ctx.accounts.oracle;
    let state = &ctx.accounts.state;
    let ai_request = &mut ctx.accounts.ai_validation_request;
    let link = &mut ctx.accounts.submission_topic_link;

    // Validation Checks:
    // 1. Signer must be the authorized Oracle stored in the global state
    require_keys_eq!(
        oracle.key(),
        state.oracle_pubkey,
        ErrorCode::UnauthorizedOracle // Keeping specific error
    );

    // 2. AI Request must be in Pending state (waiting for oracle processing)
    require!(
        ai_request.status == AiValidationStatus::Pending,
        ErrorCode::InvalidAiRequestStatus // Keeping specific error
    );

    // 3. AI Request must belong to the SubmissionTopicLink being processed
    require_keys_eq!(
        ai_request.submission_topic_link,
        link.key(),
        ErrorCode::MismatchedAiRequestLink // Keeping specific error
    );

    // 4. Voting period must still be active (reveal phase hasn't ended)
    require!(
        current_timestamp_u64 <= link.reveal_phase_end, // Compare u64 with u64
        ErrorCode::RevealPhaseEnded                     // Using existing error
    );

    // Logic:
    // 1. Calculate quadratic voting power from the tempRep staked by the user
    let voting_power = calculate_quadratic_voting_power(ai_request.temp_rep_staked);

    // 2. Update SubmissionTopicLink vote counts with AI's power
    match ai_decision {
        VoteChoice::Yes => {
            link.yes_voting_power = link
                .yes_voting_power
                .checked_add(voting_power)
                .ok_or(ErrorCode::Overflow)?; // Using existing error
            msg!("AI voted Yes with power {}", voting_power);
        }
        VoteChoice::No => {
            link.no_voting_power = link
                .no_voting_power
                .checked_add(voting_power)
                .ok_or(ErrorCode::Overflow)?; // Using existing error
            msg!("AI voted No with power {}", voting_power);
        }
    }
    // Note: We are not incrementing total_committed_votes or total_revealed_votes
    // as the AI doesn't follow the commit/reveal scheme. Its power is added directly.

    // 3. Update AiValidationRequest status and details
    ai_request.status = AiValidationStatus::Completed;
    ai_request.ai_decision = Some(ai_decision);
    ai_request.ai_voting_power = voting_power;

    msg!(
        "AI Vote submitted for link {}. Decision: {:?}, Power: {}",
        link.key(),
        ai_decision,
        voting_power
    );

    Ok(())
}
