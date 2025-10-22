//! Data query handlers (list, search, activity)

use crate::cli::commands::DataType;
use crate::cli::output::print_cast_list;
use crate::cli::output::print_error;
use crate::cli::output::print_fid_list;
use crate::cli::output::print_info;
use crate::cli::output::print_link_list;
use crate::cli::output::print_list_header;
use crate::cli::output::print_profile_list;
use crate::cli::output::print_user_data_list;
use crate::cli::output::print_warning;
use crate::Result;
use crate::SnapRag;

/// Handle list command
pub async fn handle_list_command(
    snaprag: &SnapRag,
    data_type: DataType,
    limit: u32,
    search: Option<String>,
    sort_by: Option<String>,
    sort_order: String,
    fid_range: Option<String>,
    username: Option<String>,
    display_name: Option<String>,
    bio: Option<String>,
    location: Option<String>,
    twitter: Option<String>,
    github: Option<String>,
    has_username: bool,
    has_display_name: bool,
    has_bio: bool,
) -> Result<()> {
    match data_type {
        DataType::Fid => {
            print_list_header("FIDs", limit);

            // Parse FID range if provided
            let (min_fid, max_fid) = if let Some(range) = fid_range {
                if let Some((min, max)) = range.split_once('-') {
                    (
                        Some(min.parse::<i64>().unwrap_or(0)),
                        Some(max.parse::<i64>().unwrap_or(i64::MAX)),
                    )
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            // Parse sort options
            let sort_by = match sort_by.as_deref() {
                Some("fid") => Some(crate::models::FidSortBy::Fid),
                Some("username") => Some(crate::models::FidSortBy::Username),
                Some("last_updated") => Some(crate::models::FidSortBy::LastUpdated),
                Some("created_at") => Some(crate::models::FidSortBy::CreatedAt),
                _ => None,
            };

            let sort_order = match sort_order.as_str() {
                "asc" => Some(crate::models::SortOrder::Asc),
                "desc" => Some(crate::models::SortOrder::Desc),
                _ => Some(crate::models::SortOrder::Asc),
            };

            // Build FID query
            let fid_query = crate::models::FidQuery {
                fid: None,
                min_fid,
                max_fid,
                has_username: if has_username { Some(true) } else { None },
                has_display_name: if has_display_name { Some(true) } else { None },
                has_bio: if has_bio { Some(true) } else { None },
                limit: Some(i64::from(limit)),
                offset: None,
                sort_by,
                sort_order,
                search_term: search,
            };

            let profiles = snaprag.database().list_fids(fid_query).await?;
            print_fid_list(&profiles);
        }
        DataType::Profiles => {
            print_list_header("User Profiles", limit);

            // Parse sort options
            let sort_by = match sort_by.as_deref() {
                Some("fid") => Some(crate::models::ProfileSortBy::Fid),
                Some("username") => Some(crate::models::ProfileSortBy::Username),
                Some("display_name") => Some(crate::models::ProfileSortBy::DisplayName),
                Some("last_updated") => Some(crate::models::ProfileSortBy::LastUpdated),
                Some("created_at") => Some(crate::models::ProfileSortBy::CreatedAt),
                _ => None,
            };

            let sort_order = match sort_order.as_str() {
                "asc" => Some(crate::models::SortOrder::Asc),
                "desc" => Some(crate::models::SortOrder::Desc),
                _ => Some(crate::models::SortOrder::Desc),
            };

            // Build profile query
            let profile_query = crate::models::UserProfileQuery {
                fid: None,
                username,
                display_name,
                bio,
                location,
                twitter_username: twitter,
                github_username: github,
                limit: Some(i64::from(limit)),
                offset: None,
                start_timestamp: None,
                end_timestamp: None,
                sort_by,
                sort_order,
                search_term: search,
            };

            let profiles = snaprag.database().list_user_profiles(profile_query).await?;
            print_profile_list(&profiles);
        }
        DataType::Casts => {
            print_list_header("Casts", limit);

            // Build cast query
            let cast_query = crate::models::CastQuery {
                fid: None,
                text_search: search,
                parent_hash: None,
                root_hash: None,
                has_mentions: None,
                has_embeds: None,
                start_timestamp: None,
                end_timestamp: None,
                limit: Some(i64::from(limit)),
                offset: None,
                sort_by: Some(crate::models::CastSortBy::Timestamp),
                sort_order: Some(crate::models::SortOrder::Desc),
            };

            let casts = snaprag.database().list_casts(cast_query).await?;
            print_cast_list(&casts);
        }
        DataType::Follows => {
            print_list_header("Follows", limit);

            // Build link query for follows
            let link_query = crate::models::LinkQuery {
                fid: None,
                target_fid: None,
                link_type: Some("follow".to_string()),
                start_timestamp: None,
                end_timestamp: None,
                limit: Some(i64::from(limit)),
                offset: None,
                sort_by: Some(crate::models::LinkSortBy::Timestamp),
                sort_order: Some(crate::models::SortOrder::Desc),
            };

            let links = snaprag.database().list_links(link_query).await?;
            print_link_list(&links);
        }
        DataType::UserData => {
            print_list_header("User Data", limit);

            // Build user data query
            let user_data_query = crate::models::UserDataQuery {
                fid: None,
                data_type: None,
                value_search: search.clone(),
                start_timestamp: None,
                end_timestamp: None,
                limit: Some(i64::from(limit)),
                offset: None,
                sort_by: Some(crate::models::UserDataSortBy::Timestamp),
                sort_order: Some(crate::models::SortOrder::Desc),
            };

            let user_data = snaprag.database().list_user_data(user_data_query).await?;
            print_user_data_list(&user_data);
        }
    }
    Ok(())
}

/// Handle search command
pub async fn handle_search_command(
    snaprag: &SnapRag,
    query: String,
    limit: u32,
    fields: String,
) -> Result<()> {
    print_info(&format!("🔍 Searching: \"{query}\""));

    let profile_query = crate::models::UserProfileQuery {
        fid: None,
        username: if fields.contains("username") || fields == "all" {
            Some(query.clone())
        } else {
            None
        },
        display_name: if fields.contains("display_name") || fields == "all" {
            Some(query.clone())
        } else {
            None
        },
        bio: if fields.contains("bio") || fields == "all" {
            Some(query.clone())
        } else {
            None
        },
        location: None,
        twitter_username: None,
        github_username: None,
        limit: Some(i64::from(limit)),
        offset: None,
        start_timestamp: None,
        end_timestamp: None,
        sort_by: Some(crate::models::ProfileSortBy::LastUpdated),
        sort_order: Some(crate::models::SortOrder::Desc),
        search_term: Some(query),
    };

    let profiles = snaprag.database().list_user_profiles(profile_query).await?;

    if profiles.is_empty() {
        print_warning("No results found");
    } else {
        print_profile_list(&profiles);
    }

    Ok(())
}

/// Handle activity command
pub async fn handle_activity_command(
    snaprag: &SnapRag,
    fid: i64,
    limit: i64,
    offset: i64,
    activity_type: Option<String>,
    detailed: bool,
) -> Result<()> {
    print_info(&format!("🔍 Querying activity timeline for FID {fid}"));

    // Check if profile exists
    let profile = snaprag.database().get_user_profile(fid).await?;
    if profile.is_none() {
        print_error(&format!("❌ Profile not found for FID {fid}"));
        return Ok(());
    }

    let profile = profile.unwrap();

    // Get registration activity
    let registration = snaprag
        .database()
        .get_user_activity_timeline(
            fid,
            Some("id_register".to_string()),
            None,
            None,
            Some(1),
            Some(0),
        )
        .await?;

    println!("\n👤 Profile Information:");
    if let Some(username) = &profile.username {
        println!("  Username: @{username}");
    }
    if let Some(display_name) = &profile.display_name {
        println!("  Display Name: {display_name}");
    }
    println!("  FID: {fid}");

    // Show registration time if available
    if let Some(reg) = registration.first() {
        if reg.timestamp > 0 {
            if let Some(dt) = chrono::DateTime::from_timestamp(reg.timestamp, 0) {
                println!("  🆕 Registered: {}", dt.format("%Y-%m-%d %H:%M:%S UTC"));
            }
        }

        // Show registration block if available
        if let Some(data) = &reg.activity_data {
            if let Some(block) = data.get("block_number") {
                println!("  📦 Registration Block: {block}");
            }
        }
    }

    println!();

    // Get activities
    let activities = snaprag
        .database()
        .get_user_activity_timeline(
            fid,
            activity_type.clone(),
            None,
            None,
            Some(limit),
            Some(offset),
        )
        .await?;

    if activities.is_empty() {
        print_warning("No activities found for this user");
        return Ok(());
    }

    // Group activities by type for summary
    let mut type_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for activity in &activities {
        *type_counts
            .entry(activity.activity_type.clone())
            .or_insert(0) += 1;
    }

    // Print summary
    println!("📊 Activity Summary ({} total):", activities.len());
    let mut sorted_types: Vec<_> = type_counts.iter().collect();
    sorted_types.sort_by(|a, b| b.1.cmp(a.1));
    for (activity_type, count) in sorted_types {
        let icon = match activity_type.as_str() {
            "cast_add" => "✍️",
            "cast_remove" => "🗑️",
            "reaction_add" => "❤️",
            "reaction_remove" => "💔",
            "link_add" => "👥",
            "link_remove" => "👋",
            "verification_add" => "✅",
            "verification_remove" => "❌",
            "user_data_add" => "📝",
            "id_register" => "🆕",
            "storage_rent" => "💰",
            "signer_event" => "🔑",
            _ => "📌",
        };
        println!("  {icon} {activity_type}: {count}");
    }
    println!();

    // Print activity timeline
    println!("📅 Activity Timeline:");
    println!("{}", "─".repeat(100));

    for (idx, activity) in activities.iter().enumerate() {
        let icon = match activity.activity_type.as_str() {
            "cast_add" => "✍️",
            "cast_remove" => "🗑️",
            "reaction_add" => "❤️",
            "reaction_remove" => "💔",
            "link_add" => "👥",
            "link_remove" => "👋",
            "verification_add" => "✅",
            "verification_remove" => "❌",
            "user_data_add" => "📝",
            "id_register" => "🆕",
            "storage_rent" => "💰",
            "signer_event" => "🔑",
            _ => "📌",
        };

        // Format timestamp
        let timestamp_str = if activity.timestamp > 0 {
            chrono::DateTime::from_timestamp(activity.timestamp, 0).map_or_else(
                || activity.timestamp.to_string(),
                |dt| dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            )
        } else {
            "N/A".to_string()
        };

        println!(
            "{:3}. {} {} | {}",
            offset + idx as i64 + 1,
            icon,
            activity.activity_type,
            timestamp_str
        );

        if detailed {
            if let Some(data) = &activity.activity_data {
                println!(
                    "     Data: {}",
                    serde_json::to_string_pretty(data).unwrap_or_default()
                );
            }
            if let Some(hash) = &activity.message_hash {
                println!("     Hash: {}", hex::encode(hash));
            }
            println!();
        }
    }

    println!("{}", "─".repeat(100));
    println!(
        "\n💡 Tip: Use --limit and --offset for pagination, --activity-type to filter, --detailed for full data"
    );

    Ok(())
}
