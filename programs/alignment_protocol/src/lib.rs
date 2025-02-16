use anchor_lang::prelude::*;
use anchor_spl::token::Token;

declare_id!("FtHfXYCwuVEb8LVkNwNMmqMVooyg2fxkoT8i9bkEcvKW");

// Define the data structure for the state account.
#[account] // This attribute makes Anchor treat this struct as an account, and auto-implements serialization.
pub struct State {
    pub mint: Pubkey,      // Pubkey of the token mint created in initialize.
    pub authority: Pubkey, // Pubkey of the authority (e.g., the user who initializes, could be admin).
    pub bump: u8,          // Bump seed for the PDA (so we can sign with it in future if needed).
                           // (You can add more fields here as needed, e.g., config flags, counters, etc.)
}

// Define the Accounts context for the initialize instruction
#[derive(Accounts)]
#[instruction()] // no instruction args in this example, so this can be empty
pub struct Initialize<'info> {
    // The state account (PDA) to be initialized.
    #[account(
        init,                         // Anchor will create this account
        seeds = [b"state"],           // Seed for PDA (globally unique to this program)
        bump,                         // Store the bump on the account (we have a bump field in State to record it)
        payer = authority,            // The account that will pay the rent (and must sign) is `authority`
        space = 8 + 32 + 32 + 1       // Space for State: discriminator + 32 (mint) + 32 (authority) + 1 (bump)
    )]
    pub state: Account<'info, State>, // State account, will be created and owned by our program.

    // The token mint account to be created. We will create an SPL token mint for the protocol.
    #[account(
        init,                         // Create the mint account
        seeds = [b"mint"],            // Seed for PDA for the mint (so program has a dedicated mint address)
        bump,                         // Bump for the mint PDA (we might or might not store it; can derive on the fly later)
        payer = authority,            // The same authority pays for this account as well
        mint::decimals = 0,           // Example: 0 decimals (like an integer token). Adjust as needed (e.g., 6 for USDC-like).
        mint::authority = state.key(),// Set the minting authority to the state PDA (our program will control minting)
        mint::freeze_authority = state.key() // (Optional) We also set the freeze authority to the state PDA
        // Note: Using `mint::freeze_authority` is optional. If you don't need freeze capabilities, you can omit it.
    )]
    pub mint: Account<'info, anchor_spl::token::Mint>,
    // ^ We're using Anchor's SPL helper to treat this as a Mint account.
    // This account will be owned by the Token Program after creation.
    #[account(mut)]
    pub authority: Signer<'info>, // The user calling initialize (and paying for the creation). Must be a signer.

    /// CHECK: Anchor will ensure the token program is the correct one (by checking against the declared ID).
    pub token_program: Program<'info, Token>, // The SPL Token program (needed for CPI to initialize the mint)

    pub system_program: Program<'info, System>, // System program (to create accounts)
    pub rent: Sysvar<'info, Rent>,              // Rent sysvar, for rent-exemption calculations
}

// The program module where we implement our instructions
#[program]
mod alignment_protocol {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        // Get references to the accounts from the context for convenience
        let state_account = &mut ctx.accounts.state;
        let mint_account = &ctx.accounts.mint;
        let authority_account = &ctx.accounts.authority;

        // Set the state account data
        state_account.mint = mint_account.key(); // record the mint's public key in our state
        state_account.authority = authority_account.key(); // record who initialized (could be used as admin)
        state_account.bump = ctx.bumps.state;

        // Optionally, we can log or emit an event for debugging/audit
        msg!(
            "Initialized Alignment Protocol state. Mint: {}, Authority: {}",
            mint_account.key(),
            authority_account.key()
        );

        // If we had defined an event, we could emit it here (e.g., emit!(Initialized {...})).

        Ok(())
    }
}
