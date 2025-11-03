//! Unit tests for error handling
//!
//! Tests error types, conversions, and error message formatting.

#[cfg(test)]
mod tests {
    use crate::errors::SnapRagError;
    use std::io;

    // ====== Error Type Tests ======

    #[test]
    fn test_custom_error() {
        let error = SnapRagError::Custom("Test error message".to_string());
        let display = format!("{}", error);
        assert_eq!(display, "Test error message");
    }

    #[test]
    fn test_config_error() {
        let error = SnapRagError::ConfigError("Invalid configuration".to_string());
        assert!(matches!(error, SnapRagError::ConfigError(_)));
        let display = format!("{}", error);
        assert!(display.contains("configuration"));
    }

    #[test]
    fn test_database_error() {
        let error = SnapRagError::DatabaseError("Connection failed".to_string());
        assert!(matches!(error, SnapRagError::DatabaseError(_)));
    }

    #[test]
    fn test_embedding_error() {
        let error = SnapRagError::EmbeddingError("Generation failed".to_string());
        assert!(matches!(error, SnapRagError::EmbeddingError(_)));
    }

    #[test]
    fn test_llm_error() {
        let error = SnapRagError::LlmError("API call failed".to_string());
        assert!(matches!(error, SnapRagError::LlmError(_)));
    }

    // ====== Error Conversion Tests ======

    #[test]
    fn test_error_from_io() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let snap_err: SnapRagError = io_err.into();
        
        assert!(matches!(snap_err, SnapRagError::Io(_)));
    }

    #[test]
    fn test_error_from_parse_int() {
        let parse_err = "not a number".parse::<i64>().unwrap_err();
        let snap_err: SnapRagError = parse_err.into();
        
        assert!(matches!(snap_err, SnapRagError::ParseError(_)));
    }

    #[test]
    fn test_error_from_serde_json() {
        let json_str = "{invalid json}";
        let parse_result: Result<serde_json::Value, _> = serde_json::from_str(json_str);
        
        if let Err(json_err) = parse_result {
            let snap_err: SnapRagError = json_err.into();
            assert!(matches!(snap_err, SnapRagError::JsonError(_)));
        }
    }

    // ====== Error Debug/Display Tests ======

    #[test]
    fn test_error_debug_format() {
        let error = SnapRagError::Custom("Debug test".to_string());
        let debug = format!("{:?}", error);
        assert!(debug.contains("Custom"));
        assert!(debug.contains("Debug test"));
    }

    #[test]
    fn test_error_display_format() {
        let errors = vec![
            SnapRagError::Custom("Custom message".to_string()),
            SnapRagError::ConfigError("Config issue".to_string()),
            SnapRagError::DatabaseError("DB problem".to_string()),
        ];

        for error in errors {
            let display = format!("{}", error);
            assert!(!display.is_empty());
        }
    }

    // ====== Error Chain Tests ======

    #[test]
    fn test_error_source_chain() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "Root cause");
        let snap_err: SnapRagError = io_err.into();
        
        // Error should preserve source information
        match snap_err {
            SnapRagError::Io(e) => {
                assert_eq!(e.kind(), io::ErrorKind::NotFound);
            }
            _ => panic!("Expected Io error"),
        }
    }

    // ====== Result Type Tests ======

    #[test]
    fn test_result_ok() {
        let result: crate::Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_err() {
        let result: crate::Result<i32> = Err(SnapRagError::Custom("Failed".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_result_map() {
        let result: crate::Result<i32> = Ok(42);
        let mapped = result.map(|v| v * 2);
        assert_eq!(mapped.unwrap(), 84);
    }

    #[test]
    fn test_result_and_then() {
        let result: crate::Result<i32> = Ok(42);
        let chained = result.and_then(|v| {
            if v > 40 {
                Ok(v + 10)
            } else {
                Err(SnapRagError::Custom("Too small".to_string()))
            }
        });
        assert_eq!(chained.unwrap(), 52);
    }
}

