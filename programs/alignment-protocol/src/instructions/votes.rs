use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, MintTo};
use crate::contexts::{CommitVote, RevealVote, FinalizeVote};
use crate::error::ErrorCode;
use crate::data::{VoteChoice, SubmissionStatus};
use crate::helpers::calculate_quadratic_voting_power;
use sha2::{Digest, Sha256};

pub fn commit_vote(
    ctx: Context<CommitVote>,
    vote_hash: [u8; 32],
    vote_amount: u64,
    is_permanent_rep: bool,
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
    
    // Validate the vote amount based on token type
    if vote_amount == 0 {
        return Err(ErrorCode::ZeroVoteAmount.into());
    }
    
    // Check if user has enough Rep (either temp or permanent)
    if is_permanent_rep {
        // Voting with permanent Rep - can vote across any topic
        if ctx.accounts.user_profile.permanent_rep_amount < vote_amount {
            return Err(ErrorCode::InsufficientVotingPower.into());
        }
    } else {
        // Voting with tempRep - can only vote within the topic it was gained for
        
        // First check if they have enough total tempRep (legacy check)
        if ctx.accounts.user_profile.temp_rep_amount < vote_amount {
            return Err(ErrorCode::InsufficientVotingPower.into());
        }
        
        // Get the topic ID from the submission-topic link
        let topic_id = ctx.accounts.topic.id;
        
        // Check if they have enough topic-specific tempRep for this specific topic
        let user_profile = &ctx.accounts.user_profile;
        let mut found_topic = false;
        let mut topic_temp_rep = 0;
        
        // Find the topic in the user's topic_tokens collection
        for (id, token_balance) in user_profile.topic_tokens.iter() {
            if *id == topic_id {
                found_topic = true;
                topic_temp_rep = token_balance.temp_rep_amount;
                break;
            }
        }
        
        // Ensure the user has enough topic-specific tokens
        if !found_topic || topic_temp_rep < vote_amount {
            return Err(ErrorCode::NoReputationForTopic.into());
        }
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
    vote_commit.vote_amount = vote_amount;
    vote_commit.is_permanent_rep = is_permanent_rep;
    vote_commit.bump = ctx.bumps.vote_commit;
    
    // Increment the submission-topic link's committed votes counter
    let link = &mut ctx.accounts.submission_topic_link;
    link.total_committed_votes = link.total_committed_votes
        .checked_add(1)
        .ok_or(ErrorCode::Overflow)?;
    
    msg!("Vote committed for submission in topic '{}'", ctx.accounts.topic.name);
    msg!("Vote amount: {}", vote_amount);
    msg!("Using {} Rep", if is_permanent_rep { "permanent" } else { "temporary" });
    
    Ok(())
}

pub fn reveal_vote(
    ctx: Context<RevealVote>,
    vote_choice: VoteChoice,
    nonce: String,
) -> Result<()> {
    // Get current time to validate voting window
    let current_time = Clock::get()?.unix_timestamp as u64;
    let link = &ctx.accounts.submission_topic_link;
    
    // Check if reveal phase is active
    if current_time < link.reveal_phase_start {
        return Err(ErrorCode::RevealPhaseNotStarted.into());
    }
    
    if current_time > link.reveal_phase_end {
        return Err(ErrorCode::RevealPhaseEnded.into());
    }
    
    // Reconstruct the hash from the reveal data and verify it matches the commit
    let vote_commit = &mut ctx.accounts.vote_commit;
    
    // Create the pre-image for the hash
    // Format: validator pubkey + submission_topic_link pubkey + vote choice (0 for Yes, 1 for No) + nonce
    let mut hasher = Sha256::new();
    hasher.update(ctx.accounts.validator.key().as_ref());
    hasher.update(ctx.accounts.submission_topic_link.key().as_ref());
    hasher.update(&[vote_choice as u8]);
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
    let voting_power = calculate_quadratic_voting_power(vote_commit.vote_amount);
    
    // Add the voting power to the appropriate counter
    let link = &mut ctx.accounts.submission_topic_link;
    match vote_choice {
        VoteChoice::Yes => {
            link.yes_voting_power = link.yes_voting_power
                .checked_add(voting_power)
                .ok_or(ErrorCode::Overflow)?;
        },
        VoteChoice::No => {
            link.no_voting_power = link.no_voting_power
                .checked_add(voting_power)
                .ok_or(ErrorCode::Overflow)?;
        },
    }
    
    // Increment the revealed votes counter
    link.total_revealed_votes = link.total_revealed_votes
        .checked_add(1)
        .ok_or(ErrorCode::Overflow)?;
    
    msg!("Vote revealed for submission in topic '{}'", ctx.accounts.topic.name);
    msg!("Vote choice: {:?}", vote_choice);
    msg!("Voting power (quadratic): {}", voting_power);
    
    Ok(())
}

pub fn finalize_vote(
    ctx: Context<FinalizeVote>,
) -> Result<()> {
    // Check if the vote has already been finalized
    if ctx.accounts.vote_commit.finalized {
        return Err(ErrorCode::VoteAlreadyFinalized.into());
    }
    
    // Get the consensus outcome (accepted/rejected)
    let consensus_is_yes = ctx.accounts.submission_topic_link.status == SubmissionStatus::Accepted;
    
    // Get the validator's vote choice
    let vote_choice = ctx.accounts.vote_commit.vote_choice.ok_or(ErrorCode::InvalidVoteHash)?;
    let voted_yes = vote_choice == VoteChoice::Yes;
    
    // Check if the validator voted with the consensus
    let voted_with_consensus = (consensus_is_yes && voted_yes) || (!consensus_is_yes && !voted_yes);
    
    // Only process token conversions for temporary reputation
    // With permanent reputation, we don't burn or reward tokens for now
    if !ctx.accounts.vote_commit.is_permanent_rep {
        let vote_amount = ctx.accounts.vote_commit.vote_amount;
        
        if voted_with_consensus {
            // Validator voted correctly - convert tempRep to permanent Rep
            
            // Check if the validator has enough tempRep tokens
            if ctx.accounts.validator_temp_rep_ata.amount < vote_amount {
                return Err(ErrorCode::InsufficientTokenBalance.into());
            }
            
            // 1. Burn tempRep tokens
            let burn_cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.temp_rep_mint.to_account_info(),
                    from: ctx.accounts.validator_temp_rep_ata.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            );
            
            token::burn(burn_cpi_ctx, vote_amount)?;
            
            // 2. Mint permanent Rep tokens
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
            
            // Update validator profile
            let validator_profile = &mut ctx.accounts.validator_profile;
            validator_profile.temp_rep_amount = validator_profile.temp_rep_amount
                .checked_sub(vote_amount)
                .ok_or(ErrorCode::Overflow)?;
                
            validator_profile.permanent_rep_amount = validator_profile.permanent_rep_amount
                .checked_add(vote_amount)
                .ok_or(ErrorCode::Overflow)?;
            
            // Get the topic ID
            let topic_id = ctx.accounts.topic.id;
            
            // Update topic-specific token balances
            let mut found_topic = false;
            
            // Find the topic in the user's topic_tokens collection
            for (id, token_balance) in validator_profile.topic_tokens.iter_mut() {
                if *id == topic_id {
                    found_topic = true;
                    
                    // Decrease tempRep for this topic
                    token_balance.temp_rep_amount = token_balance.temp_rep_amount
                        .checked_sub(vote_amount)
                        .ok_or(ErrorCode::Overflow)?;
                    
                    break;
                }
            }
            
            // We should always find the topic since we already verified in commit_vote
            if !found_topic {
                msg!("Warning: Topic {} not found in validator's profile during finalization", topic_id);
            }
            
            msg!(
                "Validator voted correctly! Converted {} tempRep to {} permanent Rep",
                vote_amount,
                vote_amount
            );
        } else {
            // Validator voted incorrectly - burn tempRep tokens with no replacement
            
            // Check if the validator has enough tempRep tokens
            if ctx.accounts.validator_temp_rep_ata.amount < vote_amount {
                return Err(ErrorCode::InsufficientTokenBalance.into());
            }
            
            // Burn tempRep tokens
            let burn_cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.temp_rep_mint.to_account_info(),
                    from: ctx.accounts.validator_temp_rep_ata.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            );
            
            token::burn(burn_cpi_ctx, vote_amount)?;
            
            // Update validator profile
            let validator_profile = &mut ctx.accounts.validator_profile;
            validator_profile.temp_rep_amount = validator_profile.temp_rep_amount
                .checked_sub(vote_amount)
                .ok_or(ErrorCode::Overflow)?;
                
            // Get the topic ID
            let topic_id = ctx.accounts.topic.id;
            
            // Update topic-specific token balances
            let mut found_topic = false;
            
            // Find the topic in the user's topic_tokens collection
            for (id, token_balance) in validator_profile.topic_tokens.iter_mut() {
                if *id == topic_id {
                    found_topic = true;
                    
                    // Decrease tempRep for this topic
                    token_balance.temp_rep_amount = token_balance.temp_rep_amount
                        .checked_sub(vote_amount)
                        .ok_or(ErrorCode::Overflow)?;
                    
                    break;
                }
            }
            
            // We should always find the topic since we already verified in commit_vote
            if !found_topic {
                msg!("Warning: Topic {} not found in validator's profile during finalization", topic_id);
            }
            
            msg!(
                "Validator voted incorrectly. Burned {} tempRep tokens with no replacement",
                vote_amount
            );
        }
    } else {
        // Using permanent Rep tokens
        // For MVP we don't apply penalties to permanent Rep
        msg!("Vote was made with permanent Rep tokens. No token conversion applied.");
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