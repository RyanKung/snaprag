use clap::Parser;
use snaprag::cli::CastCommands;
use snaprag::cli::CastEmbeddingAction;
use snaprag::cli::Cli;
use snaprag::cli::Commands;
use snaprag::cli::EmbeddingDataType;
use snaprag::cli::EmbeddingsCommands;
use snaprag::cli::FastsyncCommands;
use snaprag::cli::FetchCommands;
use snaprag::cli::RagCommands;
use snaprag::cli::ServeCommands;
use snaprag::cli::SyncCommands;
use snaprag::AppConfig;
use snaprag::Result;
use snaprag::SnapRag;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        snaprag::logging::init_logging_with_level("debug")?;
    } else {
        snaprag::logging::init_logging()?;
    }

    // Load configuration
    let config = AppConfig::load()?;
    info!("Configuration loaded successfully");

    // Create SnapRAG instance
    let snaprag = SnapRag::new(&config).await?;

    // 🚀 OPTIMIZATION: Only initialize database schema for commands that need it
    // Skip for read-only or sync management commands
    let needs_schema_init = matches!(
        cli.command,
        Commands::Init { .. } | Commands::Reset { .. } | Commands::Embeddings(..)
    );

    if needs_schema_init {
        snaprag.init_database().await?;
        info!("Database schema initialized");
    } else {
        tracing::debug!("Skipping schema initialization for this command");
    }

    // Execute the requested command
    match cli.command {
        Commands::Init {
            force,
            skip_indexes,
        } => {
            snaprag::cli::handle_init_command(&snaprag, force, skip_indexes).await?;
        }
        Commands::List {
            data_type,
            limit,
            search,
            sort_by,
            sort_order,
            fid_range,
            username,
            display_name,
            bio,
            location,
            twitter,
            github,
            has_username,
            has_display_name,
            has_bio,
        } => {
            snaprag::cli::handle_list_command(
                &snaprag,
                data_type,
                limit,
                search,
                sort_by,
                sort_order,
                fid_range,
                username,
                display_name,
                bio,
                location,
                twitter,
                github,
                has_username,
                has_display_name,
                has_bio,
            )
            .await?;
        }
        Commands::Reset { force } => {
            snaprag::cli::handle_reset_command(&snaprag, force).await?;
        }
        Commands::Index(index_command) => {
            snaprag::cli::handle_index_command(&snaprag, &index_command).await?;
        }
        Commands::Fastsync(fastsync_command) => {
            snaprag::cli::handle_fastsync_command(&snaprag, &fastsync_command).await?;
        }
        Commands::Sync(sync_command) => {
            snaprag::cli::handle_sync_command(snaprag, sync_command).await?;
        }
        Commands::Stats { detailed, export } => {
            snaprag::cli::handle_stats_command(&snaprag, detailed, export).await?;
        }
        Commands::Search {
            query,
            limit,
            fields,
        } => {
            snaprag::cli::handle_search_command(&snaprag, query, limit, fields).await?;
        }
        Commands::Config => {
            snaprag::cli::handle_config_command(&config)?;
        }
        Commands::Activity {
            fid,
            limit,
            offset,
            activity_type,
            detailed,
        } => {
            snaprag::cli::handle_activity_command(
                &snaprag,
                fid,
                limit,
                offset,
                activity_type,
                detailed,
            )
            .await?;
        }
        Commands::Cast(cast_command) => match cast_command {
            CastCommands::Search {
                query,
                limit,
                threshold,
                detailed,
            } => {
                snaprag::cli::handle_cast_search(&snaprag, query, limit, threshold, detailed)
                    .await?;
            }
            CastCommands::Recent { fid, limit } => {
                snaprag::cli::handle_cast_recent(&snaprag, fid, limit).await?;
            }
            CastCommands::Thread { hash, depth } => {
                snaprag::cli::handle_cast_thread(&snaprag, hash, depth).await?;
            }
        },
        Commands::Rag(rag_command) => match rag_command {
            RagCommands::Query {
                query,
                limit,
                method,
                temperature,
                max_tokens,
                verbose,
            } => {
                snaprag::cli::handle_rag_query(
                    &config,
                    query,
                    limit,
                    method,
                    temperature,
                    max_tokens,
                    verbose,
                )
                .await?;
            }
            RagCommands::QueryCasts {
                query,
                limit,
                threshold,
                temperature,
                max_tokens,
                verbose,
            } => {
                snaprag::cli::handle_rag_query_casts(
                    &snaprag,
                    query,
                    limit,
                    threshold,
                    temperature,
                    max_tokens,
                    verbose,
                )
                .await?;
            }
            RagCommands::Search {
                query,
                limit,
                method,
            } => {
                snaprag::cli::handle_rag_search(&config, query, limit, method).await?;
            }
        },
        Commands::Embeddings(embeddings_command) => match embeddings_command {
            EmbeddingsCommands::Backfill {
                data_type,
                force,
                batch_size,
                limit,
                endpoint,
                #[cfg(feature = "local-gpu")]
                local_gpu,
                #[cfg(feature = "local-gpu")]
                multiprocess,
                #[cfg(feature = "local-gpu")]
                gpu_device,
            } => {
                snaprag::cli::handle_embeddings_backfill(
                    &config,
                    data_type,
                    force,
                    batch_size,
                    limit,
                    endpoint,
                    #[cfg(feature = "local-gpu")]
                    local_gpu,
                    #[cfg(feature = "local-gpu")]
                    multiprocess,
                    #[cfg(feature = "local-gpu")]
                    gpu_device,
                )
                .await?;
            }
            EmbeddingsCommands::Generate { fid, verbose } => {
                snaprag::cli::handle_embeddings_generate(&config, fid, verbose).await?;
            }
            EmbeddingsCommands::Test { text } => {
                snaprag::cli::handle_embeddings_test(&config, text).await?;
            }
            EmbeddingsCommands::TestCast {
                message_hash,
                endpoint,
                #[cfg(feature = "local-gpu")]
                local_gpu,
                #[cfg(feature = "local-gpu")]
                gpu_device,
            } => {
                snaprag::cli::handle_embeddings_test_cast(
                    &config,
                    message_hash,
                    endpoint,
                    #[cfg(feature = "local-gpu")]
                    local_gpu,
                    #[cfg(feature = "local-gpu")]
                    gpu_device,
                )
                .await?;
            }
            EmbeddingsCommands::Stats => {
                snaprag::cli::handle_embeddings_stats(&config).await?;
            }
            EmbeddingsCommands::Reset { force } => {
                snaprag::cli::handle_embeddings_reset(&config, force).await?;
            }
            EmbeddingsCommands::Cast { action } => match action {
                snaprag::cli::CastEmbeddingAction::Backfill {
                    force,
                    batch_size,
                    limit,
                    endpoint,
                    #[cfg(feature = "local-gpu")]
                    local_gpu,
                    #[cfg(feature = "local-gpu")]
                    multiprocess,
                    #[cfg(feature = "local-gpu")]
                    gpu_device,
                } => {
                    snaprag::cli::handle_cast_embeddings_backfill(
                        &config,
                        limit,
                        endpoint,
                        #[cfg(feature = "local-gpu")]
                        local_gpu,
                        #[cfg(feature = "local-gpu")]
                        multiprocess,
                        #[cfg(feature = "local-gpu")]
                        gpu_device,
                    )
                    .await?;
                }
                snaprag::cli::CastEmbeddingAction::Reset { force } => {
                    snaprag::cli::handle_cast_embeddings_reset(&config, force).await?;
                }
                snaprag::cli::CastEmbeddingAction::BackfillMultiVector {
                    force,
                    limit,
                    endpoint,
                    #[cfg(feature = "local-gpu")]
                    local_gpu,
                    #[cfg(feature = "local-gpu")]
                    gpu_device,
                    enable_multi_vector,
                    strategy,
                    aggregation,
                    min_length,
                } => {
                    snaprag::cli::handle_cast_embeddings_backfill_multivector(
                        &config,
                        force,
                        limit,
                        endpoint,
                        #[cfg(feature = "local-gpu")]
                        local_gpu,
                        #[cfg(feature = "local-gpu")]
                        gpu_device,
                        enable_multi_vector,
                        &strategy,
                        &aggregation,
                        min_length,
                    )
                    .await?;
                }
                snaprag::cli::CastEmbeddingAction::Migrate {
                    force,
                    min_length,
                    strategy,
                    keep_original,
                    batch_size,
                } => {
                    snaprag::cli::handle_cast_embeddings_migrate(
                        &config,
                        force,
                        min_length,
                        &strategy,
                        keep_original,
                        batch_size,
                    )
                    .await?;
                }
                snaprag::cli::CastEmbeddingAction::Analyze => {
                    snaprag::cli::handle_cast_embeddings_analyze(&config).await?;
                }
            }
            EmbeddingsCommands::BackfillCasts {
                force,
                batch_size,
                limit,
                endpoint,
                #[cfg(feature = "local-gpu")]
                local_gpu,
                #[cfg(feature = "local-gpu")]
                multiprocess,
                #[cfg(feature = "local-gpu")]
                gpu_device,
            } => {
                snaprag::cli::handle_cast_embeddings_backfill(
                    &config,
                    limit,
                    endpoint,
                    #[cfg(feature = "local-gpu")]
                    local_gpu,
                    #[cfg(feature = "local-gpu")]
                    multiprocess,
                    #[cfg(feature = "local-gpu")]
                    gpu_device,
                )
                .await?;
            }
        },
        Commands::Ask {
            user,
            question,
            chat,
            fetch_casts,
            context_limit,
            temperature,
            verbose,
        } => {
            snaprag::cli::handle_ask(
                &config,
                user,
                question,
                chat,
                fetch_casts,
                context_limit,
                temperature,
                verbose,
            )
            .await?;
        }
        Commands::Social { user, verbose } => {
            snaprag::cli::handle_social_analysis(&config, user, verbose).await?;
        }
        Commands::Fetch(fetch_command) => match fetch_command {
            FetchCommands::User {
                fid,
                with_casts,
                max_casts,
                generate_embeddings,
                embedding_endpoint,
            } => {
                snaprag::cli::handle_fetch_user(
                    &config,
                    fid,
                    with_casts,
                    max_casts,
                    generate_embeddings,
                    embedding_endpoint,
                )
                .await?;
            }
            FetchCommands::Users {
                fids,
                with_casts,
                generate_embeddings,
                embedding_endpoint,
            } => {
                snaprag::cli::handle_fetch_users(
                    &config,
                    fids,
                    with_casts,
                    generate_embeddings,
                    embedding_endpoint,
                )
                .await?;
            }
            FetchCommands::Popular {
                limit,
                with_casts,
                generate_embeddings,
                embedding_endpoint,
            } => {
                snaprag::cli::handle_fetch_popular(
                    &config,
                    limit,
                    with_casts,
                    generate_embeddings,
                    embedding_endpoint,
                )
                .await?;
            }
        },
        Commands::Serve(serve_command) => match serve_command {
            ServeCommands::Api {
                host,
                port,
                cors,
                #[cfg(feature = "payment")]
                payment,
                #[cfg(feature = "payment")]
                payment_address,
                #[cfg(feature = "payment")]
                testnet,
            } => {
                snaprag::cli::handle_serve_api(
                    &config,
                    host,
                    port,
                    cors,
                    #[cfg(feature = "payment")]
                    payment,
                    #[cfg(feature = "payment")]
                    payment_address,
                    #[cfg(feature = "payment")]
                    testnet,
                )
                .await?;
            }
        },
    }

    Ok(())
}
