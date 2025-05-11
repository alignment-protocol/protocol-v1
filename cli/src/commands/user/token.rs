use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use anyhow::Result;
use std::rc::Rc;

use alignment_protocol::{
    accounts as AccountsAll, instruction as InstructionAll, State as StateAccount,
};

use crate::commands::common::pda::{
    get_state_pda, get_topic_pda, get_user_profile_pda, get_user_temp_token_account_pda,
    get_user_topic_balance_pda,
};

/// Stake temporary alignment tokens for a topic to earn reputation
pub fn cmd_stake_topic_specific_tokens(
    program: &Program<Rc<Keypair>>,
    topic_index: u64,
    amount: u64,
) -> Result<()> {
    let user = program.payer();
    let (user_profile_pda, _) = get_user_profile_pda(program, &user);
    let (topic_pda, _) = get_topic_pda(program, topic_index);
    let (state_pda, _) = get_state_pda(program);
    let (user_topic_balance_pda, _) = get_user_topic_balance_pda(program, &user, &topic_pda);

    // Get mint addresses from state
    let state_data: StateAccount = program.account(state_pda)?;
    let temp_align_mint = state_data.temp_align_mint;
    let temp_rep_mint = state_data.temp_rep_mint;

    // Get token accounts
    let (user_temp_align_account_pda, _) =
        get_user_temp_token_account_pda(program, &user, "user_temp_align");
    let (user_temp_rep_account_pda, _) =
        get_user_temp_token_account_pda(program, &user, "user_temp_rep");

    // Check if user profile and token accounts exist
    let profile_exists = program.rpc().get_account(&user_profile_pda).is_ok();
    let temp_align_account_exists = program
        .rpc()
        .get_account(&user_temp_align_account_pda)
        .is_ok();
    let temp_rep_account_exists = program
        .rpc()
        .get_account(&user_temp_rep_account_pda)
        .is_ok();

    if !profile_exists || !temp_align_account_exists || !temp_rep_account_exists {
        return Err(anyhow::anyhow!(
            "User profile or token accounts not set up. Please run 'alignment-protocol-cli user create-profile' first."
        ));
    }

    println!(
        "Staking {} temp alignment tokens for topic #{}",
        amount, topic_index
    );

    let accounts = AccountsAll::StakeTopicSpecificTokens {
        user,
        payer: user,
        user_profile: user_profile_pda,
        topic: topic_pda,
        state: state_pda,
        user_topic_balance: user_topic_balance_pda,
        temp_align_mint,
        user_temp_align_account: user_temp_align_account_pda,
        temp_rep_mint,
        user_temp_rep_account: user_temp_rep_account_pda,
        token_program: anchor_spl::token::ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::StakeTopicSpecificTokens { amount })
        .send()?;

    println!("Tokens staked successfully (txSig: {})", tx_sig);
    Ok(())
}
