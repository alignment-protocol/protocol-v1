use crate::contexts::CreateTopic;
use crate::data::{MAX_TOPIC_DESCRIPTION_LENGTH, MAX_TOPIC_NAME_LENGTH};
use crate::error::ErrorCode;
use anchor_lang::prelude::*;

pub fn create_topic(
    ctx: Context<CreateTopic>,
    name: String,
    description: String,
    commit_phase_duration: Option<u64>,
    reveal_phase_duration: Option<u64>,
) -> Result<()> {
    // Validate inputs
    if name.is_empty() {
        return Err(ErrorCode::EmptyTopicName.into());
    }

    if name.len() > MAX_TOPIC_NAME_LENGTH {
        return Err(ErrorCode::TopicNameTooLong.into());
    }

    if description.len() > MAX_TOPIC_DESCRIPTION_LENGTH {
        return Err(ErrorCode::TopicDescriptionTooLong.into());
    }

    // Initialize the topic
    let topic = &mut ctx.accounts.topic;
    let state = &mut ctx.accounts.state;
    let creator = &ctx.accounts.creator;

    topic.name = name.clone();
    topic.description = description.clone();
    // Record the wallet that created the topic for future reference/permissions.
    topic.creator = creator.key();
    topic.submission_count = 0;
    topic.is_active = true;
    topic.bump = ctx.bumps.topic;

    // Set the commit and reveal phase durations - use provided values or defaults from state
    topic.commit_phase_duration =
        commit_phase_duration.unwrap_or(state.default_commit_phase_duration);
    topic.reveal_phase_duration =
        reveal_phase_duration.unwrap_or(state.default_reveal_phase_duration);

    // Increment the topic count
    state.topic_count = state
        .topic_count
        .checked_add(1)
        .ok_or(ErrorCode::Overflow)?;

    msg!("Created new topic: {}", name);
    msg!("Description: {}", description);
    msg!(
        "Commit phase duration: {} seconds",
        topic.commit_phase_duration
    );
    msg!(
        "Reveal phase duration: {} seconds",
        topic.reveal_phase_duration
    );

    Ok(())
}

/// Update mutable fields of an existing topic (phase durations, activity flag).
/// The signer must be either the protocol authority (state.authority) or the
/// original topic creator (topic.authority).
pub fn update_topic(
    ctx: Context<crate::contexts::UpdateTopic>,
    commit_phase_duration: Option<u64>,
    reveal_phase_duration: Option<u64>,
    is_active: Option<bool>,
) -> Result<()> {
    let state = &ctx.accounts.state;
    let topic = &mut ctx.accounts.topic;
    let signer = ctx.accounts.authority.key();

    // Authorisation check
    require!(
        signer == state.authority || signer == topic.creator,
        ErrorCode::InvalidAuthority
    );

    // Apply updates if provided
    if let Some(new_commit) = commit_phase_duration {
        topic.commit_phase_duration = new_commit;
    }
    if let Some(new_reveal) = reveal_phase_duration {
        topic.reveal_phase_duration = new_reveal;
    }
    if let Some(active) = is_active {
        topic.is_active = active;
    }

    msg!("Updated topic {} by {}", topic.key(), signer);
    msg!(
        "commit_phase_duration = {}, reveal_phase_duration = {}, is_active = {}",
        topic.commit_phase_duration,
        topic.reveal_phase_duration,
        topic.is_active
    );

    Ok(())
}
