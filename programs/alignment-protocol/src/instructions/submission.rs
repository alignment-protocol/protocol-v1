use crate::contexts::{FinalizeSubmission, LinkSubmissionToTopic, SubmitDataToTopic};
use crate::data::{SubmissionStatus, MAX_DATA_REFERENCE_LENGTH};
use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, MintTo};

// Removed legacy submit_data function

/// Submit data to a specific topic, earn tempAlign, and update UserTopicBalance
pub fn submit_data_to_topic(
    ctx: Context<SubmitDataToTopic>,
    data_reference: String,
    current_submission_index: u64,
) -> Result<()> {
    // Validate inputs
    if data_reference.len() > MAX_DATA_REFERENCE_LENGTH {
        return Err(ErrorCode::DataReferenceTooLong.into());
    }
    if data_reference.is_empty() {
        return Err(ErrorCode::EmptyDataReference.into());
    }

    let state = &ctx.accounts.state;
    let topic = &mut ctx.accounts.topic;
    let submission = &mut ctx.accounts.submission;
    let submission_topic_link = &mut ctx.accounts.submission_topic_link;
    let contributor_profile = &mut ctx.accounts.contributor_profile;
    let user_topic_balance = &mut ctx.accounts.user_topic_balance;
    let clock = Clock::get()?;

    // Verify Submission Index
    require_eq!(
        contributor_profile.user_submission_count,
        current_submission_index,
        ErrorCode::IncorrectSubmissionIndex
    );

    // Check if topic is active
    if !topic.is_active {
        return Err(ErrorCode::TopicInactive.into());
    }

    // --- Initialize Submission Account ---
    submission.contributor = ctx.accounts.contributor.key();
    submission.timestamp = clock.unix_timestamp as u64;
    submission.data_reference = data_reference;
    submission.bump = ctx.bumps.submission;

    // --- Initialize SubmissionTopicLink Account ---
    submission_topic_link.submission = submission.key();
    submission_topic_link.topic = topic.key();
    submission_topic_link.status = SubmissionStatus::Pending;
    // Set phase start/end times based on topic defaults and current time
    submission_topic_link.commit_phase_start = clock.unix_timestamp as u64;
    submission_topic_link.commit_phase_end = submission_topic_link
        .commit_phase_start
        .checked_add(topic.commit_phase_duration)
        .ok_or(ErrorCode::Overflow)?;
    submission_topic_link.reveal_phase_start = submission_topic_link.commit_phase_end;
    submission_topic_link.reveal_phase_end = submission_topic_link
        .reveal_phase_start
        .checked_add(topic.reveal_phase_duration)
        .ok_or(ErrorCode::Overflow)?;
    // Initialize counters CORRECTLY based on data.rs definition
    submission_topic_link.yes_voting_power = 0;
    submission_topic_link.no_voting_power = 0;
    submission_topic_link.total_committed_votes = 0; // Correct field name
    submission_topic_link.total_revealed_votes = 0; // Correct field name
    submission_topic_link.bump = ctx.bumps.submission_topic_link;

    // --- Mint Temporary Alignment Tokens ---
    let tokens_to_mint = state.tokens_to_mint; // Ensure correct field name used if changed from tokens_to_mint_per_submission
    if tokens_to_mint > 0 {
        let state_bump = state.bump;
        let seeds = &[b"state".as_ref(), &[state_bump]];
        let signer = &[&seeds[..]];

        let mint_to_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.temp_align_mint.to_account_info(),
                to: ctx
                    .accounts
                    .contributor_temp_align_account
                    .to_account_info(),
                authority: state.to_account_info(),
            },
            signer,
        );
        token::mint_to(mint_to_ctx, tokens_to_mint)?;
    }

    // --- Update UserTopicBalance ---
    user_topic_balance.temp_align_amount = user_topic_balance
        .temp_align_amount
        .checked_add(tokens_to_mint)
        .ok_or(ErrorCode::Overflow)?;

    // --- Update Topic Counter ---
    topic.submission_count = topic
        .submission_count
        .checked_add(1)
        .ok_or(ErrorCode::Overflow)?;

    // --- Update User Submission Count ---
    contributor_profile.user_submission_count = contributor_profile
        .user_submission_count
        .checked_add(1)
        .ok_or(ErrorCode::Overflow)?;

    msg!(
        "User {} submitted data (index {}) to topic {}. Earned {} tempAlign.",
        ctx.accounts.contributor.key(),
        current_submission_index,
        topic.key(),
        tokens_to_mint
    );
    msg!(
        "Updated UserTopicBalance: tempAlign = {}",
        user_topic_balance.temp_align_amount
    );
    msg!(
        "User Profile Submission Count NOW: {}",
        contributor_profile.user_submission_count
    );

    Ok(())
}

