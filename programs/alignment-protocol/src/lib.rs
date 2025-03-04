use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{create, AssociatedToken, Create},
    token::{self, Mint, MintTo, Token, TokenAccount},
};

declare_id!("BMYn8rtstaZhzFZtgMVMY9io1zhnqacr3yANZrgkv7DF");

// ------------------------------
//          Data Structs
// ------------------------------

/// Global state account for this protocol
#[account]
pub struct State {
    /// The temporary alignment token mint (non-transferable until converted)
    pub temp_align_mint: Pubkey,
    
    /// The permanent alignment token mint (transferable)
    pub align_mint: Pubkey,
    
    /// The temporary reputation token mint (non-transferable)
    pub temp_rep_mint: Pubkey,
    
    /// The permanent reputation token mint (non-transferable)
    pub rep_mint: Pubkey,

    /// The protocol authority (admin, DAO, etc.)
    pub authority: Pubkey,

    /// Bump seed for the state PDA
    pub bump: u8,

    /// Counts how many submissions have been made
    pub submission_count: u64,

    /// The number of tokens to mint for each submission
    pub tokens_to_mint: u64,
}

/// Each submission entry
#[account]
pub struct Submission {
    /// The user who submitted the data
    pub contributor: Pubkey,

    /// Unix timestamp of when they submitted
    pub timestamp: u64,

    /// Arbitrary string to store data or a code-in TX reference
    pub data: String,
}

// ------------------------------
//          Error Codes
// ------------------------------
#[error_code]
pub enum ErrorCode {
    #[msg("Invalid authority for this state")]
    InvalidAuthority,

    #[msg("Arithmetic overflow occurred")]
    Overflow,
}

// ------------------------------
//          Instructions
// ------------------------------

/// Instruction: Initialize the protocol state + create a token mint
///
/// 1) Creates the `State` account (PDA with seeds=["state"]).
/// 2) Creates the `Mint` account (PDA with seeds=["mint"]).
/// 3) Sets `submission_count = 0`.
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        seeds = [b"state"],
        bump,
        payer = authority,
        space = 8 + 32 + 32 + 1 + 8 + 8 // Discriminator + mint + authority + bump + submission_count + tokens_to_mint
    )]
    pub state: Account<'info, State>,

    #[account(
        init,
        seeds = [b"mint"],
        bump,
        payer = authority,
        mint::decimals = 0,            // Adjust decimals as needed
        mint::authority = state.key(), // The state PDA is the mint authority
        mint::freeze_authority = state.key()
    )]
    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Instruction: Update the number of tokens to mint for each submission
///
/// 1) Updates the `tokens_to_mint` field in the `State` account.
/// 2) Requires the authority to sign.
#[derive(Accounts)]
pub struct UpdateTokensToMint<'info> {
    #[account(mut, has_one = authority)]
    pub state: Account<'info, State>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CreateUserAta<'info> {
    /// The person paying for creating the ATA
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The user for whom we want to create an ATA
    #[account(mut)]
    pub user: Signer<'info>,

    /// The mint for which we want the user's ATA
    #[account(mut)]
    pub mint: Account<'info, Mint>,

    /// The Associated Token Account (will be created if it doesn't exist)
    /// We do not use `init_if_needed`; we do a CPI call to the ATA program explicitly below.
    /// CHECK: We do not check the ATA account here because it's created by the ATA program.
    #[account(mut)]
    pub user_ata: UncheckedAccount<'info>,

    /// Programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

/// Instruction: Store data directly in your program's Submission account.
#[derive(Accounts)]
pub struct SubmitData<'info> {
    #[account(mut)]
    pub state: Account<'info, State>,

    /// The mint for temp tokens, must be mutable if we plan to mint more
    #[account(mut)]
    pub mint: Account<'info, Mint>,

    /// The user's ATA to receive minted tokens
    /// We only mark it mut. We assume it's already created via `create_user_ata`.
    #[account(mut)]
    pub contributor_ata: Account<'info, TokenAccount>,

    /// The new Submission account
    #[account(
        init,
        payer = contributor,
        // Use seeds to ensure uniqueness
        seeds = [
            b"submission",
            state.submission_count.to_le_bytes().as_ref(),
        ],
        bump,
        // Discriminator + contributor pubkey + timestamp + data field (4 + your chosen max length)
        space = 8 + 32 + 8 + (4 + 256)
    )]
    pub submission: Account<'info, Submission>,

    /// The user making the submission
    #[account(mut)]
    pub contributor: Signer<'info>,

    /// We do NOT require the authority to sign, we only check `state.authority` to match
    /// so we pass it as a normal AccountInfo if needed or omit it if not used
    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// ------------------------------
