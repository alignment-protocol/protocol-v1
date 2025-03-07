mod cli;
mod client;
mod commands;
mod utils;

use anyhow::Result;
use clap::Parser;
use cli::{
    Cli, Commands, DebugCommands, InitCommands, QueryCommands, SubmissionCommands, TokenCommands,
    TopicCommands, UserCommands, VoteCommands,
};
use commands::*;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup client using our client module
    let program = client::setup_client(&cli.keypair, &cli.cluster, &cli.program_id)?;

    // Handle commands
    match cli.command {
        Commands::Init { subcommand } => match subcommand {
            InitCommands::State => cmd_init_state(&program)?,
            InitCommands::TempAlignMint => cmd_init_temp_align_mint(&program)?,
            InitCommands::AlignMint => cmd_init_align_mint(&program)?,
            InitCommands::TempRepMint => cmd_init_temp_rep_mint(&program)?,
            InitCommands::RepMint => cmd_init_rep_mint(&program)?,
            InitCommands::UpdateTokensToMint { tokens } => {
                cmd_update_tokens_to_mint(&program, tokens)?
            }
        },
        Commands::Topic { subcommand } => match subcommand {
            TopicCommands::Create {
                name,
                description,
                commit_duration,
                reveal_duration,
            } => cmd_create_topic(
                &program,
                name,
                description,
                commit_duration,
                reveal_duration,
            )?,
            TopicCommands::List => cmd_list_topics(&program)?,
            TopicCommands::View { id } => cmd_view_topic(&program, id)?,
        },
        Commands::User { subcommand } => match subcommand {
            UserCommands::CreateProfile => cmd_create_user_profile(&program)?,
            UserCommands::CreateAta { token_type } => cmd_create_user_ata(&program, token_type)?,
            UserCommands::CreateTempAccount { token_type } => {
                cmd_create_user_temp_account(&program, token_type)?
            }
            UserCommands::Profile { user } => cmd_view_user_profile(&program, user)?,
        },
        Commands::Submission { subcommand } => match subcommand {
            SubmissionCommands::Submit {
                topic_id,
                data_reference,
            } => cmd_submit_data_to_topic(&program, topic_id, data_reference)?,
            SubmissionCommands::Link {
                submission_id,
                topic_id,
            } => cmd_link_submission_to_topic(&program, submission_id, topic_id)?,
            SubmissionCommands::Finalize {
                submission_id,
                topic_id,
            } => cmd_finalize_submission(&program, submission_id, topic_id)?,
        },
        Commands::Vote { subcommand } => match subcommand {
            VoteCommands::Commit {
                submission_id,
                topic_id,
                choice,
                amount,
                nonce,
                permanent,
            } => cmd_commit_vote(
                &program,
                submission_id,
                topic_id,
                choice,
                amount,
                nonce,
                permanent,
            )?,
            VoteCommands::Reveal {
                submission_id,
                topic_id,
                choice,
                nonce,
            } => cmd_reveal_vote(&program, submission_id, topic_id, choice, nonce)?,
            VoteCommands::Finalize {
                submission_id,
                topic_id,
            } => cmd_finalize_vote(&program, submission_id, topic_id)?,
            VoteCommands::SetPhases {
                submission_id,
                topic_id,
                commit_start,
                commit_end,
                reveal_start,
                reveal_end,
            } => cmd_set_voting_phases(
                &program,
                submission_id,
                topic_id,
                commit_start,
                commit_end,
                reveal_start,
                reveal_end,
            )?,
        },
        Commands::Token { subcommand } => match subcommand {
            TokenCommands::Stake { topic_id, amount } => {
                cmd_stake_topic_specific_tokens(&program, topic_id, amount)?
            }
        },
        Commands::Query { subcommand } => match subcommand {
            QueryCommands::State => cmd_query_state(&program)?,
            QueryCommands::Submission { id } => cmd_query_submission(&program, id)?,
            QueryCommands::Submissions { by, topic } => cmd_query_submissions(&program, by, topic)?,
            QueryCommands::SubmissionTopic {
                submission_id,
                topic_id,
            } => cmd_query_submission_topic(&program, submission_id, topic_id)?,
            QueryCommands::Vote {
                submission_id,
                topic_id,
                validator,
            } => cmd_query_vote(&program, submission_id, topic_id, validator)?,
        },
        Commands::Debug { subcommand } => match subcommand {
            DebugCommands::TokenAccount { token_type, user } => {
                cmd_debug_token_account(&program, token_type, user)?
            }
            DebugCommands::Tx { signature } => cmd_get_tx_logs(&program, signature)?,
        },
    }

    Ok(())
}
