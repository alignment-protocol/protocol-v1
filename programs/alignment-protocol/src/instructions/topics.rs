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

    topic.name = name.clone();
    topic.description = description.clone();
    topic.authority = ctx.accounts.authority.key();
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
