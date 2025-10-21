//! Social graph analysis command handler

use std::sync::Arc;

use crate::cli::output::{print_info, print_warning};
use crate::database::Database;
use crate::social_graph::SocialGraphAnalyzer;
use crate::sync::client::SnapchainClient;
use crate::sync::lazy_loader::LazyLoader;
use crate::AppConfig;
use crate::Result;

/// Handle social graph analysis command
pub async fn handle_social_analysis(
    config: &AppConfig,
    user_identifier: String,
    verbose: bool,
) -> Result<()> {
    // Initialize services
    let database = Arc::new(Database::from_config(config).await?);
    let snapchain_client = Arc::new(SnapchainClient::from_config(config).await?);
    let lazy_loader = LazyLoader::new(database.clone(), snapchain_client.clone());

    // Parse user identifier
    let fid = parse_user_identifier(&user_identifier, &database).await?;

    // Get user profile
    let profile = lazy_loader
        .get_user_profile_smart(fid as i64)
        .await?
        .ok_or_else(|| crate::SnapRagError::Custom(format!("User {fid} not found")))?;

    let username = profile
        .username
        .as_ref().map_or_else(|| format!("FID {fid}"), |u| format!("@{u}"));
    let display_name = profile.display_name.as_deref().unwrap_or("Unknown");

    print_info(&format!(
        "📊 Analyzing social graph for {display_name} ({username})..."
    ));
    println!();

    // Analyze social profile with lazy loading capability
    let analyzer = SocialGraphAnalyzer::with_snapchain(database.clone(), snapchain_client.clone());
    let social_profile = analyzer.analyze_user(fid as i64).await?;

    // Display results
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  SOCIAL NETWORK ANALYSIS                                      ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    // Check if links data is available
    let has_links_data = social_profile.following_count > 0 || social_profile.followers_count > 0;

    if !has_links_data {
        print_warning("⚠️  Follow/follower data not available (links table is empty)");
        println!("   This happens when link messages haven't been synced yet.");
        println!("   Analysis will be based on mentions from casts instead.\n");
    }

    // Basic stats
    println!(
        "  Following:        {} {}",
        social_profile.following_count,
        if has_links_data { "" } else { "(not synced)" }
    );
    println!(
        "  Followers:        {} {}",
        social_profile.followers_count,
        if has_links_data { "" } else { "(not synced)" }
    );

    if has_links_data {
        println!("  Influence Score:  {:.2}x", social_profile.influence_score);
    }
    println!();

    // Social circles
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  NETWORK COMPOSITION                                          ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");

    if !has_links_data && !social_profile.most_mentioned_users.is_empty() {
        println!("  (Based on mentioned users in casts)\n");
    } else {
        println!();
    }

    let has_circle_data = social_profile.social_circles.tech_builders > 0.0
        || social_profile.social_circles.web3_natives > 0.0
        || social_profile.social_circles.content_creators > 0.0
        || social_profile.social_circles.casual_users > 0.0;

    if has_circle_data {
        print_percentage(
            "🔧 Tech/Builders",
            social_profile.social_circles.tech_builders,
        );
        print_percentage("⛓️ Web3/Crypto", social_profile.social_circles.web3_natives);
        print_percentage(
            "🎨 Content Creators",
            social_profile.social_circles.content_creators,
        );
        print_percentage(
            "💬 Casual Users",
            social_profile.social_circles.casual_users,
        );
    } else {
        println!("  No network composition data available yet.");
    }
    println!();

    // Most mentioned users
    if !social_profile.most_mentioned_users.is_empty() {
        println!("╔═══════════════════════════════════════════════════════════════╗");
        println!("║  MOST MENTIONED USERS                                         ║");
        println!("╚═══════════════════════════════════════════════════════════════╝");
        println!();

        for (idx, user) in social_profile.most_mentioned_users.iter().enumerate() {
            let name = user
                .username
                .as_ref()
                .map(|u| format!("@{u}"))
                .or_else(|| user.display_name.clone())
                .unwrap_or_else(|| format!("FID {}", user.fid));

            println!(
                "  {}. {} - mentioned {}x ({})",
                idx + 1,
                name,
                user.count,
                user.category
            );
        }
        println!();
    }

    // Interaction style
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  INTERACTION STYLE                                            ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    println!(
        "  Community Role:     {}",
        social_profile.interaction_style.community_role
    );
    println!(
        "  Reply Frequency:    {:.1}%",
        social_profile.interaction_style.reply_frequency * 100.0
    );
    println!(
        "  Mention Frequency:  {:.1}%",
        social_profile.interaction_style.mention_frequency * 100.0
    );

    if social_profile.interaction_style.network_connector {
        println!("  🌐 Network Connector - actively introduces people");
    }
    println!();

    // Word cloud
    if !social_profile.word_cloud.top_words.is_empty() {
        println!("╔═══════════════════════════════════════════════════════════════╗");
        println!("║  VOCABULARY & WORD CLOUD                                      ║");
        println!("╚═══════════════════════════════════════════════════════════════╝");
        println!();

        println!("  Top Words:");
        for (idx, word_freq) in social_profile
            .word_cloud
            .top_words
            .iter()
            .take(15)
            .enumerate()
        {
            let bar_length = (word_freq.percentage * 0.5) as usize; // Scale for display
            let bar = "█".repeat(bar_length.max(1));
            println!(
                "    {:2}. {:<15} {:>4}x {:>5.1}% {}",
                idx + 1,
                word_freq.word,
                word_freq.count,
                word_freq.percentage,
                bar
            );
        }
        println!();

        // Show common phrases
        if !social_profile.word_cloud.top_phrases.is_empty() {
            println!("  Common Phrases:");
            for (idx, phrase) in social_profile
                .word_cloud
                .top_phrases
                .iter()
                .take(8)
                .enumerate()
            {
                println!("    {}. \"{}\" ({}x)", idx + 1, phrase.word, phrase.count);
            }
            println!();
        }

        // Show signature words
        if !social_profile.word_cloud.signature_words.is_empty() {
            println!(
                "  Signature Words: {}",
                social_profile.word_cloud.signature_words.join(", ")
            );
            println!();
        }
    }

    // Analysis summary
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  PROFILE SUMMARY                                              ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    let mut summary = Vec::new();

    if social_profile.influence_score > 2.0 {
        summary.push("🌟 Influential user with strong reach");
    } else if social_profile.influence_score > 1.0 {
        summary.push("📈 Growing influence in the network");
    } else {
        summary.push("🌱 Building network and connections");
    }

    if social_profile.social_circles.tech_builders > 40.0 {
        summary.push("💻 Deeply embedded in tech/builder circles");
    }

    if social_profile.social_circles.web3_natives > 40.0 {
        summary.push("⛓️ Strong web3/crypto network");
    }

    if social_profile.interaction_style.community_role == "leader" {
        summary.push("👑 Community leader - active and engaged");
    } else if social_profile.interaction_style.community_role == "contributor" {
        summary.push("🤝 Active contributor to discussions");
    }

    if social_profile.interaction_style.network_connector {
        summary.push("🌐 Network connector - bridges communities");
    }

    for item in summary {
        println!("  • {item}");
    }
    println!();

    // Verbose mode: show detailed data
    if verbose {
        println!("╔═══════════════════════════════════════════════════════════════╗");
        println!("║  DETAILED DATA                                                ║");
        println!("╚═══════════════════════════════════════════════════════════════╝");
        println!();
        println!("{social_profile:#?}");
    }

    Ok(())
}

/// Parse user identifier (FID or username)
async fn parse_user_identifier(identifier: &str, database: &Database) -> Result<u64> {
    let trimmed = identifier.trim();

    if trimmed.starts_with('@') {
        let username = trimmed.trim_start_matches('@');
        let profile = database
            .get_user_profile_by_username(username)
            .await?
            .ok_or_else(|| {
                crate::SnapRagError::Custom(format!("Username @{username} not found"))
            })?;
        Ok(profile.fid as u64)
    } else {
        trimmed.parse::<u64>().map_err(|_| {
            crate::SnapRagError::Custom(format!(
                "Invalid user identifier '{identifier}'. Use FID or @username"
            ))
        })
    }
}

/// Helper to print percentage with visual bar
fn print_percentage(label: &str, value: f32) {
    let bar_length = (value / 5.0) as usize; // 20% = 4 chars
    let bar = "█".repeat(bar_length);
    println!("  {label:<20} {value:>5.1}% {bar}");
}
