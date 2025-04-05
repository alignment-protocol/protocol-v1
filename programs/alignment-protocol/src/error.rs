use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid authority for this state")]
    InvalidAuthority,

    #[msg("Arithmetic overflow occurred")]
    Overflow,

    #[msg("Insufficient token balance for staking")]
    InsufficientTokenBalance,

    #[msg("Token mint mismatch")]
    TokenMintMismatch,

    #[msg("Invalid token account")]
    InvalidTokenAccount,

    #[msg("Invalid user profile")]
    InvalidUserProfile,

    #[msg("User profile already initialized")]
    UserProfileAlreadyInitialized,

    #[msg("Cannot stake zero tokens")]
    ZeroStakeAmount,

    // Topic-related errors
    #[msg("Topic name cannot be empty")]
    EmptyTopicName,

    #[msg("Topic name exceeds maximum length")]
    TopicNameTooLong,

    #[msg("Topic description exceeds maximum length")]
    TopicDescriptionTooLong,

    #[msg("Topic is inactive")]
    TopicInactive,

    #[msg("No active topics available for submission")]
    NoActiveTopics,

    #[msg("Submission already exists in this topic")]
    SubmissionAlreadyInTopic,

    // Cross-topic submission errors
    #[msg("Not authorized to link this submission")]
    NotAuthorizedToLinkSubmission,

    // Voting-related errors
    #[msg("Vote has already been committed")]
    VoteAlreadyCommitted,

    #[msg("Vote has already been revealed")]
    VoteAlreadyRevealed,

    #[msg("Invalid vote hash")]
    InvalidVoteHash,

    #[msg("Validator has no reputation tokens for this topic")]
    NoReputationForTopic,

    #[msg("Submission is not in the pending state")]
    SubmissionNotPending,

    #[msg("Vote amount exceeds available reputation")]
    InsufficientVotingPower,

    #[msg("Vote amount must be greater than zero")]
    ZeroVoteAmount,

    #[msg("Commit phase has not started yet")]
    CommitPhaseNotStarted,

    #[msg("Commit phase has ended")]
    CommitPhaseEnded,

    #[msg("Reveal phase has not started yet")]
    RevealPhaseNotStarted,

    #[msg("Reveal phase has ended")]
    RevealPhaseEnded,

    #[msg("Reveal phase has not ended yet")]
    RevealPhaseNotEnded,

    #[msg("Vote has already been finalized")]
    VoteAlreadyFinalized,

    #[msg("Insufficient topic-specific token balance")]
    InsufficientTopicTokens,

    #[msg("Invalid voting phase order")]
    InvalidPhaseOrder,

    #[msg("Self-voting is not allowed: validators cannot vote on their own submissions")]
    SelfVotingNotAllowed,

    #[msg("You have already committed a vote for this submission-topic pair")]
    DuplicateVoteCommitment,

    #[msg("The user account in the provided data does not match the expected user.")]
    UserAccountMismatch,

    #[msg("The provided topic account does not match the expected topic.")]
    InvalidTopic,

    #[msg("The provided submission index does not match the user's current submission count.")]
    IncorrectSubmissionIndex,

    #[msg("The data reference exceeds the maximum allowed length")]
    DataReferenceTooLong,

    #[msg("The data reference cannot be empty")]
    EmptyDataReference,

    #[msg("The provided submission does not match the expected submission.")]
    InvalidSubmission,

    // --- AI Validation Errors ---
    #[msg("The signer is not the original contributor of the submission.")]
    NotSubmissionContributor,

    #[msg("Insufficient temporary reputation balance for AI validation request.")]
    InsufficientTempRepBalance,

    #[msg("The signer is not the authorized AI Oracle.")]
    UnauthorizedOracle,

    #[msg("The AI validation request is not in the expected state (e.g., not Pending).")]
    InvalidAiRequestStatus,

    #[msg("The AI validation request account does not correspond to the provided SubmissionTopicLink.")]
    MismatchedAiRequestLink,

    #[msg("This submission has already been finalized.")]
    SubmissionAlreadyFinalized,

    // State Mismatch Errors (Start: 2024)
    #[msg("AI request index mismatch. State may have changed.")]
    StateMismatch,
}
