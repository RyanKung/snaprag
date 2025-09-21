use thiserror::Error;

#[derive(Error, Debug)]
pub enum SnapRagError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("User not found: fid {0}")]
    UserNotFound(u64),
    
    #[error("Profile snapshot not found: fid {0}, timestamp {1}")]
    ProfileSnapshotNotFound(u64, i64),
    
    #[error("Invalid user data type: {0}")]
    InvalidUserDataType(i32),
    
    #[error("Invalid username type: {0}")]
    InvalidUsernameType(i32),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("TOML parsing error: {0}")]
    TomlParsing(#[from] toml::de::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, SnapRagError>;
