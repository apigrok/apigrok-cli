# Contribution Guide

## Development Environment

### Prerequisites
- Rust 1.85+ (recommended: use `rustup`)
- Cargo
- Git

### Setup
1. Fork and clone the repository
2. Install dependencies: `cargo build`
3. Verify setup: `cargo test`

## Workflow

### Making Changes
1. Create a feature branch: `git checkout -b feat/new-command`
2. Implement your changes
3. Run checks:
   ```bash
   cargo fmt
   cargo clippy
   cargo test
   ```