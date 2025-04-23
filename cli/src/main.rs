mod cli;
mod client;
mod commands;

use anyhow::Result;
use clap::Parser;
use cli::{
    Cli, Commands, ConfigCommands, DebugCommands, InitCommands, QueryCommands, SubmissionCommands,
    TokenCommands, TopicCommands, UserCommands, VoteCommands,
};
use commands::{admin, user};

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Determine which cluster to use
    let cluster = match &cli.cluster {
        // If --cluster parameter is provided, use it (overrides saved config)
        Some(specified_cluster) => specified_cluster.clone(),

        // Otherwise, try to use the saved configuration
        None => {
            // Read saved cluster configuration

            commands::admin::config::read_cluster_config()?
        }
    };

    // Setup client using our client module
    let program = client::setup_client(&cli.keypair, &cluster, &cli.program_id)?;

    // Handle commands
    match cli.command {
        Commands::Topic { subcommand } => match subcommand {
            TopicCommands::List => user::topic::cmd_list_topics(&program)?,
            TopicCommands::View { id } => user::topic::cmd_view_topic(&program, id)?,
            TopicCommands::Create {
                name,
                description,
                commit_duration,
                reveal_duration,
            } => {
                println!("Creating new topic...");
                user::topic::cmd_create_topic(
                    &program,
                    name,
                    description,
                    commit_duration,
                    reveal_duration,
                )?
            }
            TopicCommands::Update {
                id,
                commit_duration,
                reveal_duration,
                active,
            } => user::topic::cmd_update_topic(
                &program,
                id,
                commit_duration,
                reveal_duration,
                active,
            )?,
        },
        Commands::User { subcommand } => match subcommand {
            UserCommands::CreateProfile => user::user::cmd_create_user_profile(&program)?,
            UserCommands::Profile { user } => user::user::cmd_view_user_profile(&program, user)?,
            UserCommands::InitializeTopicBalance { topic_id } => {
                user::topic::cmd_initialize_user_topic_balance(&program, topic_id)?
            }
        },
        Commands::Submission { subcommand } => match subcommand {
            SubmissionCommands::Submit {
                topic_id,
                data_reference,
            } => user::submission::cmd_submit_data_to_topic(&program, topic_id, data_reference)?,
            SubmissionCommands::Link {
                submission_pda,
                topic_id,
            } => {
                user::submission::cmd_link_submission_to_topic(&program, submission_pda, topic_id)?
            }
            SubmissionCommands::Finalize {
                submission_pda,
                topic_id,
            } => user::submission::cmd_finalize_submission(&program, submission_pda, topic_id)?,
            SubmissionCommands::RequestAiValidation {
                submission_pda,
                topic_id,
                amount,
            } => user::submission::cmd_request_ai_validation(
                &program,
                submission_pda,
                topic_id,
                amount,
            )?,
        },
        Commands::Vote { subcommand } => match subcommand {
            VoteCommands::Commit {
                submission_pda,
                topic_id,
                choice,
                amount,
                nonce,
                permanent,
            } => user::vote::cmd_commit_vote(
                &program,
                submission_pda,
                topic_id,
                choice,
                amount,
                nonce,
                permanent,
            )?,
            VoteCommands::Reveal {
                submission_pda,
                topic_id,
                choice,
                nonce,
            } => user::vote::cmd_reveal_vote(&program, submission_pda, topic_id, choice, nonce)?,
            VoteCommands::Finalize {
                submission_pda,
                topic_id,
            } => user::vote::cmd_finalize_vote(&program, submission_pda, topic_id)?,
            VoteCommands::SetPhases {
                submission_pda,
                topic_id,
                commit_start,
                commit_end,
                reveal_start,
                reveal_end,
            } => {
                println!("[ADMIN] Setting voting phases...");
                admin::vote::cmd_set_voting_phases(
                    &program,
                    submission_pda,
                    topic_id,
                    commit_start,
                    commit_end,
                    reveal_start,
                    reveal_end,
                )?
            }
        },
        Commands::Token { subcommand } => match subcommand {
            TokenCommands::Stake { topic_id, amount } => {
                user::token::cmd_stake_topic_specific_tokens(&program, topic_id, amount)?
            }
            TokenCommands::Mint {
                token_type,
                to,
                amount,
            } => {
                println!("[ADMIN] Minting tokens...");
                admin::token::cmd_admin_mint_tokens(&program, &token_type, &to, amount)?
            }
        },
        Commands::Query { subcommand } => match subcommand {
            QueryCommands::State => user::query::cmd_query_state(&program)?,
            QueryCommands::Submission { pda } => user::query::cmd_query_submission(&program, pda)?,
            QueryCommands::Submissions { by, topic } => {
                user::query::cmd_query_submissions(&program, by, topic)?
            }
            QueryCommands::SubmissionTopic {
                submission_pda,
                topic_id,
            } => user::query::cmd_query_submission_topic(&program, submission_pda, topic_id)?,
            QueryCommands::Vote {
                submission_pda,
                topic_id,
                validator,
            } => user::query::cmd_query_vote(&program, submission_pda, topic_id, validator)?,
            QueryCommands::TopicBalance { topic_id, user } => {
                user::query::cmd_view_user_topic_balance(&program, topic_id, user)?
            }
        },
        Commands::Debug { subcommand } => match subcommand {
            DebugCommands::TokenAccount { token_type, user } => {
                user::debug::cmd_debug_token_account(&program, token_type, user)?
            }
            DebugCommands::Tx { signature } => user::debug::cmd_get_tx_logs(&program, signature)?,
        },
        Commands::Init { subcommand } => {
            println!("[ADMIN] Running initialization...");
            match subcommand {
                InitCommands::State { oracle_pubkey } => {
                    admin::init::cmd_init_state(&program, oracle_pubkey)?
                }
                InitCommands::TempAlignMint => admin::init::cmd_init_temp_align_mint(&program)?,
                InitCommands::AlignMint => admin::init::cmd_init_align_mint(&program)?,
                InitCommands::TempRepMint => admin::init::cmd_init_temp_rep_mint(&program)?,
                InitCommands::RepMint => admin::init::cmd_init_rep_mint(&program)?,
                InitCommands::All { oracle_pubkey } => {
                    admin::init::cmd_init_all(&program, oracle_pubkey)?
                }
            }
        }
        Commands::Config { subcommand } => match subcommand {
            ConfigCommands::UpdateTokensToMint { tokens } => {
                println!("[ADMIN] Updating token configuration...");
                admin::config::cmd_admin_update_tokens_to_mint(&program, tokens)?
            }
            ConfigCommands::SetCluster { cluster } => {
                println!("[ADMIN] Setting cluster configuration...");
                admin::config::cmd_admin_set_cluster(cluster)?
            }
            ConfigCommands::GetCluster => {
                println!("[ADMIN] Retrieving cluster configuration...");
                admin::config::cmd_admin_get_cluster()?
            }
        },
    }

    Ok(())
}
