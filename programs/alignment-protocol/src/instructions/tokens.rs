use crate::contexts::{
    CreateUserAta, CreateUserTempAlignAccount, CreateUserTempRepAccount, StakePermanentAlignForRep,
    StakeTopicSpecificTokens,
};
use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{create, Create},
    token::{self, Burn, MintTo},
};

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

    // Update the user profile with the newly created ATA address
    let user_profile = &mut ctx.accounts.user_profile;
    let user_ata_key = ctx.accounts.user_ata.key();
    let mint_key = ctx.accounts.mint.key();
    let state = &ctx.accounts.state;

    if mint_key == state.align_mint {
        user_profile.user_align_ata = user_ata_key;
        msg!(
            "Updated user {} profile with Align ATA {}",
            ctx.accounts.user.key(),
            user_ata_key
        );
    } else if mint_key == state.rep_mint {
        user_profile.user_rep_ata = user_ata_key;
        msg!(
            "Updated user {} profile with Rep ATA {}",
            ctx.accounts.user.key(),
            user_ata_key
        );
    } else {
        // This case should be prevented by the context constraints, but good practice to handle
        return Err(ErrorCode::TokenMintMismatch.into());
    }

    msg!(
        "Created permanent token ATA {} for user {}",
        user_ata_key,
        ctx.accounts.user.key()
    );
    Ok(())
}

/// Creates a protocol-owned tempAlign token account for a user
///
/// This creates a token account that:
/// 1. Is owned by the protocol (state PDA) rather than the user
/// 2. Has the state PDA as the authority, allowing burns without user signature
/// 3. Uses PDA with seeds ["user_temp_align", user.key()]
/// 4. Updates the user's profile with the account address
pub fn create_user_temp_align_account(ctx: Context<CreateUserTempAlignAccount>) -> Result<()> {
    // The token account is initialized in the context with proper ownership and authority

    // Update the user profile with the newly created token account address
    let user_profile = &mut ctx.accounts.user_profile;
    let token_account_key = ctx.accounts.token_account.key();
    user_profile.user_temp_align_account = token_account_key;

    msg!(
        "Created protocol-owned tempAlign token account {} for user {}",
        token_account_key,
        ctx.accounts.user.key()
    );
    msg!(
        "Updated user {} profile with tempAlign account {}",
        ctx.accounts.user.key(),
        token_account_key
    );

    Ok(())
}

/// Creates a protocol-owned tempRep token account for a user
///
/// This creates a token account that:
/// 1. Is owned by the protocol (state PDA) rather than the user
/// 2. Has the state PDA as the authority, allowing burns without user signature
/// 3. Uses PDA with seeds ["user_temp_rep", user.key()]
/// 4. Updates the user's profile with the account address
pub fn create_user_temp_rep_account(ctx: Context<CreateUserTempRepAccount>) -> Result<()> {
    // The token account is initialized in the context with proper ownership and authority

    // Update the user profile with the newly created token account address
    let user_profile = &mut ctx.accounts.user_profile;
    let token_account_key = ctx.accounts.token_account.key();
    user_profile.user_temp_rep_account = token_account_key;

    msg!(
        "Created protocol-owned tempRep token account {} for user {}",
        token_account_key,
        ctx.accounts.user.key()
    );
    msg!(
        "Updated user {} profile with tempRep account {}",
        ctx.accounts.user.key(),
        token_account_key
    );

    Ok(())
}

// Removed legacy stake_alignment_tokens function

