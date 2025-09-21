use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
            1 => UserDataType::Pfp,
            2 => UserDataType::Display,
            3 => UserDataType::Bio,
            5 => UserDataType::Url,
            6 => UserDataType::Username,
            7 => UserDataType::Location,
            8 => UserDataType::Twitter,
            9 => UserDataType::Github,
            10 => UserDataType::Banner,
            11 => UserDataType::PrimaryAddressEthereum,
            12 => UserDataType::PrimaryAddressSolana,
            13 => UserDataType::ProfileToken,
            _ => UserDataType::None,
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
            1 => UsernameType::Fname,
            2 => UsernameType::EnsL1,
            3 => UsernameType::Basename,
            _ => UsernameType::None,
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
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub start_timestamp: Option<i64>,
    pub end_timestamp: Option<i64>,
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
