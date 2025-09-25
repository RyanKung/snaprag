#!/bin/bash

# Strict test runner for SnapRAG
# This script runs tests with the strictest possible settings

set -e

echo "🚀 Starting strict test execution for SnapRAG..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Step 1: Code formatting check
print_status "Checking code formatting..."
if cargo fmt --all -- --check; then
    print_success "Code formatting check passed"
else
    print_error "Code formatting check failed"
    print_status "Running cargo fmt to fix formatting..."
    cargo fmt --all
    print_warning "Code has been reformatted. Please review changes."
    exit 1
fi

# Step 2: Clippy check (with allowances for generated code)
print_status "Running clippy with strict settings..."
if cargo clippy --all-targets --all-features -- \
    -D warnings \
    -D clippy::all \
    -D clippy::pedantic \
    --allow unused_lifetimes \
    --allow elided-lifetimes-in-paths \
    --allow unused_imports \
    --allow unused_variables; then
    print_success "Clippy check passed"
else
    print_error "Clippy check failed"
    exit 1
fi

# Step 3: Run tests with strict settings
print_status "Running tests with strict settings..."
if cargo test --lib -- --test-threads 1; then
    print_success "All tests passed"
else
    print_error "Tests failed"
    exit 1
fi

# Step 4: Run specific strict validation tests
print_status "Running strict validation tests..."
if cargo test --lib strict_validation_tests -- --test-threads 1; then
    print_success "Strict validation tests passed"
else
    print_error "Strict validation tests failed"
    exit 1
fi

# Step 5: Check for any remaining warnings in non-generated code
print_status "Checking for warnings in non-generated code..."
if cargo check --lib 2>&1 | grep -v "generated/" | grep -v "protobuf" | grep -v "prost" | grep -v "tonic" | grep -i warning; then
    print_warning "Found warnings in non-generated code"
    exit 1
else
    print_success "No warnings found in non-generated code"
fi

print_success "🎉 All strict tests passed! Code is ready for production."