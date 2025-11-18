//! `SnapRAG` Build Script
//!
//! This build script handles:
//! 1. `SQLx` compilation by setting `SQLX_OFFLINE=true` to avoid database connection issues during build
//! 2. Protobuf compilation for gRPC services

use std::env;
use std::fs;

fn main() {
    // Enable verbose build output with VERBOSE_BUILD=1
    let verbose = env::var("VERBOSE_BUILD").unwrap_or_else(|_| "0".to_string()) == "1";

    // Check if we should use offline mode
    // Default to "true" (offline mode) to allow building without database connection
    // This is safe because the project uses runtime queries (query()) instead of compile-time queries (query!())
    let use_offline = env::var("SQLX_OFFLINE").unwrap_or_else(|_| "true".to_string()) == "true";

    if use_offline {
        println!("cargo:rustc-env=SQLX_OFFLINE=true");
        if verbose {
            println!("cargo:warning=Using SQLX_OFFLINE mode - build does not require database connection");
        }
    } else {
        // Try to use database connection for live query validation
        if verbose {
            println!("cargo:warning=Attempting live SQLx query validation (may fail if database is unavailable)");
        }
    }

    // Set DATABASE_URL for SQLx (only used if SQLX_OFFLINE=false)
    // This is optional since we use offline mode by default
    if !use_offline {
        match env::var("DATABASE_URL") {
            Ok(_) => {
                if verbose {
                    println!("cargo:warning=Using provided DATABASE_URL for SQLx compilation");
                }
                println!("cargo:rerun-if-env-changed=DATABASE_URL");
            }
            Err(_) => {
                // Try to read database URL from config.toml
                match read_database_url_from_config() {
                    Ok(database_url) => {
                        println!("cargo:rustc-env=DATABASE_URL={database_url}");
                        env::set_var("DATABASE_URL", &database_url);
                        if verbose {
                            println!("cargo:warning=Using database URL from config.toml for SQLx compilation");
                        }
                    }
                    Err(e) => {
                        if verbose {
                            println!("cargo:warning=Failed to read config.toml: {e}");
                        }
                        // Fallback to a generic URL for compilation
                        let fallback_url = "postgresql://user:pass@localhost/db";
                        println!("cargo:rustc-env=DATABASE_URL={fallback_url}");
                        env::set_var("DATABASE_URL", fallback_url);
                    }
                }
            }
        }
    }

    // Compile protobuf files for gRPC
    compile_protobufs();

    // Tell Cargo to re-run this build script if config files change
    println!("cargo:rerun-if-changed=config.toml");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=proto/");
}

/// Read database URL from config.toml file
fn read_database_url_from_config() -> Result<String, Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string("config.toml")?;

    // Simple TOML parsing for the database.url field
    // This is a basic implementation - for production, consider using a proper TOML library
    for line in config_content.lines() {
        let line = line.trim();
        if line.starts_with("url = ") {
            // Extract the URL from the line
            let url = line
                .strip_prefix("url = ")
                .ok_or("Invalid URL format in config.toml")?
                .trim_matches('"');

            return Ok(url.to_string());
        }
    }

    Err("Database URL not found in config.toml".into())
}

