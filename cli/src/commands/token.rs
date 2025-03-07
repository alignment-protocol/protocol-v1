use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::{
    solana_sdk::{system_program, sysvar},
    Program,
};
use anyhow::Result;
use std::rc::Rc;

use alignment_protocol::{
    accounts as AccountsAll, instruction as InstructionAll, State as StateAccount,
};

use crate::utils::pda::{
    get_state_pda, get_topic_pda, get_user_profile_pda, get_user_temp_token_account_pda,
};

/// Stake temporary alignment tokens for a topic to earn reputation
pub fn cmd_stake_topic_specific_tokens(
    program: &Program<Rc<Keypair>>,
    topic_id: u64,
    amount: u64,
) -> Result<()> {
    let user = program.payer();
    let (user_profile_pda, _) = get_user_profile_pda(program, &user);
    let (topic_pda, _) = get_topic_pda(program, topic_id);
    let (state_pda, _) = get_state_pda(program);

    // Get mint addresses from state
    let state_data: StateAccount = program.account(state_pda)?;
    let temp_align_mint = state_data.temp_align_mint;
    let temp_rep_mint = state_data.temp_rep_mint;

    // Get token accounts
    let (user_temp_align_account_pda, _) =
        get_user_temp_token_account_pda(program, &user, "temp_align_account");
    let (user_temp_rep_account_pda, _) =
        get_user_temp_token_account_pda(program, &user, "temp_rep_account");

    println!(
        "Staking {} temp alignment tokens for topic #{}",
        amount, topic_id
    );

    let accounts = AccountsAll::StakeTopicSpecificTokens {
        user,
        user_profile: user_profile_pda,
        topic: topic_pda,
        state: state_pda,
        temp_align_mint,
        user_temp_align_account: user_temp_align_account_pda,
        temp_rep_mint,
        user_temp_rep_account: user_temp_rep_account_pda,
        token_program: anchor_spl::token::ID,
        system_program: system_program::ID,
        rent: sysvar::rent::ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::StakeTopicSpecificTokens { amount })
        .send()?;

    println!("Tokens staked successfully (txSig: {})", tx_sig);
    Ok(())
}
