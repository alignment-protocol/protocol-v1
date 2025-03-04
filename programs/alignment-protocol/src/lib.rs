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
    
    /// Counts of yes votes received
    pub yes_count: u64,
    
    /// Counts of no votes received
    pub no_count: u64,
    
    /// Status of the submission
    pub status: SubmissionStatus,
}

/// Status of a submission
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum SubmissionStatus {
    /// Submission is pending voting/finalization
    Pending,
    
    /// Submission has been accepted by voters
    Accepted,
    
    /// Submission has been rejected by voters
    Rejected,
}

/// User profile account to track reputation
#[account]
pub struct UserProfile {
    /// The user's public key
    pub user: Pubkey,
    
    /// Amount of temporary reputation tokens staked
    pub temp_rep_amount: u64,
    
    /// Amount of permanent reputation tokens earned
    pub permanent_rep_amount: u64,
    
    /// Bump seed for the user profile PDA
    pub bump: u8,
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
}

// ------------------------------
//          Instructions
// ------------------------------

/// Instruction: Initialize the protocol state + create four token mints
///
/// 1) Creates the `State` account (PDA with seeds=["state"]).
/// 2) Creates the four token mint accounts with different seeds and properties:
///   - temp_align_mint: Non-transferable temporary alignment tokens (seeds=["temp_align_mint"])
///   - align_mint: Transferable permanent alignment tokens (seeds=["align_mint"])
///   - temp_rep_mint: Non-transferable temporary reputation tokens (seeds=["temp_rep_mint"])
///   - rep_mint: Non-transferable permanent reputation tokens (seeds=["rep_mint"])
/// 3) Sets `submission_count = 0`.
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        seeds = [b"state"],
        bump,
        payer = authority,
        space = 8 + 32 + 32 + 32 + 32 + 32 + 1 + 8 + 8 // Discriminator + 4 mints + authority + bump + submission_count + tokens_to_mint
    )]
    pub state: Account<'info, State>,

    #[account(
        init,
        seeds = [b"temp_align_mint"],
        bump,
        payer = authority,
        mint::decimals = 0,            
        mint::authority = state.key(), // The state PDA is the mint authority
        mint::freeze_authority = state.key()
    )]
    pub temp_align_mint: Account<'info, Mint>,

    #[account(
        init,
        seeds = [b"align_mint"],
        bump,
        payer = authority,
        mint::decimals = 0,            
        mint::authority = state.key(), 
        mint::freeze_authority = state.key()
    )]
    pub align_mint: Account<'info, Mint>,

    #[account(
        init,
        seeds = [b"temp_rep_mint"],
        bump,
        payer = authority,
        mint::decimals = 0,            
        mint::authority = state.key(), 
        mint::freeze_authority = state.key()
    )]
    pub temp_rep_mint: Account<'info, Mint>,

    #[account(
        init,
        seeds = [b"rep_mint"],
        bump,
        payer = authority,
        mint::decimals = 0,            
        mint::authority = state.key(), 
        mint::freeze_authority = state.key()
    )]
    pub rep_mint: Account<'info, Mint>,

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
    /// The state account containing all mint references
    pub state: Account<'info, State>,

    /// The person paying for creating the ATA
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The user for whom we want to create an ATA
    #[account(mut)]
    pub user: Signer<'info>,

    /// The mint for which we want the user's ATA (must match one of the four mints in state)
    #[account(mut, constraint = 
        *mint.to_account_info().key == state.temp_align_mint || 
        *mint.to_account_info().key == state.align_mint || 
        *mint.to_account_info().key == state.temp_rep_mint || 
        *mint.to_account_info().key == state.rep_mint
    )]
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

/// Instruction: Store data directly in your program's Submission account and mint temporary alignment tokens.
#[derive(Accounts)]
pub struct SubmitData<'info> {
    #[account(mut)]
    pub state: Account<'info, State>,

    /// The temporary alignment token mint, must be mutable for minting
    #[account(
        mut,
        constraint = *temp_align_mint.to_account_info().key == state.temp_align_mint
    )]
    pub temp_align_mint: Account<'info, Mint>,

    /// The user's ATA for temporary alignment tokens
    /// We only mark it mut. We assume it's already created via `create_user_ata`.
    #[account(
        mut,
        constraint = contributor_ata.mint == state.temp_align_mint,
        constraint = contributor_ata.owner == contributor.key()
    )]
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
        // Discriminator + contributor pubkey + timestamp + data field (4 + your chosen max length) + yes_count + no_count + status
        space = 8 + 32 + 8 + (4 + 256) + 8 + 8 + 1
    )]
    pub submission: Account<'info, Submission>,

    /// The user making the submission
    #[account(mut)]
    pub contributor: Signer<'info>,

    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Account constraints for staking temporary alignment tokens
