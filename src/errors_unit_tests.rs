//! Additional error handling unit tests

#[cfg(test)]
mod tests {
    use crate::errors::{Result, SnapRagError};

    #[test]
    fn test_error_variants() {
        let errors = vec![
            SnapRagError::Custom("custom".to_string()),
            SnapRagError::ConfigError("config".to_string()),
            SnapRagError::DatabaseError("database".to_string()),
            SnapRagError::EmbeddingError("embedding".to_string()),
            SnapRagError::LlmError("llm".to_string()),
            SnapRagError::SyncError("sync".to_string()),
        ];

        assert_eq!(errors.len(), 6);
        
        for error in &errors {
            let display = format!("{}", error);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_result_combinators() {
        let ok_result: Result<i32> = Ok(42);
        let err_result: Result<i32> = Err(SnapRagError::Custom("fail".to_string()));

        // Test map
        let mapped_ok = ok_result.clone().map(|v| v * 2);
        assert_eq!(mapped_ok.unwrap(), 84);

        let mapped_err = err_result.clone().map(|v| v * 2);
        assert!(mapped_err.is_err());

        // Test and_then
        let chained_ok = ok_result.clone().and_then(|v| Ok(v + 10));
        assert_eq!(chained_ok.unwrap(), 52);

        // Test or_else
        let recovered = err_result.clone().or_else(|_| Ok(100));
        assert_eq!(recovered.unwrap(), 100);
    }

    #[test]
    fn test_error_not_found() {
        let error = SnapRagError::UserNotFound(123);
        assert!(matches!(error, SnapRagError::UserNotFound(123)));
        
        let display = format!("{}", error);
        assert!(display.contains("123"));
    }

    #[test]
    fn test_io_error_conversion() {
        use std::io;

        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let snap_err: SnapRagError = io_err.into();
        
        match snap_err {
            SnapRagError::Io(e) => {
                assert_eq!(e.kind(), io::ErrorKind::PermissionDenied);
            }
            _ => panic!("Expected Io error"),
        }
    }

    #[test]
    fn test_parse_error_conversion() {
        let parse_err = "not a number".parse::<i64>().unwrap_err();
        let snap_err: SnapRagError = parse_err.into();
        
        assert!(matches!(snap_err, SnapRagError::ParseError(_)));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid}").unwrap_err();
        let snap_err: SnapRagError = json_err.into();
        
        assert!(matches!(snap_err, SnapRagError::JsonError(_)));
    }

    #[test]
    fn test_error_chain_preservation() {
        use std::io;

        // Create nested error
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file.txt");
        let snap_err: SnapRagError = io_err.into();
        
        // Error message should preserve context
        let msg = format!("{}", snap_err);
        assert!(msg.contains("file.txt") || msg.contains("NotFound"));
    }
}

