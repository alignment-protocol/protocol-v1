use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::solana_sdk::system_program;
use anchor_client::Program;
use anchor_lang::prelude::Pubkey;
use anchor_spl::token::ID as TokenProgramID;
use anyhow::{anyhow, Result};
use std::rc::Rc;
use std::str::FromStr;

use alignment_protocol::{
    accounts as AccountsAll, data::State as StateAccount, instruction as InstructionAll,
};

use crate::commands::common::pda::{get_mint_pda, get_state_pda};

/// Check if the protocol state is already initialized
pub fn is_state_initialized(program: &Program<Rc<Keypair>>) -> bool {
    let (state_pda, _) = get_state_pda(program);
    program.account::<StateAccount>(state_pda).is_ok()
}

/// Check if all token mints have been initialized
pub fn are_mints_initialized(program: &Program<Rc<Keypair>>) -> bool {
    let (state_pda, _) = get_state_pda(program);

    match program.account::<StateAccount>(state_pda) {
        Ok(state) => {
            state.temp_align_mint != Pubkey::default()
                && state.align_mint != Pubkey::default()
                && state.temp_rep_mint != Pubkey::default()
                && state.rep_mint != Pubkey::default()
        }
        Err(_) => false,
    }
}

/// Initialize protocol state account
pub fn cmd_init_state(program: &Program<Rc<Keypair>>, oracle_pubkey_str: String) -> Result<()> {
    // Check if state is already initialized
    if is_state_initialized(program) {
        println!("Protocol state is already initialized.");
        return Ok(());
    }

    // Parse the oracle pubkey string
    let oracle_pubkey = Pubkey::from_str(&oracle_pubkey_str)
        .map_err(|e| anyhow!("Invalid oracle pubkey provided: {}", e))?;

    let (state_pda, _) = get_state_pda(program);

    println!("Initializing protocol state account...");
    println!("  Oracle Pubkey: {}", oracle_pubkey);

    // Get the rent sysvar
    let rent = anchor_client::solana_sdk::sysvar::rent::ID;

    let accounts = AccountsAll::InitializeState {
        authority: program.payer(),
        state: state_pda,
        system_program: system_program::ID,
        rent,
    };

    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeState { oracle_pubkey })
        .send()?;

    println!("Protocol state account initialized (txSig: {})", tx_sig);
    Ok(())
}

/// Initialize temporary alignment token mint
pub fn cmd_init_temp_align_mint(program: &Program<Rc<Keypair>>) -> Result<()> {
    // Check if state exists first
    if !is_state_initialized(program) {
        return Err(anyhow!(
            "Protocol state not initialized. Run 'init state' first."
        ));
    }

    // Check if this mint already exists
    let (state_pda, _) = get_state_pda(program);
    let state = program.account::<StateAccount>(state_pda)?;
    if state.temp_align_mint != Pubkey::default() {
        println!("Temporary alignment token mint is already initialized.");
        return Ok(());
    }

    let (temp_align_mint_pda, _) = get_mint_pda(program, "temp_align_mint");

    println!("Initializing temporary alignment token mint...");

    // Get the rent sysvar
    let rent = anchor_client::solana_sdk::sysvar::rent::ID;

    let accounts = AccountsAll::InitializeTempAlignMint {
        authority: program.payer(),
        state: state_pda,
        temp_align_mint: temp_align_mint_pda,
        token_program: TokenProgramID,
        system_program: system_program::ID,
        rent,
    };

    // No arguments needed, bumps are handled by Anchor
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeTempAlignMint {})
        .send()?;

    println!(
        "Temporary alignment token mint initialized (txSig: {})",
        tx_sig
    );
    Ok(())
}

/// Initialize permanent alignment token mint
pub fn cmd_init_align_mint(program: &Program<Rc<Keypair>>) -> Result<()> {
    // Check if state exists first
    if !is_state_initialized(program) {
        return Err(anyhow!(
            "Protocol state not initialized. Run 'init state' first."
        ));
    }

    // Check if this mint already exists
    let (state_pda, _) = get_state_pda(program);
    let state = program.account::<StateAccount>(state_pda)?;
    if state.align_mint != Pubkey::default() {
        println!("Permanent alignment token mint is already initialized.");
        return Ok(());
    }

    let (align_mint_pda, _) = get_mint_pda(program, "align_mint");

    println!("Initializing permanent alignment token mint...");

    // Get the rent sysvar
    let rent = anchor_client::solana_sdk::sysvar::rent::ID;

    let accounts = AccountsAll::InitializeAlignMint {
        authority: program.payer(),
        state: state_pda,
        align_mint: align_mint_pda,
        token_program: TokenProgramID,
        system_program: system_program::ID,
        rent,
    };

    // No arguments needed, bumps are handled by Anchor
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeAlignMint {})
        .send()?;

    println!(
        "Permanent alignment token mint initialized (txSig: {})",
        tx_sig
    );
    Ok(())
}