#[derive(Accounts)]
pub struct StakeAlignmentTokens<'info> {
    #[account(mut)]
    pub state: Account<'info, State>,
    
    /// The user's profile PDA to track reputation
    /// This will be checked/initialized in the instruction handler
    /// CHECK: Verified in the instruction handler
    #[account(mut)]
    pub user_profile: UncheckedAccount<'info>,
    
    /// The temporary alignment token mint (source tokens to burn)
    #[account(
        mut,
        constraint = *temp_align_mint.to_account_info().key == state.temp_align_mint
    )]
    pub temp_align_mint: Account<'info, Mint>,
    
    /// The temporary reputation token mint (target tokens to mint)
    #[account(
        mut,
        constraint = *temp_rep_mint.to_account_info().key == state.temp_rep_mint
    )]
    pub temp_rep_mint: Account<'info, Mint>,
    
    /// The user's ATA for temporary alignment tokens (source)
    #[account(
        mut,
        constraint = user_temp_align_ata.mint == state.temp_align_mint,
        constraint = user_temp_align_ata.owner == user.key()
    )]
    pub user_temp_align_ata: Account<'info, TokenAccount>,
    
    /// The user's ATA for temporary reputation tokens (target)
    #[account(
        mut,
        constraint = user_temp_rep_ata.mint == state.temp_rep_mint,
        constraint = user_temp_rep_ata.owner == user.key()
    )]
    pub user_temp_rep_ata: Account<'info, TokenAccount>,
    
    /// The user performing the stake
    #[account(mut)]
    pub user: Signer<'info>,
    
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

    /// Instruction handler: initialize the protocol with four token mints
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let state_acc = &mut ctx.accounts.state;
        
        // Store all four token mint addresses
        state_acc.temp_align_mint = ctx.accounts.temp_align_mint.key();
        state_acc.align_mint = ctx.accounts.align_mint.key();
        state_acc.temp_rep_mint = ctx.accounts.temp_rep_mint.key();
        state_acc.rep_mint = ctx.accounts.rep_mint.key();
        
        // Set other state properties
        state_acc.authority = ctx.accounts.authority.key();
        state_acc.bump = ctx.bumps.state;
        state_acc.submission_count = 0;
        state_acc.tokens_to_mint = 0;
        
        msg!("Initialized protocol with four token mints:");
        msg!("temp_align_mint = {}", state_acc.temp_align_mint);
        msg!("align_mint = {}", state_acc.align_mint);
        msg!("temp_rep_mint = {}", state_acc.temp_rep_mint);
        msg!("rep_mint = {}", state_acc.rep_mint);
        
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
    /// 2) Mints a fixed number of temporary alignment tokens to the user's ATA.
    /// 3) Increments the state's submission_count.
    pub fn submit_data(ctx: Context<SubmitData>, data_str: String) -> Result<()> {
        // 1) Fill out the Submission account
        let submission = &mut ctx.accounts.submission;
        submission.contributor = ctx.accounts.contributor.key();
        submission.timestamp = Clock::get()?.unix_timestamp as u64;
        submission.data = data_str.clone(); // store the text or JSON
        submission.yes_count = 0;
        submission.no_count = 0;
        submission.status = SubmissionStatus::Pending;

        // 2) Mint temporary alignment tokens to the contributor
        if ctx.accounts.state.tokens_to_mint > 0 {
            let state_bump = ctx.accounts.state.bump;
            let seeds = &[b"state".as_ref(), &[state_bump]];
            let signer = &[&seeds[..]];

            // CPI to the Token Program's 'mint_to'
            let cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.temp_align_mint.to_account_info(),
                    to: ctx.accounts.contributor_ata.to_account_info(),
                    authority: ctx.accounts.state.to_account_info(),
                },
            )
            .with_signer(signer);

            token::mint_to(cpi_ctx, ctx.accounts.state.tokens_to_mint)?;
            msg!(
                "Minted {} tempAlign tokens to {}",
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
