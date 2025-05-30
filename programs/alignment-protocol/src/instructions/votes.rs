use crate::contexts::{CommitVote, FinalizeVote, RevealVote, SetVotingPhases};
use crate::data::{SubmissionStatus, VoteChoice};
use crate::error::ErrorCode;
use crate::helpers::calculate_quadratic_voting_power;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, MintTo};
use sha2::{Digest, Sha256};

pub fn commit_vote(
    ctx: Context<CommitVote>,
    vote_hash: [u8; 32],
    temp_rep_amount: u64,
    perm_rep_amount: u64,
) -> Result<()> {
    // Get current time to validate voting window
    let current_time = Clock::get()?.unix_timestamp as u64;
    let link = &ctx.accounts.submission_topic_link;

    // Check if commit phase is active
    if current_time < link.commit_phase_start {
        return Err(ErrorCode::CommitPhaseNotStarted.into());
    }

    if current_time > link.commit_phase_end {
        return Err(ErrorCode::CommitPhaseEnded.into());
    }

    // Validate vote amounts
    if temp_rep_amount == 0 && perm_rep_amount == 0 {
        return Err(ErrorCode::ZeroVoteAmount.into());
    }

    // MVP Constraint: perm_rep_amount must be 0
    if perm_rep_amount > 0 {
        // This is a temporary restriction for the MVP
        // msg!("Permanent REP commitment is not yet fully supported in MVP.");
        return Err(ErrorCode::PermRepCommitmentNotSupported.into());
    }

    // IMPORTANT: Prevent self-voting by checking if the validator is the submission contributor
    if ctx.accounts.validator.key() == ctx.accounts.submission.contributor {
        msg!("Self-voting is not allowed: validators cannot vote on their own submissions");
        return Err(ErrorCode::SelfVotingNotAllowed.into());
    }

    // Check if vote has already been committed (examine if vote_commit is initialized)
    if ctx.accounts.vote_commit.validator != Pubkey::default() {
        msg!("You have already committed a vote for this submission-topic pair");
        return Err(ErrorCode::DuplicateVoteCommitment.into());
    }

    // Handle temporary reputation (tempRep)
    if temp_rep_amount > 0 {
        let user_topic_balance = &mut ctx.accounts.user_topic_balance;

        // Check available topic-specific tempRep
        if user_topic_balance.temp_rep_amount < temp_rep_amount {
            msg!(
                "Insufficient tempRep for topic {}. Required: {}, Available: {}",
                ctx.accounts.topic.key(),
                temp_rep_amount,
                user_topic_balance.temp_rep_amount
            );
            return Err(ErrorCode::NoReputationForTopic.into());
        }

        // Lock the tempRep tokens
        user_topic_balance.temp_rep_amount = user_topic_balance
            .temp_rep_amount
            .checked_sub(temp_rep_amount)
            .ok_or(ErrorCode::Overflow)?;

        user_topic_balance.locked_temp_rep_amount = user_topic_balance
            .locked_temp_rep_amount
            .checked_add(temp_rep_amount)
            .ok_or(ErrorCode::Overflow)?;

        msg!(
            "Locked {} tempRep tokens for voting in topic {}",
            temp_rep_amount,
            ctx.accounts.topic.key()
        );
        msg!(
            "New available tempRep balance in UserTopicBalance: {}",
            user_topic_balance.temp_rep_amount
        );
        msg!(
            "New locked tempRep balance in UserTopicBalance: {}",
            user_topic_balance.locked_temp_rep_amount
        );
    }

    // Handle permanent reputation (Rep)
    // For MVP, perm_rep_amount must be 0 (checked above).
    // This section is structured for future non-zero perm_rep_amount.
    if perm_rep_amount > 0 {
        // Check balance in user-owned ATA for permanent Rep
        if ctx.accounts.validator_rep_ata.amount < perm_rep_amount {
            msg!(
                "Insufficient permanent Rep. Required: {}, Available: {}",
                perm_rep_amount,
                ctx.accounts.validator_rep_ata.amount
            );
            return Err(ErrorCode::InsufficientVotingPower.into());
        }
        // Actual locking/transfer of permanent REP will be handled post-MVP,
        // potentially involving a different ATA structure or escrow mechanism (ALIGN-18).
        // For MVP, if perm_rep_amount > 0, it would have failed the earlier MVP constraint.
        // If somehow perm_rep_amount is > 0 here and passed the MVP check (which it shouldn't),
        // this message indicates that no locking action is taken for perm_rep in this MVP version.
        msg!("Committing vote with {} permanent Rep (Note: No on-chain locking for perm_rep in MVP beyond balance check)", perm_rep_amount);
    }

    // Initialize the vote commit
    let vote_commit = &mut ctx.accounts.vote_commit;
    vote_commit.submission_topic_link = ctx.accounts.submission_topic_link.key();
    vote_commit.validator = ctx.accounts.validator.key();
    vote_commit.vote_hash = vote_hash;
    vote_commit.revealed = false;
    vote_commit.finalized = false;
    vote_commit.vote_choice = None;
    vote_commit.commit_timestamp = current_time;
    vote_commit.temp_rep_amount = temp_rep_amount;
    vote_commit.perm_rep_amount = perm_rep_amount;
    vote_commit.bump = ctx.bumps.vote_commit;

    // Increment the submission-topic link's committed votes counter
    let link = &mut ctx.accounts.submission_topic_link;
    msg!(
        "Before increment: total_committed_votes = {}",
        link.total_committed_votes
    );
    link.total_committed_votes = link
        .total_committed_votes
        .checked_add(1)
        .ok_or(ErrorCode::Overflow)?;

    msg!(
        "Vote committed for submission in topic \'{}\'",
        ctx.accounts.topic.name
    );
    if temp_rep_amount > 0 {
        msg!("Temporary REP committed: {}", temp_rep_amount);
    }
    if perm_rep_amount > 0 {
        // This message will effectively not show for perm_rep_amount in MVP due to earlier constraint
        msg!("Permanent REP committed: {}", perm_rep_amount);
    }
    // For overall context, let's log total effective amount for this commit
    // Even if perm_rep_amount is 0, this helps in logs
    msg!(
        "Total effective vote amount considered for this commit: {}",
        temp_rep_amount + perm_rep_amount
    );
    msg!(
        "Total committed votes for link now: {}",
        link.total_committed_votes
    );

    Ok(())
}

