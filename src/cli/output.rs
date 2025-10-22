//! CLI output formatting utilities
//!
//! This module provides consistent output formatting for the `SnapRAG` CLI

use crate::models::{UserProfile, Cast, Link, UserData, StatisticsResult};
use crate::AppConfig;

/// Safely truncate a string at character boundary (not byte boundary)
/// 
/// This prevents panics when truncating strings with multi-byte UTF-8 characters (emojis, etc.)
/// 
/// # Arguments
/// * `s` - The string to truncate
/// * `max_chars` - Maximum number of characters (not bytes)
/// 
/// # Returns
/// Truncated string with "..." suffix if truncated, otherwise the original string
#[must_use]
pub fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() > max_chars {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{truncated}...")
    } else {
        s.to_string()
    }
}

/// Print a list header
pub fn print_list_header(data_type: &str, limit: u32) {
    println!("📋 Listing {data_type} (limit: {limit})");
}

/// Print FID list
pub fn print_fid_list(profiles: &[UserProfile]) {
    println!("Found {} FIDs:", profiles.len());
    for profile in profiles {
        println!(
            "  - FID: {} | Username: {} | Display: {}",
            profile.fid,
            profile.username.as_deref().unwrap_or("N/A"),
            profile.display_name.as_deref().unwrap_or("N/A")
        );
    }
}

/// Print profile list
pub fn print_profile_list(profiles: &[UserProfile]) {
    println!("Found {} profiles:", profiles.len());
    for profile in profiles {
        println!(
            "  - FID: {}, Username: {:?}, Display: {:?}, Bio: {:?}",
            profile.fid, profile.username, profile.display_name, profile.bio
        );
    }
}

/// Print cast list
pub fn print_cast_list(casts: &[Cast]) {
    println!("Found {} casts:", casts.len());
    for cast in casts {
        let text_preview = cast
            .text
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(100)
            .collect::<String>();
        let text_display = if text_preview.len() >= 100 {
            format!("{text_preview}...")
        } else {
            text_preview
        };
        println!(
            "  - FID: {} | Text: {} | Timestamp: {}",
            cast.fid, text_display, cast.timestamp
        );
    }
}

/// Print link list
pub fn print_link_list(links: &[Link]) {
    println!("Found {} follow relationships:", links.len());
    for link in links {
        println!(
            "  - FID: {} -> Target: {} | Type: {} | Timestamp: {}",
            link.fid, link.target_fid, link.link_type, link.timestamp
        );
    }
}

/// Print user data list
pub fn print_user_data_list(user_data: &[UserData]) {
    println!("Found {} user data records:", user_data.len());
    for data in user_data {
        println!(
            "  - FID: {} | Type: {} | Value: {} | Timestamp: {}",
            data.fid, data.data_type, data.value, data.timestamp
        );
    }
}

/// Print search header
pub fn print_search_header(query: &str, fields: &str) {
    println!("🔍 Searching profiles for: \"{query}\"");
    println!("Fields: {fields}");
    println!();
}

