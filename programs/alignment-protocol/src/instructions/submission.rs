use crate::contexts::{FinalizeSubmission, LinkSubmissionToTopic, SubmitDataToTopic};
use crate::data::{SubmissionStatus, MAX_DATA_REFERENCE_LENGTH};
use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, MintTo};

// Removed legacy submit_data function

pub fn submit_data_to_topic(ctx: Context<SubmitDataToTopic>, data_reference: String) -> Result<()> {
    // Validate inputs
    if data_reference.len() > MAX_DATA_REFERENCE_LENGTH {
        return Err(error!(ErrorCode::TopicDescriptionTooLong));
    }

    // Get current time
    let current_time = Clock::get()?.unix_timestamp as u64;

    // Fill out the Submission account
    let submission = &mut ctx.accounts.submission;
    submission.contributor = ctx.accounts.contributor.key();
    submission.timestamp = current_time;
    submission.data_reference = data_reference.clone();
    submission.bump = ctx.bumps.submission;

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

    // Mint temporary alignment tokens to the contributor if configured
    if ctx.accounts.state.tokens_to_mint > 0 {
        let state_bump = ctx.accounts.state.bump;
        let seeds = &[b"state".as_ref(), &[state_bump]];
        let signer = &[&seeds[..]];

        // CPI to the Token Program's 'mint_to'
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.temp_align_mint.to_account_info(),
                to: ctx
                    .accounts
                    .contributor_temp_align_account
                    .to_account_info(),
                authority: ctx.accounts.state.to_account_info(),
            },
        )
        .with_signer(signer);

        token::mint_to(cpi_ctx, ctx.accounts.state.tokens_to_mint)?;

        // If contributor has a user profile, update their topic-specific token balance
        if let Some(contributor_profile) = ctx.accounts.contributor_profile.as_mut() {
            // Get the topic ID
            let topic_id = ctx.accounts.topic.id;

            // Find or create the topic token entry
            let topic_tokens = &mut contributor_profile.topic_tokens;

            // Try to find the topic in the user's topic_tokens map
            let mut found = false;
            for topic_pair in topic_tokens.iter_mut() {
                if topic_pair.topic_id == topic_id {
                    // Topic found, add to its temp_align_amount
                    topic_pair.token.temp_align_amount = topic_pair
                        .token
                        .temp_align_amount
                        .checked_add(ctx.accounts.state.tokens_to_mint)
                        .ok_or(ErrorCode::Overflow)?;
                    found = true;
                    break;
                }
            }

            // If not found, create a new entry
            if !found {
                topic_tokens.push(crate::data::TopicTokenPair {
                    topic_id,
                    token: crate::data::UserTopicBalance {
                        temp_align_amount: ctx.accounts.state.tokens_to_mint,
                        temp_rep_amount: 0,
                        locked_temp_rep_amount: 0,
                    },
                });
            }

            msg!(
                "Updated topic-specific token balance for topic {} (+{} tempAlign)",
                topic_id,
                ctx.accounts.state.tokens_to_mint
            );
        }

        msg!(
            "Minted {} tempAlign tokens to {} (protocol-owned account for user {})",
            ctx.accounts.state.tokens_to_mint,
            ctx.accounts.contributor_temp_align_account.key(),
            ctx.accounts.contributor.key()
        );
    }

    // Increment submission counts
    let state = &mut ctx.accounts.state;
    state.submission_count = state
        .submission_count
        .checked_add(1)
        .ok_or(ErrorCode::Overflow)?;

    let topic = &mut ctx.accounts.topic;
    topic.submission_count = topic
        .submission_count
        .checked_add(1)
        .ok_or(ErrorCode::Overflow)?;

    msg!("New submission added to topic '{}'", topic.name);
    msg!("Data reference: {}", data_reference);
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
    // Calculate final vote tallies
    let link = &mut ctx.accounts.submission_topic_link;

    // Determine if the submission is accepted or rejected
    let is_accepted = link.yes_voting_power > link.no_voting_power;

    // Update submission status
    if is_accepted {
        link.status = SubmissionStatus::Accepted;

        // Convert contributor's tempAlign tokens to permanent Align tokens
        // For simplicity, we assume a 1:1 conversion rate in the MVP

        // Get conversion amount (tempAlign to burn and Align to mint)
        // In a real implementation, this might be a function of the submission quality
        let tokens_to_mint = ctx.accounts.state.tokens_to_mint;

        // Check if topic-specific balance can be found
        let topic_id = ctx.accounts.topic.id;
        let mut topic_align_balance = 0;
        let mut found_topic = false;

        // Check the contributor's topic-specific token balance
        for topic_pair in ctx.accounts.contributor_profile.topic_tokens.iter() {
            if topic_pair.topic_id == topic_id {
                found_topic = true;
                topic_align_balance = topic_pair.token.temp_align_amount;
                break;
            }
        }

        // Determine conversion amount based on topic-specific balance
        let conversion_amount = if found_topic {
            // Don't try to convert more than what was earned in this topic
            std::cmp::min(tokens_to_mint, topic_align_balance)
        } else {
            // If topic not found, assume they have none from this topic
            0
        };

        // If no topic-specific tokens, abort with error
        if conversion_amount == 0 {
            return Err(ErrorCode::InsufficientTopicTokens.into());
        }

        // Check if the contributor has enough tempAlign tokens globally
        if ctx.accounts.contributor_temp_align_account.amount < conversion_amount {
            return Err(ErrorCode::InsufficientTokenBalance.into());
        }

        // 1. Burn tempAlign tokens from contributor's protocol-owned token account
        // Since the token account is owned by the protocol, we use the state PDA as the authority
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
                authority: ctx.accounts.state.to_account_info(),
            },
        )
        .with_signer(signer);

        token::burn(burn_cpi_ctx, conversion_amount)?;

        // 2. Mint permanent Align tokens to contributor
        let state_bump = ctx.accounts.state.bump;
        let seeds = &[b"state".as_ref(), &[state_bump]];
        let signer = &[&seeds[..]];

        let mint_cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.align_mint.to_account_info(),
                to: ctx.accounts.contributor_align_ata.to_account_info(),
                authority: ctx.accounts.state.to_account_info(),
            },
        )
        .with_signer(signer);

        token::mint_to(mint_cpi_ctx, conversion_amount)?;

        // Update the contributor's topic-specific balance
        if found_topic {
            for topic_pair in ctx.accounts.contributor_profile.topic_tokens.iter_mut() {
                if topic_pair.topic_id == topic_id {
                    // Reduce the topic-specific tempAlign amount
                    topic_pair.token.temp_align_amount = topic_pair
                        .token
                        .temp_align_amount
                        .checked_sub(conversion_amount)
                        .ok_or(ErrorCode::Overflow)?;
                    break;
                }
            }
        }

        msg!(
            "Submission accepted! Converted {} tempAlign to {} Align for contributor",
            conversion_amount,
            conversion_amount
        );
        msg!(
            "Remaining topic-specific tempAlign for topic {}: {}",
            topic_id,
            if found_topic {
                topic_align_balance.saturating_sub(conversion_amount)
            } else {
                0
            }
        );
    } else {
        // If rejected, no token conversion happens
        link.status = SubmissionStatus::Rejected;
        msg!("Submission rejected. No token conversion performed.");
    }

    // Log the voting results
    msg!(
        "Finalized submission in topic '{}' with status: {:?}",
        ctx.accounts.topic.name,
        link.status
    );
    msg!(
        "Final vote tally: {} YES vs {} NO",
        link.yes_voting_power,
        link.no_voting_power
    );

    Ok(())
}
