use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// User data types as defined in Farcaster protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum UserDataType {
    None = 0,
    Pfp = 1,                     // Profile Picture
    Display = 2,                 // Display Name
    Bio = 3,                     // Bio
    Url = 5,                     // URL
    Username = 6,                // Username
    Location = 7,                // Location
    Twitter = 8,                 // Twitter username
    Github = 9,                  // GitHub username
    Banner = 10,                 // Banner image
    PrimaryAddressEthereum = 11, // Primary Ethereum address
    PrimaryAddressSolana = 12,   // Primary Solana address
    ProfileToken = 13,           // Profile token (CAIP-19 format)
}

impl From<i16> for UserDataType {
    fn from(value: i16) -> Self {
        match value {
            1 => Self::Pfp,
            2 => Self::Display,
            3 => Self::Bio,
            5 => Self::Url,
            6 => Self::Username,
            7 => Self::Location,
            8 => Self::Twitter,
            9 => Self::Github,
            10 => Self::Banner,
            11 => Self::PrimaryAddressEthereum,
            12 => Self::PrimaryAddressSolana,
            13 => Self::ProfileToken,
            _ => Self::None,
        }
    }
}

/// Username types as defined in Farcaster protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum UsernameType {
    None = 0,
    Fname = 1,    // Farcaster name
    EnsL1 = 2,    // ENS L1
    Basename = 3, // Basename
}

impl From<i32> for UsernameType {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::Fname,
            2 => Self::EnsL1,
            3 => Self::Basename,
            _ => Self::None,
        }
    }
}

/// Current user profile (latest state only)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserProfile {
    pub id: Uuid,
    pub fid: i64,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub pfp_url: Option<String>,
    pub banner_url: Option<String>,
    pub location: Option<String>,
    pub website_url: Option<String>,
    pub twitter_username: Option<String>,
    pub github_username: Option<String>,
    pub primary_address_ethereum: Option<String>,
    pub primary_address_solana: Option<String>,
    pub profile_token: Option<String>,
    pub profile_embedding: Option<Vec<f32>>,
    pub bio_embedding: Option<Vec<f32>>,
    pub interests_embedding: Option<Vec<f32>>,
    pub last_updated_timestamp: i64,
    pub last_updated_at: DateTime<Utc>,
    pub shard_id: Option<i32>,
    pub block_height: Option<i64>,
    pub transaction_fid: Option<i64>,
}

/// User profile snapshot (historical state)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserProfileSnapshot {
    pub id: Uuid,
    pub fid: i64,
    pub snapshot_timestamp: i64,
    pub message_hash: Vec<u8>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub pfp_url: Option<String>,
    pub banner_url: Option<String>,
    pub location: Option<String>,
    pub website_url: Option<String>,
    pub twitter_username: Option<String>,
    pub github_username: Option<String>,
    pub primary_address_ethereum: Option<String>,
    pub primary_address_solana: Option<String>,
    pub profile_token: Option<String>,
    pub profile_embedding: Option<Vec<f32>>,
    pub bio_embedding: Option<Vec<f32>>,
    pub interests_embedding: Option<Vec<f32>>,
    pub created_at: DateTime<Utc>,
    pub shard_id: Option<i32>,
    pub block_height: Option<i64>,
    pub transaction_fid: Option<i64>,
}

/// User data change record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserDataChange {
    pub id: Uuid,
    pub fid: i64,
    pub data_type: i16,
    pub old_value: Option<String>,
    pub new_value: String,
    pub change_timestamp: i64,
    pub message_hash: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub shard_id: Option<i32>,
    pub block_height: Option<i64>,
    pub transaction_fid: Option<i64>,
}

/// Username proof record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UsernameProof {
    pub id: Uuid,
    pub fid: i64,
    pub username: String,
    pub username_type: i32,
    pub owner_address: String,
    pub signature: Vec<u8>,
    pub timestamp: i64,
    pub created_at: DateTime<Utc>,
    pub shard_id: Option<i32>,
    pub block_height: Option<i64>,
    pub transaction_fid: Option<i64>,
}

/// User activity timeline record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserActivityTimeline {
    pub id: Uuid,
    pub fid: i64,
    pub activity_type: String,
    pub activity_data: Option<serde_json::Value>,
    pub timestamp: i64,
    pub message_hash: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub shard_id: Option<i32>,
    pub block_height: Option<i64>,
    pub transaction_fid: Option<i64>,
}

