use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::{solana_sdk::pubkey::Pubkey, Program};
use anchor_spl::associated_token::get_associated_token_address;
use std::rc::Rc;

/// Get the program-derived address (PDA) for the state account
pub fn get_state_pda(program: &Program<Rc<Keypair>>) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"state"], &program.id())
}

/// Get the PDA for a specific mint
pub fn get_mint_pda(program: &Program<Rc<Keypair>>, mint_type: &str) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[mint_type.as_bytes()], &program.id())
}

/// Get the PDA for a topic account
pub fn get_topic_pda(program: &Program<Rc<Keypair>>, topic_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"topic", &topic_id.to_le_bytes()], &program.id())
}

/// Get the PDA for a user profile account
pub fn get_user_profile_pda(program: &Program<Rc<Keypair>>, user: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"user_profile", user.as_ref()], &program.id())
}

/// Get the PDA for a submission-topic link account
pub fn get_submission_topic_link_pda(
    program: &Program<Rc<Keypair>>,
    submission: &Pubkey,
    topic: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"submission_topic_link",
            submission.as_ref(),
            topic.as_ref(),
        ],
        &program.id(),
    )
}

/// Get the PDA for a vote commit account
pub fn get_vote_commit_pda(
    program: &Program<Rc<Keypair>>,
    submission_topic_link: &Pubkey,
    validator: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"vote_commit",
            submission_topic_link.as_ref(),
            validator.as_ref(),
        ],
        &program.id(),
    )
}

/// Get the PDA for a user's temporary token account
pub fn get_user_temp_token_account_pda(
    program: &Program<Rc<Keypair>>,
    user: &Pubkey,
    token_type: &str,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[token_type.as_bytes(), user.as_ref()], &program.id())
}

/// Get the ATA for a token mint
pub fn get_token_ata(user: &Pubkey, mint: &Pubkey) -> Pubkey {
    get_associated_token_address(user, mint)
}

/// Get the PDA for a user's topic balance account
pub fn get_user_topic_balance_pda(
    program: &Program<Rc<Keypair>>,
    user: &Pubkey,
    topic: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"user_topic_balance", user.as_ref(), topic.as_ref()],
        &program.id(),
    )
}
