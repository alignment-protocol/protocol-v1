use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::{
    solana_sdk::{system_program, sysvar},
    Program,
};
use anyhow::Result;
use std::rc::Rc;

use alignment_protocol::{accounts as AccountsAll, instruction as InstructionAll};
use anchor_spl;

use crate::utils::pda::{get_mint_pda, get_state_pda};

/// Initialize the protocol state account
pub fn cmd_init_state(program: &Program<Rc<Keypair>>) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);

    println!("Initializing protocol state at {}", state_pda);

    let accounts = AccountsAll::InitializeState {
        authority: program.payer(),
        state: state_pda,
        system_program: system_program::ID,
        rent: sysvar::rent::ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeState {})
        .send()?;

    println!("State initialized successfully (txSig: {})", tx_sig);
    Ok(())
}

/// Initialize the temporary alignment token mint
pub fn cmd_init_temp_align_mint(program: &Program<Rc<Keypair>>) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let (temp_align_mint_pda, _) = get_mint_pda(program, "temp_align_mint");

    println!(
        "Initializing temporary alignment token mint at {}",
        temp_align_mint_pda
    );

    let accounts = AccountsAll::InitializeTempAlignMint {
        state: state_pda,
        temp_align_mint: temp_align_mint_pda,
        authority: program.payer(),
        token_program: anchor_spl::token::ID,
        system_program: system_program::ID,
        rent: sysvar::rent::ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeTempAlignMint {})
        .send()?;

    println!(
        "Temporary alignment token mint initialized successfully (txSig: {})",
        tx_sig
    );
    Ok(())
}

/// Initialize the permanent alignment token mint
pub fn cmd_init_align_mint(program: &Program<Rc<Keypair>>) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let (align_mint_pda, _) = get_mint_pda(program, "align_mint");

    println!(
        "Initializing permanent alignment token mint at {}",
        align_mint_pda
    );

    let accounts = AccountsAll::InitializeAlignMint {
        state: state_pda,
        align_mint: align_mint_pda,
        authority: program.payer(),
        token_program: anchor_spl::token::ID,
        system_program: system_program::ID,
        rent: sysvar::rent::ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeAlignMint {})
        .send()?;

    println!(
        "Permanent alignment token mint initialized successfully (txSig: {})",
        tx_sig
    );
    Ok(())
}

/// Initialize the temporary reputation token mint
pub fn cmd_init_temp_rep_mint(program: &Program<Rc<Keypair>>) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let (temp_rep_mint_pda, _) = get_mint_pda(program, "temp_rep_mint");

    println!(
        "Initializing temporary reputation token mint at {}",
        temp_rep_mint_pda
    );

    let accounts = AccountsAll::InitializeTempRepMint {
        state: state_pda,
        temp_rep_mint: temp_rep_mint_pda,
        authority: program.payer(),
        token_program: anchor_spl::token::ID,
        system_program: system_program::ID,
        rent: sysvar::rent::ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeTempRepMint {})
        .send()?;

    println!(
        "Temporary reputation token mint initialized successfully (txSig: {})",
        tx_sig
    );
    Ok(())
}

/// Initialize the permanent reputation token mint
pub fn cmd_init_rep_mint(program: &Program<Rc<Keypair>>) -> Result<()> {
    let (state_pda, _) = get_state_pda(program);
    let (rep_mint_pda, _) = get_mint_pda(program, "rep_mint");

    println!(
        "Initializing permanent reputation token mint at {}",
        rep_mint_pda
    );

    let accounts = AccountsAll::InitializeRepMint {
        state: state_pda,
        rep_mint: rep_mint_pda,
        authority: program.payer(),
        token_program: anchor_spl::token::ID,
        system_program: system_program::ID,
        rent: sysvar::rent::ID,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeRepMint {})
        .send()?;

    println!(
        "Permanent reputation token mint initialized successfully (txSig: {})",
        tx_sig
    );
    Ok(())
}

/// Update the number of tokens to mint per submission
pub fn cmd_update_tokens_to_mint(program: &Program<Rc<Keypair>>, tokens: u64) -> Result<()> {
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