/// Print search results
pub fn print_search_results(profiles: &[UserProfile], limit: usize) {
    println!("Found {} profiles:", profiles.len().min(limit));

    for profile in profiles.iter().take(limit) {
        println!();
        println!("  🆔 FID: {}", profile.fid);
        if let Some(username) = &profile.username {
            println!("  👤 Username: {username}");
        }
        if let Some(display_name) = &profile.display_name {
            println!("  📝 Display Name: {display_name}");
        }
        if let Some(bio) = &profile.bio {
            println!("  📄 Bio: {bio}");
        }
        if let Some(location) = &profile.location {
            println!("  📍 Location: {location}");
        }
        if let Some(twitter) = &profile.twitter_username {
            println!("  🐦 Twitter: @{twitter}");
        }
        if let Some(github) = &profile.github_username {
            println!("  🐙 GitHub: @{github}");
        }
        println!(
            "  🕒 Last Updated: {}",
            profile.last_updated_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
    }
}

/// Print statistics
pub fn print_statistics(stats: &StatisticsResult, detailed: bool) {
    println!("📊 SnapRAG Statistics");
    println!("===================");

    println!();
    println!("📈 Overview:");
    println!("  Total FIDs: {}", stats.total_fids);
    println!("  Total Profiles: {}", stats.total_profiles);

    println!();
    println!("👤 Profile Completeness:");
    println!(
        "  With Username: {} ({:.1}%)",
        stats.profiles_with_username,
        if stats.total_profiles > 0 {
            (stats.profiles_with_username as f64 / stats.total_profiles as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "  With Display Name: {} ({:.1}%)",
        stats.profiles_with_display_name,
        if stats.total_profiles > 0 {
            (stats.profiles_with_display_name as f64 / stats.total_profiles as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "  With Bio: {} ({:.1}%)",
        stats.profiles_with_bio,
        if stats.total_profiles > 0 {
            (stats.profiles_with_bio as f64 / stats.total_profiles as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "  With Profile Picture: {} ({:.1}%)",
        stats.profiles_with_pfp,
        if stats.total_profiles > 0 {
            (stats.profiles_with_pfp as f64 / stats.total_profiles as f64) * 100.0
        } else {
            0.0
        }
    );

    if detailed {
        println!();
        println!("🔗 Social Links:");
        println!(
            "  With Website: {} ({:.1}%)",
            stats.profiles_with_website,
            if stats.total_profiles > 0 {
                (stats.profiles_with_website as f64 / stats.total_profiles as f64) * 100.0
            } else {
                0.0
            }
        );
        println!(
            "  With Twitter: {} ({:.1}%)",
            stats.profiles_with_twitter,
            if stats.total_profiles > 0 {
                (stats.profiles_with_twitter as f64 / stats.total_profiles as f64) * 100.0
            } else {
                0.0
            }
        );
        println!(
            "  With GitHub: {} ({:.1}%)",
            stats.profiles_with_github,
            if stats.total_profiles > 0 {
                (stats.profiles_with_github as f64 / stats.total_profiles as f64) * 100.0
            } else {
                0.0
            }
        );
        println!(
            "  With Ethereum Address: {} ({:.1}%)",
            stats.profiles_with_ethereum_address,
            if stats.total_profiles > 0 {
                (stats.profiles_with_ethereum_address as f64 / stats.total_profiles as f64) * 100.0
            } else {
                0.0
            }
        );
        println!(
            "  With Solana Address: {} ({:.1}%)",
            stats.profiles_with_solana_address,
            if stats.total_profiles > 0 {
                (stats.profiles_with_solana_address as f64 / stats.total_profiles as f64) * 100.0
            } else {
                0.0
            }
        );

        println!();
        println!("🆕 Recent Registrations:");
        for reg in &stats.recent_registrations {
            println!(
                "  - FID: {} | Username: {} | Display: {} | Created: {}",
                reg.fid,
                reg.username.as_deref().unwrap_or("N/A"),
                reg.display_name.as_deref().unwrap_or("N/A"),
                reg.created_at.format("%Y-%m-%d %H:%M:%S UTC")
            );
        }
    }
}

/// Print dashboard
pub fn print_dashboard(stats: &StatisticsResult) {
    println!("📊 SnapRAG Dashboard");
    println!("===================");

    println!();
    println!("🎯 Key Metrics:");
    println!("  Total Users: {}", stats.total_fids);
    println!("  Total Activities: {}", stats.total_activities);
    println!("  Total Casts: {}", stats.total_casts);
    println!(
        "  Complete Profiles: {} ({:.1}%)",
        stats.complete_profiles,
        if stats.total_profiles > 0 {
            (stats.complete_profiles as f64 / stats.total_profiles as f64) * 100.0
        } else {
            0.0
        }
    );
    println!("    (username + display_name + bio)");

    println!();
    println!("📈 Profile Health:");
    println!("  ✅ With Username: {}", stats.profiles_with_username);
    println!(
        "  ✅ With Display Name: {}",
        stats.profiles_with_display_name
    );
    println!("  ✅ With Bio: {}", stats.profiles_with_bio);
    println!("  ✅ With Profile Picture: {}", stats.profiles_with_pfp);

    println!();
    println!("🔗 Social Presence:");
    println!("  🌐 With Website: {}", stats.profiles_with_website);
    println!("  🐦 With Twitter: {}", stats.profiles_with_twitter);
    println!("  🐙 With GitHub: {}", stats.profiles_with_github);
    println!(
        "  💰 With Ethereum: {}",
        stats.profiles_with_ethereum_address
    );
    println!("  💰 With Solana: {}", stats.profiles_with_solana_address);

    println!();
    println!("📊 Activity Breakdown:");
    for activity_stat in stats.activities_by_type.iter().take(5) {
        println!(
            "  {} {}: {}",
            match activity_stat.activity_type.as_str() {
                "cast_add" => "✍️",
                "reaction_add" => "❤️",
                "link_add" => "👥",
                "link_remove" => "👋",
                "cast_remove" => "🗑️",
                "verification_add" => "✅",
                _ => "📌",
            },
            activity_stat.activity_type,
            activity_stat.count
        );
    }

    println!();
    println!("🆕 Recent Activity:");
    for (i, reg) in stats.recent_registrations.iter().take(5).enumerate() {
        println!(
            "  {}. FID: {} | @{} | {}",
            i + 1,
            reg.fid,
            reg.username.as_deref().unwrap_or("N/A"),
            reg.display_name.as_deref().unwrap_or("N/A")
        );
    }
}

/// Print configuration
pub fn print_config(config: &AppConfig) {
    println!("📋 SnapRAG Configuration:");
    println!();

    println!("🗄️  Database:");
    println!("  URL: {}", mask_database_url(config.database_url()));
    println!("  Max connections: {}", config.max_connections());
    println!("  Min connections: {}", config.min_connections());
    println!("  Connection timeout: {}s", config.connection_timeout());
    println!();

    println!("📝 Logging:");
    println!("  Level: {}", config.logging.level);
    println!("  Backtrace: {}", config.logging.backtrace);
    println!();

    println!("🧠 Embeddings:");
    println!("  Dimension: {}", config.embedding_dimension());
    println!("  Model: {}", config.embedding_model());
    println!();

    println!("⚡ Performance:");
    println!("  Vector indexes: {}", config.vector_indexes_enabled());
    println!("  Vector index lists: {}", config.vector_index_lists());
    println!();

    println!("🔄 Sync:");
    println!("  HTTP endpoint: {}", config.snapchain_http_endpoint());
    println!("  gRPC endpoint: {}", config.snapchain_grpc_endpoint());
    println!("  Real-time sync: {}", config.realtime_sync_enabled());
    println!("  Historical sync: {}", config.historical_sync_enabled());
    println!(
        "  Historical sync from event ID: {}",
        config.historical_sync_from_event_id()
    );
    println!("  Batch size: {}", config.sync_batch_size());
    println!("  Sync interval: {}ms", config.sync_interval_ms());
    println!("  Shard IDs: {:?}", config.shard_ids());
    println!();

    println!("🤖 LLM:");
    println!("  Endpoint: {}", config.llm_endpoint());
    println!("  Key: {}", config.llm_key());
}

/// Mask database URL for logging (hide password)
fn mask_database_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            format!(
                "{}://{}@{}:{}",
                parsed.scheme(),
                parsed.username(),
                host,
                parsed.port().unwrap_or(5432)
            )
        } else {
            "***masked***".to_string()
        }
    } else {
        "***invalid***".to_string()
    }
}

/// Print colored output functions
pub fn print_info(msg: &str) {
    println!("ℹ️  {msg}");
}

pub fn print_success(msg: &str) {
    println!("✅ {msg}");
}

pub fn print_warning(msg: &str) {
    println!("⚠️  {msg}");
}

pub fn print_error(msg: &str) {
    println!("❌ {msg}");
}

pub fn print_prompt(msg: &str) {
    print!("{msg}");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
}
