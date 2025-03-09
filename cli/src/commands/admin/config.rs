use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use anyhow::{anyhow, Result};
use std::rc::Rc;

use alignment_protocol::{accounts as AccountsAll, instruction as InstructionAll};

use crate::commands::admin::init::is_state_initialized;
use crate::commands::common::pda::get_state_pda;

/// Update the number of tokens to mint per submission (admin only)
pub fn cmd_admin_update_tokens_to_mint(program: &Program<Rc<Keypair>>, tokens: u64) -> Result<()> {
    // Check if state is initialized
    if !is_state_initialized(program) {
        return Err(anyhow!(
            "Protocol state not initialized. Run 'init state' first."
        ));
    }

    let (state_pda, _) = get_state_pda(program);

    println!("Updating tokens to mint to {}", tokens);

    let accounts = AccountsAll::UpdateTokensToMint {
        authority: program.payer(),
        state: state_pda,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::UpdateTokensToMint {
            new_tokens_to_mint: tokens,
        })
        .send()?;

    println!("Tokens to mint updated successfully (txSig: {})", tx_sig);
    Ok(())
}