/// Cast search result with similarity score and engagement metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastSearchResult {
    pub message_hash: Vec<u8>,
    pub fid: i64,
    pub text: String,
    pub timestamp: i64,
    pub parent_hash: Option<Vec<u8>>,
    pub embeds: Option<serde_json::Value>,
    pub mentions: Option<serde_json::Value>,
    pub similarity: f32,
    #[serde(default)]
    pub reply_count: i64,
    #[serde(default)]
    pub reaction_count: i64,
}

/// Cast statistics
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CastStats {
    pub message_hash: Vec<u8>,
    pub reply_count: i64,
    pub reaction_count: i64,
    pub unique_reactors: i64,
}

/// User profile trend record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserProfileTrend {
    pub id: Uuid,
    pub fid: i64,
    pub trend_period: String,
    pub trend_date: chrono::NaiveDate,
    pub profile_changes_count: i32,
    pub bio_changes_count: i32,
    pub username_changes_count: i32,
    pub activity_score: f64,
    pub engagement_score: f64,
    pub profile_embedding: Option<Vec<f32>>,
    pub bio_embedding: Option<Vec<f32>>,
    pub created_at: DateTime<Utc>,
    pub shard_id: Option<i32>,
    pub block_height: Option<i64>,
}

/// Cast message record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Cast {
    pub id: Uuid,
    pub fid: i64,
    pub text: Option<String>,
    pub timestamp: i64,
    pub message_hash: Vec<u8>,
    pub parent_hash: Option<Vec<u8>>,
    pub root_hash: Option<Vec<u8>>,
    pub embeds: Option<serde_json::Value>,
    pub mentions: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub shard_id: Option<i32>,
    pub block_height: Option<i64>,
    pub transaction_fid: Option<i64>,
}

/// Link relationship record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Link {
    pub id: Uuid,
    pub fid: i64,
    pub target_fid: i64,
    pub link_type: String,
    pub timestamp: i64,
    pub message_hash: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub shard_id: Option<i32>,
    pub block_height: Option<i64>,
    pub transaction_fid: Option<i64>,
}

/// User data record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserData {
    pub id: Uuid,
    pub fid: i64,
    pub data_type: i16,
    pub value: String,
    pub timestamp: i64,
    pub message_hash: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub shard_id: Option<i32>,
    pub block_height: Option<i64>,
    pub transaction_fid: Option<i64>,
}

/// User activity record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserActivity {
    pub id: Uuid,
    pub fid: i64,
    pub activity_type: String,
    pub activity_data: Option<String>,
    pub timestamp: i64,
    pub message_hash: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub shard_id: Option<i32>,
    pub block_height: Option<i64>,
    pub transaction_fid: Option<i64>,
}

/// Reaction record (likes and recasts)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Reaction {
    pub id: Uuid,
    pub fid: i64,
    pub target_cast_hash: Vec<u8>,
    pub target_fid: Option<i64>,
    pub reaction_type: i16, // 1=like, 2=recast
    pub timestamp: i64,
    pub message_hash: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub shard_id: Option<i32>,
    pub block_height: Option<i64>,
    pub transaction_fid: Option<i64>,
}

/// Verification record (address verifications)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Verification {
    pub id: Uuid,
    pub fid: i64,
    pub address: Vec<u8>,
    pub claim_signature: Option<Vec<u8>>,
    pub block_hash: Option<Vec<u8>>,
    pub verification_type: Option<i16>,
    pub chain_id: Option<i32>,
    pub timestamp: i64,
    pub message_hash: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub shard_id: Option<i32>,
    pub block_height: Option<i64>,
    pub transaction_fid: Option<i64>,
}

/// Shard and block information for tracking data provenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardBlockInfo {
    pub shard_id: u32,
    pub block_height: u64,
    pub transaction_fid: u64,
    pub timestamp: u64,
}

impl ShardBlockInfo {
    #[must_use] 
    pub const fn new(shard_id: u32, block_height: u64, transaction_fid: u64, timestamp: u64) -> Self {
        Self {
            shard_id,
            block_height,
            transaction_fid,
            timestamp,
        }
    }
}

/// Create user profile request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserProfileRequest {
    pub id: uuid::Uuid,
    pub fid: i64,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub pfp_url: Option<String>,
    pub banner_url: Option<String>,
    pub location: Option<String>,
    pub website_url: Option<String>,
    pub twitter_username: Option<String>,
    pub github_username: Option<String>,
    pub primary_address_ethereum: Option<String>,
    pub primary_address_solana: Option<String>,
    pub profile_token: Option<String>,
    pub created_at: i64,
    pub message_hash: Option<Vec<u8>>,
}

/// Update user profile request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserProfileRequest {
    pub fid: i64,
    pub data_type: UserDataType,
    pub new_value: String,
    pub message_hash: Vec<u8>,
    pub timestamp: i64,
}