//          Program Logic
// ------------------------------
#[program]
pub mod alignment_protocol {
    use super::*;

    /// Instruction handler: initialize the protocol
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let state_acc = &mut ctx.accounts.state;
        state_acc.mint = ctx.accounts.mint.key();
        state_acc.authority = ctx.accounts.authority.key();
        state_acc.bump = ctx.bumps.state;
        state_acc.submission_count = 0;
        state_acc.tokens_to_mint = 0;
        msg!("Initialized protocol. Mint = {}", state_acc.mint);
        Ok(())
    }

    /// Instruction handler: update the number of tokens to mint for each submission
    pub fn update_tokens_to_mint(
        ctx: Context<UpdateTokensToMint>,
        new_tokens_to_mint: u64,
    ) -> Result<()> {
        let state_acc = &mut ctx.accounts.state;
        let previous_tokens_to_mint = state_acc.tokens_to_mint;
        state_acc.tokens_to_mint = new_tokens_to_mint;
        msg!(
            "Updated tokens_to_mint from {} to {}",
            previous_tokens_to_mint,
            new_tokens_to_mint
        );
        Ok(())
    }

    /// Instruction handler: explicitly create user's ATA
    ///
    /// This does NOT use `init_if_needed`. Instead, it does a CPI to the associated_token::create method.
    /// If the ATA already exists, this transaction will fail (unless you do extra checks).
    pub fn create_user_ata(ctx: Context<CreateUserAta>) -> Result<()> {
        // Build a CPI context for the associated token program
        let cpi_ctx = CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            Create {
                payer: ctx.accounts.payer.to_account_info(),
                associated_token: ctx.accounts.user_ata.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
        );

        // If the ATA already exists, create(...) will throw an error
        create(cpi_ctx)?;

        msg!("Created ATA for user {}", ctx.accounts.user.key());
        Ok(())
    }

    /// Instruction handler: Submit data directly on-chain
    /// 1) Creates new `Submission` account with the given data.
    /// 2) Mints a fixed number of tokens from the State's mint to the user's ATA.
    /// 3) Increments the state's submission_count.
    pub fn submit_data(ctx: Context<SubmitData>, data_str: String) -> Result<()> {
        // 1) Fill out the Submission account
        let submission = &mut ctx.accounts.submission;
        submission.contributor = ctx.accounts.contributor.key();
        submission.timestamp = Clock::get()?.unix_timestamp as u64;
        submission.data = data_str.clone(); // store the text or JSON

        // 2) Mint tokens to the contributor
        if ctx.accounts.state.tokens_to_mint > 0 {
            let state_bump = ctx.accounts.state.bump;
            let seeds = &[b"state".as_ref(), &[state_bump]];
            let signer = &[&seeds[..]];

            // CPI to the Token Program's 'mint_to'
            let cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.contributor_ata.to_account_info(),
                    authority: ctx.accounts.state.to_account_info(),
                },
            )
            .with_signer(signer);

            token::mint_to(cpi_ctx, ctx.accounts.state.tokens_to_mint)?;
            msg!(
                "Minted {} tokens to {}",
                ctx.accounts.state.tokens_to_mint,
                ctx.accounts.contributor_ata.key()
            );
        }

        // 3) Increment submission_count
        let state_acc = &mut ctx.accounts.state;
        state_acc.submission_count = state_acc
            .submission_count
            .checked_add(1)
            .ok_or(ErrorCode::Overflow)?;

        msg!("New submission on-chain: {}", data_str);
        Ok(())
    }
}
