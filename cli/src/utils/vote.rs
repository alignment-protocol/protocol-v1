use alignment_protocol::VoteChoice;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use sha2::{Digest, Sha256};

/// Generate a vote hash for commitment phase
pub fn generate_vote_hash(
    validator: &Pubkey,
    submission_topic_link: &Pubkey,
    vote_choice: &VoteChoice,
    nonce: &str,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(validator.to_bytes());
    hasher.update(submission_topic_link.to_bytes());
    match vote_choice {
        VoteChoice::Yes => hasher.update(b"yes"),
        VoteChoice::No => hasher.update(b"no"),
    };
    hasher.update(nonce.as_bytes());
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result[..]);
    hash
}

/// Parse vote choice from string
pub fn parse_vote_choice(choice: &str) -> Result<VoteChoice> {
    match choice.to_lowercase().as_str() {
        "yes" | "y" | "true" | "1" => Ok(VoteChoice::Yes),
        "no" | "n" | "false" | "0" => Ok(VoteChoice::No),
        _ => Err(anyhow::anyhow!("Invalid vote choice. Use 'yes' or 'no'")),
    }
}