/// Compile protobuf files using protobuf-codegen-pure
fn compile_protobufs() {
    let proto_files = [
        "proto/rpc.proto",
        "proto/blocks.proto",
        "proto/message.proto",
        "proto/hub_event.proto",
        "proto/onchain_event.proto",
        "proto/username_proof.proto",
        "proto/admin_rpc.proto",
        "proto/gossip.proto",
        "proto/node_state.proto",
        "proto/replication.proto",
        "proto/request_response.proto",
        "proto/sync_trie.proto",
    ];

    // Check if proto files exist
    let mut existing_proto_files = Vec::new();
    for proto_file in &proto_files {
        if fs::metadata(proto_file).is_ok() {
            existing_proto_files.push(proto_file);
        }
    }

    if existing_proto_files.is_empty() {
        println!("cargo:warning=No protobuf files found in proto/ directory");
        return;
    }

    let verbose = env::var("VERBOSE_BUILD").unwrap_or_else(|_| "0".to_string()) == "1";

    if verbose {
        println!(
            "cargo:warning=Compiling {} protobuf files",
            existing_proto_files.len()
        );
    }

    // Create output directory
    let out_dir = "src/generated";
    if let Err(e) = fs::create_dir_all(out_dir) {
        println!("cargo:warning=Failed to create output directory: {e}");
        return;
    }

    // Compile all proto files at once using protobuf-codegen for messages
    match protobuf_codegen::Codegen::new()
        .pure()
        .out_dir(out_dir)
        .inputs(existing_proto_files.iter().copied())
        .include("proto")
        .run()
    {
        Ok(()) => {
            if verbose {
                let count = existing_proto_files.len();
                println!("cargo:warning=Successfully compiled {count} protobuf files");
            }

            // Add #[allow] attributes to generated files to suppress warnings
            add_allow_attributes_to_generated_files(out_dir, verbose);
        }
        Err(e) => {
            println!("cargo:warning=Failed to compile protobuf files: {e}");
            println!("cargo:warning=Continuing build without protobuf support");
        }
    }

    // Generate gRPC client code using tonic-build
    generate_grpc_client();
}

/// Format generated files using rustfmt
fn format_generated_files(out_dir: &str, verbose: bool) {
    use std::process::Command;

    let generated_files = [
        "message.rs",
        "onchain_event.rs",
        "request_response.rs",
        "replication.rs",
        "node_state.rs",
        "sync_trie.rs",
        "username_proof.rs",
        "blocks.rs",
        "hub_event.rs",
        "admin_rpc.rs",
        "gossip.rs",
        "_.rs",
    ];

    for file_name in &generated_files {
        let file_path = format!("{out_dir}/{file_name}");
        if fs::metadata(&file_path).is_ok() {
            // Try to format the file using rustfmt
            // Note: This requires rustfmt to be available, which it should be in normal builds
            let output = Command::new("rustfmt")
                .arg("--edition")
                .arg("2021")
                .arg(&file_path)
                .output();

            match output {
                Ok(result) => {
                    if !result.status.success() {
                        if verbose {
                            let stderr = String::from_utf8_lossy(&result.stderr);
                            println!("cargo:warning=Failed to format {file_name}: {stderr}");
                        }
                    } else if verbose {
                        println!("cargo:warning=Formatted {file_name}");
                    }
                }
                Err(_) => {
                    // rustfmt not available, skip formatting
                    if verbose {
                        println!(
                            "cargo:warning=rustfmt not available, skipping format for {file_name}"
                        );
                    }
                }
            }
        }
    }
}

/// Add #[allow] attributes to generated protobuf files to suppress warnings
fn add_allow_attributes_to_generated_files(out_dir: &str, verbose: bool) {
    // Comprehensive allow attributes for generated code to suppress all warnings
    let allow_attributes = "\
#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::nursery)]
#![allow(unused_lifetimes)]
#![allow(elided_lifetimes_in_paths)]
#![allow(unused_parens)]
#![allow(unknown_lints)]
#![allow(renamed_and_removed_lints)]
#![allow(warnings)]
";

    // List of generated files that need the allow attributes
    let generated_files = [
        "message.rs",
        "onchain_event.rs",
        "request_response.rs",
        "replication.rs",
        "node_state.rs",
        "sync_trie.rs",
        "username_proof.rs",
        "blocks.rs",
        "hub_event.rs",
        "admin_rpc.rs",
        "gossip.rs",
        "_.rs", // Combined protobuf definitions generated by protobuf-codegen
    ];

    for file_name in &generated_files {
        let file_path = format!("{out_dir}/{file_name}");
        if let Ok(mut content) = fs::read_to_string(&file_path) {
            // Remove old box_pointers allow attribute if it exists
            content = content.replace("#![allow(box_pointers)]\n", "");

            // Check if the file already has our allow attributes
            if content.contains("#![allow(unused_lifetimes)]") {
                // File already has allow attributes, just remove box_pointers if present
                if content.contains("#![allow(box_pointers)]") {
                    let modified_content = content.replace("#![allow(box_pointers)]\n", "");
                    if let Err(e) = fs::write(&file_path, modified_content) {
                        println!(
                            "cargo:warning=Failed to remove box_pointers from {file_name}: {e}"
                        );
                    } else {
                        println!("cargo:warning=Removed box_pointers from {file_name}");
                    }
                }
            } else {
                let modified_content = format!("{allow_attributes}\n{content}");
                if let Err(e) = fs::write(&file_path, modified_content) {
                    if verbose {
                        println!(
                            "cargo:warning=Failed to add allow attributes to {file_name}: {e}"
                        );
                    }
                } else if verbose {
                    println!("cargo:warning=Added allow attributes to {file_name}");
                }
            }
        }
    }
}