/// Initialize temporary reputation token mint
pub fn cmd_init_temp_rep_mint(program: &Program<Rc<Keypair>>) -> Result<()> {
    // Check if state exists first
    if !is_state_initialized(program) {
        return Err(anyhow!(
            "Protocol state not initialized. Run 'init state' first."
        ));
    }

    // Check if this mint already exists
    let (state_pda, _) = get_state_pda(program);
    let state = program.account::<StateAccount>(state_pda)?;
    if state.temp_rep_mint != Pubkey::default() {
        println!("Temporary reputation token mint is already initialized.");
        return Ok(());
    }

    let (temp_rep_mint_pda, _) = get_mint_pda(program, "temp_rep_mint");

    println!("Initializing temporary reputation token mint...");

    // Get the rent sysvar
    let rent = anchor_client::solana_sdk::sysvar::rent::ID;

    let accounts = AccountsAll::InitializeTempRepMint {
        authority: program.payer(),
        state: state_pda,
        temp_rep_mint: temp_rep_mint_pda,
        token_program: TokenProgramID,
        system_program: system_program::ID,
        rent,
    };

    // No arguments needed, bumps are handled by Anchor
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeTempRepMint {})
        .send()?;

    println!(
        "Temporary reputation token mint initialized (txSig: {})",
        tx_sig
    );
    Ok(())
}

/// Initialize permanent reputation token mint
pub fn cmd_init_rep_mint(program: &Program<Rc<Keypair>>) -> Result<()> {
    // Check if state exists first
    if !is_state_initialized(program) {
        return Err(anyhow!(
            "Protocol state not initialized. Run 'init state' first."
        ));
    }

    // Check if this mint already exists
    let (state_pda, _) = get_state_pda(program);
    let state = program.account::<StateAccount>(state_pda)?;
    if state.rep_mint != Pubkey::default() {
        println!("Permanent reputation token mint is already initialized.");
        return Ok(());
    }

    let (rep_mint_pda, _) = get_mint_pda(program, "rep_mint");

    println!("Initializing permanent reputation token mint...");

    // Get the rent sysvar
    let rent = anchor_client::solana_sdk::sysvar::rent::ID;

    let accounts = AccountsAll::InitializeRepMint {
        authority: program.payer(),
        state: state_pda,
        rep_mint: rep_mint_pda,
        token_program: TokenProgramID,
        system_program: system_program::ID,
        rent,
    };

    // No arguments needed, bumps are handled by Anchor
    let tx_sig = program
        .request()
        .accounts(accounts)
        .args(InstructionAll::InitializeRepMint {})
        .send()?;

    println!(
        "Permanent reputation token mint initialized (txSig: {})",
        tx_sig
    );
    Ok(())
}

/// Initialize all protocol accounts
pub fn cmd_init_all(program: &Program<Rc<Keypair>>, oracle_pubkey_str: String) -> Result<()> {
    println!("Initializing all protocol accounts...");

    // Check if everything is already initialized
    if is_state_initialized(program) && are_mints_initialized(program) {
        println!("All protocol accounts are already initialized.");
        return Ok(());
    }

    // Initialize state first
    match cmd_init_state(program, oracle_pubkey_str) {
        Ok(_) => println!("[DEBUG] State initialization successful"),
        Err(e) => {
            println!("[DEBUG] State initialization failed: {}", e);
            return Err(e);
        }
    }

    // Then initialize all token mints
    match cmd_init_temp_align_mint(program) {
        Ok(_) => println!("[DEBUG] TempAlign mint initialization successful"),
        Err(e) => {
            println!("[DEBUG] TempAlign mint initialization failed: {}", e);
            return Err(e);
        }
    }

    match cmd_init_align_mint(program) {
        Ok(_) => println!("[DEBUG] Align mint initialization successful"),
        Err(e) => {
            println!("[DEBUG] Align mint initialization failed: {}", e);
            return Err(e);
        }
    }

    match cmd_init_temp_rep_mint(program) {
        Ok(_) => println!("[DEBUG] TempRep mint initialization successful"),
        Err(e) => {
            println!("[DEBUG] TempRep mint initialization failed: {}", e);
            return Err(e);
        }
    }

    match cmd_init_rep_mint(program) {
        Ok(_) => println!("[DEBUG] Rep mint initialization successful"),
        Err(e) => {
            println!("[DEBUG] Rep mint initialization failed: {}", e);
            return Err(e);
        }
    }

    println!("All protocol accounts initialized successfully!");
    Ok(())
}