/// Stakes tempAlign tokens for a specific topic to earn topic-specific tempRep
/// Burns from the protocol-owned tempAlign PDA and Mints to the protocol-owned tempRep PDA.
/// Updates the balances tracked in the UserTopicBalance account.
pub fn stake_topic_specific_tokens(
    ctx: Context<StakeTopicSpecificTokens>,
    amount: u64,
) -> Result<()> {
    // Validate the stake amount
    if amount == 0 {
        // Use existing error code from error.rs
        return Err(ErrorCode::ZeroStakeAmount.into());
    }

    let user_topic_balance = &mut ctx.accounts.user_topic_balance;
    let state = &ctx.accounts.state;
    let user = &ctx.accounts.user;
    let topic = &ctx.accounts.topic; // Needed for logging/events

    // 1. Check if the user has enough *allocated* tempAlign in UserTopicBalance for this topic
    // Although the global tempAlign PDA might have tokens, this checks the topic-specific assignment.
    // NOTE: This check assumes tempAlign is *consumed* during staking. If tempAlign should remain
    // in UserTopicBalance and only tempRep increases, this check needs adjustment.
    // Based on the variable names, consuming tempAlign to produce tempRep seems intended.
    if user_topic_balance.temp_align_amount < amount {
        // Use existing error code from error.rs
        return Err(ErrorCode::InsufficientTopicTokens.into());
    }

    // 2. Check if the global protocol-owned tempAlign account has enough tokens to burn
    // This is a sanity check; usually the UserTopicBalance check above should suffice if balances are synced.
    if ctx.accounts.user_temp_align_account.amount < amount {
        // Use existing error code from error.rs
        return Err(ErrorCode::InsufficientTokenBalance.into());
    }

    // Get the state PDA signer seeds for CPI calls (authority is the state PDA)
    let state_bump = state.bump;
    let state_seeds = &[b"state".as_ref(), &[state_bump]];
    let signer = &[&state_seeds[..]];

    // 3. Burn the temporary alignment tokens from the protocol-owned PDA
    let burn_cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Burn {
            mint: ctx.accounts.temp_align_mint.to_account_info(),
            from: ctx.accounts.user_temp_align_account.to_account_info(),
            authority: state.to_account_info(), // State PDA is the authority
        },
        signer, // Provide state PDA seeds as signer
    );
    token::burn(burn_cpi_ctx, amount)?;

    // 4. Mint temporary reputation tokens into the protocol-owned PDA
    let mint_cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
            mint: ctx.accounts.temp_rep_mint.to_account_info(),
            to: ctx.accounts.user_temp_rep_account.to_account_info(),
            authority: state.to_account_info(), // State PDA is the authority
        },
        signer, // Provide state PDA seeds as signer
    );
    token::mint_to(mint_cpi_ctx, amount)?;

    // 5. Update the topic-specific balances in the UserTopicBalance account
    // Decrease tempAlign for this topic
    user_topic_balance.temp_align_amount = user_topic_balance
        .temp_align_amount
        .checked_sub(amount)
        .ok_or(ErrorCode::Overflow)?;

    // Increase tempRep for this topic
    user_topic_balance.temp_rep_amount = user_topic_balance
        .temp_rep_amount
        .checked_add(amount)
        .ok_or(ErrorCode::Overflow)?;

    msg!(
        "User {} staked {} tempAlign for topic {}, received {} tempRep. New topic balances: Align={}, Rep={}",
        user.key(),
        amount,
        topic.key(), // Using topic key for clarity, could use topic.id
        amount,
        user_topic_balance.temp_align_amount,
        user_topic_balance.temp_rep_amount
    );

    // (Optional) Emit event
    // emit!(TokensStaked { ... });

    Ok(())
}