/// Generate gRPC client code using tonic-build
fn generate_grpc_client() {
    let verbose = env::var("VERBOSE_BUILD").unwrap_or_else(|_| "0".to_string()) == "1";
    let out_dir = "src/generated";

    // Generate gRPC client for the main RPC service
    if fs::metadata("proto/rpc.proto").is_ok() {
        match tonic_build::configure()
            .out_dir(out_dir)
            .compile(&["proto/rpc.proto"], &["proto/"])
        {
            Ok(()) => {
                if verbose {
                    println!("cargo:warning=Successfully generated gRPC client code");
                }

                // Add allow attributes to gRPC generated files
                add_allow_attributes_to_grpc_files(out_dir, verbose);

                // Add grpc_client module to mod.rs
                add_grpc_client_to_mod_rs(out_dir, verbose);
            }
            Err(e) => {
                println!("cargo:warning=Failed to generate gRPC client: {e}");
                println!("cargo:warning=Continuing build without gRPC client");
            }
        }
    }
}

/// Add `grpc_client` module declaration to mod.rs
fn add_grpc_client_to_mod_rs(out_dir: &str, verbose: bool) {
    let mod_file = format!("{out_dir}/mod.rs");
    let grpc_client_decl = "\n// gRPC client module generated by tonic-build\n#[path = \"_.rs\"]\npub mod grpc_client;\n";

    if let Ok(content) = fs::read_to_string(&mod_file) {
        // Check if grpc_client is already declared
        if !content.contains("pub mod grpc_client") {
            let new_content = format!("{content}{grpc_client_decl}");
            if let Err(e) = fs::write(&mod_file, new_content) {
                println!("cargo:warning=Failed to add grpc_client to mod.rs: {e}");
            } else if verbose {
                println!("cargo:warning=Added grpc_client module to mod.rs");
            }
        }
    }
}

/// Add #[allow] attributes to gRPC generated files
fn add_allow_attributes_to_grpc_files(out_dir: &str, verbose: bool) {
    // Comprehensive allow attributes for generated gRPC code to suppress all warnings
    let allow_attributes = "\
#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::nursery)]
#![allow(unused_lifetimes)]
#![allow(elided_lifetimes_in_paths)]
#![allow(unused_parens)]
#![allow(unknown_lints)]
#![allow(renamed_and_removed_lints)]
#![allow(warnings)]
";

    // List of gRPC generated files
    let grpc_files = ["rpc.rs"];

    for file_name in &grpc_files {
        let file_path = format!("{out_dir}/{file_name}");
        if let Ok(mut content) = fs::read_to_string(&file_path) {
            // Remove old box_pointers allow attribute if it exists
            content = content.replace("#![allow(box_pointers)]\n", "");

            // Check if the file already has our allow attributes
            if content.contains("#![allow(unused_lifetimes)]") {
                // File already has allow attributes, just remove box_pointers if present
                if content.contains("#![allow(box_pointers)]") {
                    let modified_content = content.replace("#![allow(box_pointers)]\n", "");
                    if let Err(e) = fs::write(&file_path, modified_content) {
                        println!(
                            "cargo:warning=Failed to remove box_pointers from {file_name}: {e}"
                        );
                    } else {
                        println!("cargo:warning=Removed box_pointers from {file_name}");
                    }
                }
            } else {
                let modified_content = format!("{allow_attributes}\n{content}");
                if let Err(e) = fs::write(&file_path, modified_content) {
                    if verbose {
                        println!(
                            "cargo:warning=Failed to add allow attributes to {file_name}: {e}"
                        );
                    }
                } else if verbose {
                    println!("cargo:warning=Added allow attributes to {file_name}");
                }
            }
        }
    }
}