pub fn reveal_vote(ctx: Context<RevealVote>, vote_choice: VoteChoice, nonce: String) -> Result<()> {
    // Get current time to validate voting window
    let current_time = Clock::get()?.unix_timestamp as u64;
    let link = &ctx.accounts.submission_topic_link;

    // Check if reveal phase is active
    if current_time < link.reveal_phase_start {
        return Err(ErrorCode::RevealPhaseNotStarted.into());
    }

    // Check if reveal phase has ended - THIS USES RevealPhaseEnded
    if current_time > link.reveal_phase_end {
        return Err(ErrorCode::RevealPhaseEnded.into()); // Keep this as RevealPhaseEnded
    }

    // Reconstruct the hash from the reveal data and verify it matches the commit
    let vote_commit = &mut ctx.accounts.vote_commit;

    // Create the pre-image for the hash
    // Format: validator pubkey + submission_topic_link pubkey + vote choice (0 for Yes, 1 for No) + nonce
    let mut hasher = Sha256::new();
    hasher.update(ctx.accounts.validator.key().as_ref());
    hasher.update(ctx.accounts.submission_topic_link.key().as_ref());
    hasher.update([vote_choice as u8]);
    hasher.update(nonce.as_bytes());

    let reconstructed_hash: [u8; 32] = hasher.finalize().into();

    // Verify that the reconstructed hash matches the stored hash
    if reconstructed_hash != vote_commit.vote_hash {
        return Err(ErrorCode::InvalidVoteHash.into());
    }

    // Mark the vote as revealed and store the vote choice
    vote_commit.revealed = true;
    vote_commit.vote_choice = Some(vote_choice);

    // Calculate voting power (quadratic)
    // Sum temp_rep_amount and perm_rep_amount for total voting power base
    let total_vote_amount = vote_commit
        .temp_rep_amount
        .checked_add(vote_commit.perm_rep_amount)
        .ok_or(ErrorCode::Overflow)?;
    let voting_power = calculate_quadratic_voting_power(total_vote_amount);

    // Add the voting power to the appropriate counter
    let link = &mut ctx.accounts.submission_topic_link;
    match vote_choice {
        VoteChoice::Yes => {
            link.yes_voting_power = link
                .yes_voting_power
                .checked_add(voting_power)
                .ok_or(ErrorCode::Overflow)?;
        }
        VoteChoice::No => {
            link.no_voting_power = link
                .no_voting_power
                .checked_add(voting_power)
                .ok_or(ErrorCode::Overflow)?;
        }
    }

    // Increment the revealed votes counter
    link.total_revealed_votes = link
        .total_revealed_votes
        .checked_add(1)
        .ok_or(ErrorCode::Overflow)?;

    msg!(
        "Vote revealed for submission in topic '{}'",
        ctx.accounts.topic.name
    );
    msg!("Vote choice: {:?}", vote_choice);
    msg!("Voting power (quadratic): {}", voting_power);

    Ok(())
}