/// User profile query filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfileQuery {
    pub fid: Option<i64>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub location: Option<String>,
    pub twitter_username: Option<String>,
    pub github_username: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub start_timestamp: Option<i64>,
    pub end_timestamp: Option<i64>,
    pub sort_by: Option<ProfileSortBy>,
    pub sort_order: Option<SortOrder>,
    pub search_term: Option<String>,
}

/// Profile sorting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProfileSortBy {
    Fid,
    Username,
    DisplayName,
    LastUpdated,
    CreatedAt,
}

/// Sort order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// FID query filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FidQuery {
    pub fid: Option<i64>,
    pub min_fid: Option<i64>,
    pub max_fid: Option<i64>,
    pub has_username: Option<bool>,
    pub has_display_name: Option<bool>,
    pub has_bio: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<FidSortBy>,
    pub sort_order: Option<SortOrder>,
    pub search_term: Option<String>,
}

/// FID sorting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FidSortBy {
    Fid,
    Username,
    LastUpdated,
    CreatedAt,
}

/// Statistics query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub group_by: Option<StatisticsGroupBy>,
}

/// Statistics grouping options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatisticsGroupBy {
    Day,
    Week,
    Month,
    Year,
}

/// Statistics result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsResult {
    pub total_fids: i64,
    pub total_profiles: i64,
    pub complete_profiles: i64, // Has username + display_name + bio
    pub profiles_with_username: i64,
    pub profiles_with_display_name: i64,
    pub profiles_with_bio: i64,
    pub profiles_with_pfp: i64,
    pub profiles_with_website: i64,
    pub profiles_with_location: i64,
    pub profiles_with_twitter: i64,
    pub profiles_with_github: i64,
    pub profiles_with_ethereum_address: i64,
    pub profiles_with_solana_address: i64,
    pub recent_registrations: Vec<ProfileRegistration>,
    pub top_usernames: Vec<UsernameStats>,
    pub growth_by_period: Vec<GrowthStats>,
    // Activity statistics
    pub total_activities: i64,
    pub total_casts: i64,
    pub activities_by_type: Vec<ActivityTypeStats>,
}

/// Activity type statistics
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ActivityTypeStats {
    pub activity_type: String,
    pub count: i64,
}

/// Profile registration data
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProfileRegistration {
    pub fid: i64,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Username statistics
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UsernameStats {
    pub username: String,
    pub count: i64,
    pub percentage: f64,
}

/// Growth statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthStats {
    pub period: String,
    pub new_registrations: i64,
    pub total_fids: i64,
    pub growth_rate: f64,
}

/// Profile snapshot query filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSnapshotQuery {
    pub fid: i64,
    pub start_timestamp: Option<i64>,
    pub end_timestamp: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Record user data change request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordUserDataChangeRequest {
    pub fid: i64,
    pub data_type: UserDataType,
    pub old_value: Option<String>,
    pub new_value: String,
    pub message_hash: Vec<u8>,
    pub timestamp: i64,
}

/// Record user activity request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordUserActivityRequest {
    pub fid: i64,
    pub activity_type: String,
    pub activity_data: serde_json::Value,
    pub timestamp: i64,
    pub message_hash: Option<Vec<u8>>,
}

/// Cast query filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastQuery {
    pub fid: Option<i64>,
    pub text_search: Option<String>,
    pub parent_hash: Option<Vec<u8>>,
    pub root_hash: Option<Vec<u8>>,
    pub has_mentions: Option<bool>,
    pub has_embeds: Option<bool>,
    pub start_timestamp: Option<i64>,
    pub end_timestamp: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<CastSortBy>,
    pub sort_order: Option<SortOrder>,
}

/// Cast sorting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CastSortBy {
    Timestamp,
    Fid,
    Text,
    CreatedAt,
}

/// Link query filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkQuery {
    pub fid: Option<i64>,
    pub target_fid: Option<i64>,
    pub link_type: Option<String>,
    pub start_timestamp: Option<i64>,
    pub end_timestamp: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<LinkSortBy>,
    pub sort_order: Option<SortOrder>,
}

/// Link sorting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LinkSortBy {
    Timestamp,
    Fid,
    TargetFid,
    LinkType,
    CreatedAt,
}

/// User data query filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDataQuery {
    pub fid: Option<i64>,
    pub data_type: Option<i16>,
    pub value_search: Option<String>,
    pub start_timestamp: Option<i64>,
    pub end_timestamp: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<UserDataSortBy>,
    pub sort_order: Option<SortOrder>,
}

/// User data sorting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserDataSortBy {
    Timestamp,
    Fid,
    DataType,
    Value,
    CreatedAt,
}