pub fn link_submission_to_topic(ctx: Context<LinkSubmissionToTopic>) -> Result<()> {
    // Get current time
    let current_time = Clock::get()?.unix_timestamp as u64;

    // Fill out the SubmissionTopicLink account
    let link = &mut ctx.accounts.submission_topic_link;

    link.submission = ctx.accounts.submission.key();
    link.topic = ctx.accounts.topic.key();
    link.status = SubmissionStatus::Pending;
    link.bump = ctx.bumps.submission_topic_link;

    // Set up voting phases based on topic durations
    link.commit_phase_start = current_time;
    link.commit_phase_end = current_time
        .checked_add(ctx.accounts.topic.commit_phase_duration)
        .ok_or(ErrorCode::Overflow)?;
    link.reveal_phase_start = link.commit_phase_end;
    link.reveal_phase_end = link
        .reveal_phase_start
        .checked_add(ctx.accounts.topic.reveal_phase_duration)
        .ok_or(ErrorCode::Overflow)?;

    // Initialize vote counts
    link.yes_voting_power = 0;
    link.no_voting_power = 0;
    link.total_committed_votes = 0;
    link.total_revealed_votes = 0;

    // Increment the topic's submission count
    let topic = &mut ctx.accounts.topic;
    topic.submission_count = topic
        .submission_count
        .checked_add(1)
        .ok_or(ErrorCode::Overflow)?;

    msg!("Linked existing submission to topic '{}'", topic.name);
    msg!(
        "Commit phase: {} to {}",
        link.commit_phase_start,
        link.commit_phase_end
    );
    msg!(
        "Reveal phase: {} to {}",
        link.reveal_phase_start,
        link.reveal_phase_end
    );

    Ok(())
}

pub fn finalize_submission(ctx: Context<FinalizeSubmission>) -> Result<()> {
    let link = &mut ctx.accounts.submission_topic_link;

    // Determine if the submission is accepted or rejected
    let is_accepted = link.yes_voting_power > link.no_voting_power;

    // Update submission status in the link
    if is_accepted {
        link.status = SubmissionStatus::Accepted;

        // --- Token Conversion Logic ---
        // Get the amount of tempAlign potentially eligible for conversion from UserTopicBalance
        let topic_align_balance = ctx.accounts.user_topic_balance.temp_align_amount;

        // Determine conversion amount - use the balance from UserTopicBalance
        // Cap it at the max mintable amount (state.tokens_to_mint) if needed.
        let conversion_amount =
            std::cmp::min(ctx.accounts.state.tokens_to_mint, topic_align_balance);

        // Check if there are any tokens to convert
        if conversion_amount == 0 {
            // No tokens were earned in this topic (or state.tokens_to_mint is 0), so nothing to convert.
            msg!(
                "Submission accepted, but no tempAlign tokens found in UserTopicBalance for topic {} to convert.",
                ctx.accounts.topic.key() // Use topic key for logging
            );
            // Proceed to log results without token conversion
        } else {
            // Check if the protocol-owned tempAlign account has enough balance (safety check)
            if ctx.accounts.contributor_temp_align_account.amount < conversion_amount {
                msg!("Error: Mismatch between UserTopicBalance and protocol-owned token account balance.");
                return Err(ErrorCode::InsufficientTokenBalance.into());
            }

            // 1. Burn tempAlign tokens from contributor's protocol-owned token account
            let state_bump = ctx.accounts.state.bump;
            let seeds = &[b"state".as_ref(), &[state_bump]];
            let signer = &[&seeds[..]];

            let burn_cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.temp_align_mint.to_account_info(),
                    from: ctx
                        .accounts
                        .contributor_temp_align_account
                        .to_account_info(),
                    authority: ctx.accounts.state.to_account_info(), // State is authority
                },
            )
            .with_signer(signer); // Sign with state PDA signer

            token::burn(burn_cpi_ctx, conversion_amount)?;

            // 2. Mint permanent Align tokens to contributor's user-owned ATA
            // (Minting logic uses state PDA signer)
            let mint_cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.align_mint.to_account_info(),
                    to: ctx.accounts.contributor_align_ata.to_account_info(), // Mint to user's ATA
                    authority: ctx.accounts.state.to_account_info(),          // State is authority
                },
            )
            .with_signer(signer); // Sign with state PDA signer

            token::mint_to(mint_cpi_ctx, conversion_amount)?;

            // Update the contributor's UserTopicBalance by reducing tempAlign amount
            let user_topic_balance = &mut ctx.accounts.user_topic_balance;
            user_topic_balance.temp_align_amount = user_topic_balance
                .temp_align_amount
                .checked_sub(conversion_amount)
                .ok_or(ErrorCode::Overflow)?;

            msg!(
                "Submission accepted! Converted {} tempAlign to {} Align for contributor {}",
                conversion_amount,
                conversion_amount,
                ctx.accounts.submission.contributor // Log contributor key
            );
            msg!(
                "Remaining tempAlign in UserTopicBalance for topic {}: {}",
                ctx.accounts.topic.key(), // Use topic key for logging
                user_topic_balance.temp_align_amount  // Log remaining balance directly
            );
        }
    } else {
        // If rejected, no token conversion happens. Just update status.
        link.status = SubmissionStatus::Rejected;
        msg!("Submission rejected. No token conversion performed.");
    }

    // Log the voting results (applies to both accepted/rejected)
    msg!(
        "Finalized submission {} in topic '{}' with status: {:?}",
        ctx.accounts.submission.key(), // Log submission key
        ctx.accounts.topic.name,       // Use topic name for logging
        link.status
    );
    msg!(
        "Final vote tally: {} YES vs {} NO",
        link.yes_voting_power,
        link.no_voting_power
    );

    Ok(())
}