pub fn finalize_vote(ctx: Context<FinalizeVote>) -> Result<()> {
    // Check if the vote has already been finalized
    if ctx.accounts.vote_commit.finalized {
        return Err(ErrorCode::VoteAlreadyFinalized.into());
    }

    // Get the consensus outcome (accepted/rejected)
    let consensus_is_yes = ctx.accounts.submission_topic_link.status == SubmissionStatus::Accepted;

    // Get the validator's vote choice
    let vote_choice = ctx
        .accounts
        .vote_commit
        .vote_choice
        .ok_or(ErrorCode::InvalidVoteHash)?;
    let voted_yes = vote_choice == VoteChoice::Yes;

    // Check if the validator voted with the consensus
    let voted_with_consensus = (consensus_is_yes && voted_yes) || (!consensus_is_yes && !voted_yes);

    // Only process token conversions/burns if temporary reputation was used
    if ctx.accounts.vote_commit.temp_rep_amount > 0 {
        let vote_amount = ctx.accounts.vote_commit.temp_rep_amount; // Use temp_rep_amount here

        if voted_with_consensus {
            // Validator voted correctly - convert tempRep to permanent Rep

            // Check token balance in protocol-owned tempRep account
            if ctx.accounts.validator_temp_rep_account.amount < vote_amount {
                msg!("Error: Mismatch between locked amount and tempRep token account balance during finalization.");
                return Err(ErrorCode::InsufficientTokenBalance.into());
            }

            // 1. Burn tempRep tokens from protocol-owned account
            // Use state PDA as the authority
            let state_bump = ctx.accounts.state.bump;
            let seeds = &[b"state".as_ref(), &[state_bump]];
            let signer = &[&seeds[..]];

            let burn_cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.temp_rep_mint.to_account_info(),
                    from: ctx.accounts.validator_temp_rep_account.to_account_info(),
                    authority: ctx.accounts.state.to_account_info(),
                },
            )
            .with_signer(signer);

            token::burn(burn_cpi_ctx, vote_amount)?;

            // 2. Mint permanent Rep tokens to user-owned ATA
            let state_bump = ctx.accounts.state.bump;
            let seeds = &[b"state".as_ref(), &[state_bump]];
            let signer = &[&seeds[..]];

            let mint_cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.rep_mint.to_account_info(),
                    to: ctx.accounts.validator_rep_ata.to_account_info(),
                    authority: ctx.accounts.state.to_account_info(),
                },
            )
            .with_signer(signer);

            token::mint_to(mint_cpi_ctx, vote_amount)?;

            msg!(
                "Validator voted correctly! Converted {} tempRep to {} permanent Rep (minted to ATA)",
                vote_amount,
                vote_amount
            );
        } else {
            // Validator voted incorrectly - burn tempRep tokens with no replacement

            // Check token balance in protocol-owned tempRep account
            if ctx.accounts.validator_temp_rep_account.amount < vote_amount {
                msg!("Error: Mismatch between locked amount and tempRep token account balance during finalization.");
                return Err(ErrorCode::InsufficientTokenBalance.into());
            }

            // Burn tempRep tokens from protocol-owned account
            // Use state PDA as the authority for the protocol-owned tempRep account
            let state_bump = ctx.accounts.state.bump;
            let seeds = &[b"state".as_ref(), &[state_bump]];
            let signer = &[&seeds[..]];

            let burn_cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.temp_rep_mint.to_account_info(),
                    from: ctx.accounts.validator_temp_rep_account.to_account_info(),
                    authority: ctx.accounts.state.to_account_info(),
                },
            )
            .with_signer(signer);

            token::burn(burn_cpi_ctx, vote_amount)?;

            msg!(
                "Validator voted incorrectly. Burned {} tempRep tokens with no replacement",
                vote_amount
            );
        }
    } else if ctx.accounts.vote_commit.perm_rep_amount > 0 {
        // This branch is for votes made only with permanent REP (temp_rep_amount is 0)
        // No token conversion/burn/reward/penalty for permanent REP in MVP
        msg!(
            "Vote was made with permanent Rep tokens (amount: {}). No token conversion applied.",
            ctx.accounts.vote_commit.perm_rep_amount
        );
    }

    // Update the locked token balance in UserTopicBalance if tempRep was used
    if ctx.accounts.vote_commit.temp_rep_amount > 0 {
        let vote_amount = ctx.accounts.vote_commit.temp_rep_amount; // Use temp_rep_amount here
        let user_topic_balance = &mut ctx.accounts.user_topic_balance;

        // Unlock the tokens that were committed to this vote
        user_topic_balance.locked_temp_rep_amount = user_topic_balance
            .locked_temp_rep_amount
            .checked_sub(vote_amount)
            .ok_or(ErrorCode::Overflow)?;

        msg!(
            "Unlocked {} tempRep tokens from locked pool in UserTopicBalance for topic {}",
            vote_amount,
            ctx.accounts.topic.key()
        );
        msg!(
            "New locked balance in UserTopicBalance: {}",
            user_topic_balance.locked_temp_rep_amount
        );
    }

    // Mark the vote as finalized
    let vote_commit = &mut ctx.accounts.vote_commit;
    vote_commit.finalized = true;

    msg!(
        "Finalized vote for validator {} on submission in topic '{}'",
        ctx.accounts.validator_profile.user,
        ctx.accounts.topic.name
    );

    Ok(())
}