/// Stakes permanent ALIGN tokens for permanent REP tokens with diminishing returns.
///
/// - Burns `amount` of ALIGN from `user_align_ata` (authority: `payer`).
/// - Mints REP to `user_rep_ata` (authority: `payer`, mint authority: `state` PDA).
/// - REP minted is calculated as: `amount / (1 + k * sqrt(current_total_rep_supply))`,
///   where `k` is `diminishing_k_value / K_SCALING_FACTOR` from `State`.
pub fn stake_permanent_align_for_rep(
    ctx: Context<StakePermanentAlignForRep>,
    amount: u64,
) -> Result<()> {
    msg!(
        "Staking {} permanent ALIGN for permanent REP for user {} (via payer {})",
        amount,
        ctx.accounts.user.key(),
        ctx.accounts.payer.key()
    );

    if amount == 0 {
        return err!(ErrorCode::ZeroStakeAmount);
    }
    // diminishing_k_value is expected to be set during state initialization/update
    if ctx.accounts.state.diminishing_k_value == 0 {
        // If k=0, formula becomes rep_minted = align_staked. This might be valid if intended.
        // However, if k is meant for diminishing returns, k=0 implies no diminishing effect.
        // We'll consider k=0 an invalid configuration for this specific diminishing returns logic.
        msg!("Error: diminishing_k_value in state is zero, which is invalid for this logic.");
        return err!(ErrorCode::InvalidKValue);
    }

    // 1. Burn permanent ALIGN from user's ATA (controlled by payer)
    let cpi_accounts_burn = Burn {
        mint: ctx.accounts.align_mint.to_account_info(),
        from: ctx.accounts.user_align_ata.to_account_info(),
        authority: ctx.accounts.payer.to_account_info(), // Payer is the authority for user's ALIGN ATA
    };
    let cpi_program_burn = ctx.accounts.token_program.to_account_info();
    token::burn(CpiContext::new(cpi_program_burn, cpi_accounts_burn), amount)?;
    msg!(
        "Burned {} ALIGN from ATA {} for user {}",
        amount,
        ctx.accounts.user_align_ata.key(),
        ctx.accounts.user.key()
    );

    // 2. Calculate REP to mint using diminishing returns formula
    let align_staked_float = amount as f64;
    // The supply of rep_mint *before* this transaction's mint operation
    let current_permanent_rep_supply_float = ctx.accounts.rep_mint.supply as f64;

    let k_float =
        ctx.accounts.state.diminishing_k_value as f64 / crate::data::K_SCALING_FACTOR as f64;

    // Calculate sqrt. If current_permanent_rep_supply_float is negative (not possible for u64), sqrt would panic.
    // It's safe here as supply is u64.
    let sqrt_permanent_rep = current_permanent_rep_supply_float.sqrt();

    let denominator = 1.0 + k_float * sqrt_permanent_rep;

    if denominator <= 0.0 {
        // Denominator should be > 0 if k_float >= 0 and supply >=0
        msg!(
            "Error: Denominator is zero or negative ({}). k_float: {}, sqrt_permanent_rep: {}",
            denominator,
            k_float,
            sqrt_permanent_rep
        );
        return err!(ErrorCode::Overflow); // Or a more specific error for invalid calculation
    }

    let rep_to_mint_float = align_staked_float / denominator;
    let rep_to_mint = rep_to_mint_float as u64; // Standard u64 conversion, potential precision loss is accepted

    msg!(
        "Calculation details: REP to mint: {}. Staked ALIGN: {}, Current REP supply: {}, k_scaled (state): {}, k_float: {:.6}, sqrt_rep: {:.6}, Denominator: {:.6}",
        rep_to_mint,
        amount,
        ctx.accounts.rep_mint.supply,
        ctx.accounts.state.diminishing_k_value,
        k_float,
        sqrt_permanent_rep,
        denominator
    );

    // 3. Mint permanent REP to user's ATA (controlled by payer, mint authority is state PDA)
    if rep_to_mint > 0 {
        let state_bump = ctx.accounts.state.bump;
        // Common seeds for state PDA: b"state"
        let state_seeds = &[&b"state"[..], &[state_bump]];
        let signer_seeds = &[&state_seeds[..]];

        let cpi_accounts_mint = MintTo {
            mint: ctx.accounts.rep_mint.to_account_info(),
            to: ctx.accounts.user_rep_ata.to_account_info(),
            authority: ctx.accounts.state.to_account_info(), // State PDA is the mint authority for permanent REP
        };
        let cpi_program_mint = ctx.accounts.token_program.to_account_info();
        token::mint_to(
            CpiContext::new_with_signer(cpi_program_mint, cpi_accounts_mint, signer_seeds),
            rep_to_mint,
        )?;
        msg!(
            "Minted {} REP to ATA {} for user {}",
            rep_to_mint,
            ctx.accounts.user_rep_ata.key(),
            ctx.accounts.user.key()
        );
    } else {
        msg!(
            "No REP minted for user {} as calculated amount is 0 (amount staked: {}).",
            ctx.accounts.user.key(),
            amount
        );
    }

    Ok(())
}
