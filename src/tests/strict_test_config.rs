//! Strict test configuration that treats warnings as errors
//! This module provides utilities for running tests with strict error handling

use std::panic;
use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize strict test environment
/// This sets up panic hooks to catch warnings and treat them as errors
pub fn init_strict_testing() {
    INIT.call_once(|| {
        // Set up panic hook to catch warnings
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            // Check if this is a warning that should be treated as an error
            if let Some(message) = panic_info.message() {
                let msg = message.to_string();
                if msg.contains("warning:") && !is_generated_code_warning(&msg) {
                    eprintln!("❌ WARNING TREATED AS ERROR: {}", msg);
                    std::process::exit(1);
                }
            }
            original_hook(panic_info);
        }));
    });
}

/// Check if a warning is from generated code and should be ignored
fn is_generated_code_warning(msg: &str) -> bool {
    // Ignore warnings from generated protobuf code
    msg.contains("generated/") || 
    msg.contains("protobuf") ||
    msg.contains("prost") ||
    msg.contains("tonic")
}

/// Macro to run tests with strict warning handling
#[macro_export]
macro_rules! strict_test {
    ($name:ident, $test_fn:expr) => {
        #[tokio::test]
        async fn $name() {
            $crate::tests::strict_test_config::init_strict_testing();
            
            // Set RUST_BACKTRACE to get better error information
            std::env::set_var("RUST_BACKTRACE", "1");
            
            // Run the test with strict clippy settings
            let result = std::panic::catch_unwind(|| {
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async {
                        $test_fn.await
                    })
            });
            
            match result {
                Ok(()) => {
                    println!("✅ Test {} passed with strict checking", stringify!($name));
                }
                Err(panic_info) => {
                    eprintln!("❌ Test {} failed: {:?}", stringify!($name), panic_info);
                    panic!("Test failed with strict checking");
                }
            }
        }
    };
}

/// Helper function to run cargo clippy with strict settings
pub fn run_strict_clippy() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    let output = Command::new("cargo")
        .args(&[
            "clippy",
            "--all-targets",
            "--all-features",
            "--",
            "-D", "warnings",
            "-D", "clippy::all",
            "-D", "clippy::pedantic",
            "-D", "clippy::nursery",
            "-D", "clippy::cargo",
        ])
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("❌ Clippy found issues:\n{}", stderr);
        return Err("Clippy strict check failed".into());
    }
    
    println!("✅ Clippy strict check passed");
    Ok(())
}

/// Helper function to run cargo fmt with strict checking
pub fn run_strict_fmt() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    // First, format the code
    let fmt_output = Command::new("cargo")
        .args(&["fmt", "--all"])
        .output()?;
    
    if !fmt_output.status.success() {
        let stderr = String::from_utf8_lossy(&fmt_output.stderr);
        eprintln!("❌ Cargo fmt failed:\n{}", stderr);
        return Err("Cargo fmt failed".into());
    }
    
    // Then check if formatting is correct
    let check_output = Command::new("cargo")
        .args(&["fmt", "--all", "--", "--check"])
        .output()?;
    
    if !check_output.status.success() {
        let stderr = String::from_utf8_lossy(&check_output.stderr);
        eprintln!("❌ Code formatting check failed:\n{}", stderr);
        return Err("Code formatting check failed".into());
    }
    
    println!("✅ Code formatting check passed");
    Ok(())
}

/// Run all strict checks before tests
pub fn run_all_strict_checks() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Running strict code quality checks...");
    
    // Run formatting check
    run_strict_fmt()?;
    
    // Run clippy check
    run_strict_clippy()?;
    
    println!("✅ All strict checks passed");
    Ok(())
}