/// Set arbitrary timestamps for a submission's voting phases for testing or administrative purposes
///
/// This function allows the protocol authority to manually set timestamps for the commit and reveal phases.
/// This is primarily intended for testing where time-based constraints are difficult to simulate,
/// but could also be used for emergency situations in production.
///
/// Parameters:
/// * `commit_phase_start`: Optional start timestamp for commit phase. If None, keeps current value.
/// * `commit_phase_end`: Optional end timestamp for commit phase. If None, keeps current value.
/// * `reveal_phase_start`: Optional start timestamp for reveal phase. If None, keeps current value.
/// * `reveal_phase_end`: Optional end timestamp for reveal phase. If None, keeps current value.
pub fn set_voting_phases(
    ctx: Context<SetVotingPhases>,
    commit_phase_start: Option<u64>,
    commit_phase_end: Option<u64>,
    reveal_phase_start: Option<u64>,
    reveal_phase_end: Option<u64>,
) -> Result<()> {
    // Get the current time for logging purposes (though we don't use it for validation)
    let _current_time = Clock::get()?.unix_timestamp as u64;
    let link = &mut ctx.accounts.submission_topic_link;

    // Update timestamps, validating time ordering constraints
    let new_commit_start = commit_phase_start.unwrap_or(link.commit_phase_start);
    let new_commit_end = commit_phase_end.unwrap_or(link.commit_phase_end);
    let new_reveal_start = reveal_phase_start.unwrap_or(link.reveal_phase_start);
    let new_reveal_end = reveal_phase_end.unwrap_or(link.reveal_phase_end);

    // Basic validation: times should be in ascending order and not in the past
    // Exception: we allow setting times in the past for testing purposes, but maintain the order
    if new_commit_start > new_commit_end {
        return Err(ErrorCode::InvalidPhaseOrder.into());
    }
    if new_commit_end > new_reveal_start {
        return Err(ErrorCode::InvalidPhaseOrder.into());
    }
    if new_reveal_start > new_reveal_end {
        return Err(ErrorCode::InvalidPhaseOrder.into());
    }

    // Apply the new timestamps
    link.commit_phase_start = new_commit_start;
    link.commit_phase_end = new_commit_end;
    link.reveal_phase_start = new_reveal_start;
    link.reveal_phase_end = new_reveal_end;

    msg!(
        "Modified voting phases for submission in topic '{}' by authority",
        ctx.accounts.topic.name
    );
    msg!(
        "New commit phase: {} to {}",
        link.commit_phase_start,
        link.commit_phase_end
    );
    msg!(
        "New reveal phase: {} to {}",
        link.reveal_phase_start,
        link.reveal_phase_end
    );

    Ok(())
}
