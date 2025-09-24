//! SnapRAG Build Script
//!
//! This build script handles:
//! 1. SQLx compilation by setting SQLX_OFFLINE=true to avoid database connection issues during build
//! 2. Protobuf compilation for gRPC services

use std::env;
use std::fs;

fn main() {
    // Check if we should use offline mode
    let use_offline = env::var("SQLX_OFFLINE").unwrap_or_else(|_| "false".to_string()) == "true";

    if use_offline {
        println!("cargo:rustc-env=SQLX_OFFLINE=true");
        println!("cargo:warning=Using SQLX_OFFLINE mode - make sure .sqlx/ directory exists with prepared queries");
    } else {
        // Try to use database connection for live query validation
        println!("cargo:warning=Attempting live SQLx query validation (may fail if database is unavailable)");
    }

    // Check if DATABASE_URL is already set
    match env::var("DATABASE_URL") {
        Ok(_) => {
            println!("cargo:warning=Using provided DATABASE_URL for SQLx compilation");
            println!("cargo:rerun-if-env-changed=DATABASE_URL");
        }
        Err(_) => {
            // Try to read database URL from config.toml
            match read_database_url_from_config() {
                Ok(database_url) => {
                    println!("cargo:rustc-env=DATABASE_URL={}", database_url);

                    // Also set it for the current process so sqlx can use it
                    env::set_var("DATABASE_URL", &database_url);

                    println!(
                        "cargo:warning=Using database URL from config.toml for SQLx compilation"
                    );
                    println!(
                        "cargo:warning=Runtime database connection is configured via config.toml"
                    );
                }
                Err(e) => {
                    println!("cargo:warning=Failed to read config.toml: {}", e);
                    println!("cargo:warning=Please set DATABASE_URL environment variable or ensure config.toml exists");

                    // Fallback to a generic URL for compilation (won't work for actual queries)
                    let fallback_url = "postgresql://user:pass@localhost/db";
                    println!("cargo:rustc-env=DATABASE_URL={}", fallback_url);
                    env::set_var("DATABASE_URL", fallback_url);
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

    println!(
        "cargo:warning=Compiling {} protobuf files",
        existing_proto_files.len()
    );

    // Create output directory
    let out_dir = "src/generated";
    if let Err(e) = fs::create_dir_all(out_dir) {
        println!("cargo:warning=Failed to create output directory: {}", e);
        return;
    }

    // Compile all proto files at once using protobuf-codegen-pure for messages
    match protobuf_codegen_pure::Codegen::new()
        .out_dir(out_dir)
        .inputs(&existing_proto_files)
        .include("proto/")
        .run()
    {
        Ok(_) => {
            println!(
                "cargo:warning=Successfully compiled {} protobuf files",
                existing_proto_files.len()
            );

            // Add #[allow] attributes to generated files to suppress warnings
            add_allow_attributes_to_generated_files(out_dir);
        }
        Err(e) => {
            println!("cargo:warning=Failed to compile protobuf files: {}", e);
            println!("cargo:warning=Continuing build without protobuf support");
        }
    }

    // Generate gRPC client code using tonic-build
    generate_grpc_client();
}

/// Add #[allow] attributes to generated protobuf files to suppress warnings
fn add_allow_attributes_to_generated_files(out_dir: &str) {
    let allow_attributes = "#![allow(unused_lifetimes)]\n#![allow(clippy::all)]\n";

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
    ];

    for file_name in &generated_files {
        let file_path = format!("{}/{}", out_dir, file_name);
        if let Ok(content) = fs::read_to_string(&file_path) {
            // Check if the file already has allow attributes
            if !content.contains("#![allow(unused_lifetimes)]") {
                let modified_content = format!("{}\n{}", allow_attributes, content);
                if let Err(e) = fs::write(&file_path, modified_content) {
                    println!(
                        "cargo:warning=Failed to add allow attributes to {}: {}",
                        file_name, e
                    );
                } else {
                    println!("cargo:warning=Added allow attributes to {}", file_name);
                }
            }
        }
    }
}

/// Generate gRPC client code using tonic-build
fn generate_grpc_client() {
    let out_dir = "src/generated";

    // Generate gRPC client for the main RPC service
    if fs::metadata("proto/rpc.proto").is_ok() {
        match tonic_build::configure()
            .out_dir(out_dir)
            .compile(&["proto/rpc.proto"], &["proto/"])
        {
            Ok(_) => {
                println!("cargo:warning=Successfully generated gRPC client code");
            }
            Err(e) => {
                println!("cargo:warning=Failed to generate gRPC client: {}", e);
                println!("cargo:warning=Continuing build without gRPC client");
            }
        }
    }
}
