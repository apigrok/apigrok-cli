# APIGrok CLI - API Explorer

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/apigrok/apigrok-cli/actions/workflows/rust.yml/badge.svg)](https://github.com/apigrok/apigrok-cli/actions)
[![Crates.io](https://img.shields.io/crates/v/apigrok.svg)](https://crates.io/crates/apigrok)

A powerful CLI tool built with Rust for exploring, testing, and understanding APIs. APIGrok CLI helps developers quickly interact with any APIs and comprehend their structure.

## Features ✨

- **Interactive API exploration** with TUI interface
- **HTTP methods support**: GET, POST, PUT, DELETE, PATCH
- **Authentication helpers** for OAuth, JWT, Basic Auth
- **Response visualization** with syntax highlighting
- **Request history** and collections
- **Environment variables** support
- **Code generation** for multiple languages
- **OpenAPI/Swagger integration**

## Installation 🛠️

### From Cargo (Recommended)
```bash
cargo install apigrok-cli
```

### From Source
```bash
git clone https://github.com/apigrok/apigrok-cli.git
cd apigrok-cli
cargo install --path .
```

Quick Start 🚀

```bash
# Make a GET request
apigrok fetch https://api.example.com/users
```

# Usage 📖

### Basic Commands

```bash
apigrok [METHOD] URL [OPTIONS]
METHOD: get, post, put, delete, patch (default: get)
```

### Options
| Option | Description|
|--------|------------|
| -d, --data | Request body data |
| -H, --header | Add custom header|
| -q, --query | Add query parameters |
| -e, --env	| Use environment file |
| -o, --output | Output format (json, yaml, table) |
| --save | Save request to collection |
| --docs | Generate API documentation |

### Interactive Mode
Launch the terminal user interface:
```bash
apigrok -i
```
Examples 🧪

```bash
# GET request with query parameters
apigrok fetch "https://api.example.com/search?q=rust"

# POST with JSON body and headers
apigrok post https://api.example.com/auth \
  -d '{"username": "user", "password": "pass"}' \
  -H "Content-Type: application/json"

# Generate TypeScript interface from response
apigrok get https://api.example.com/users/1 --output ts-interface
```

# Configuration ⚙️
Create a `~/.apigrok/config.toml` file for persistent settings:
```bash
toml
Copy
[default]
output = "json"
theme = "dark"

[auth.prod]
type = "bearer"
token = "your_token_here"

[env.prod]
base_url = "https://api.example.com/v1"
```

# Contributing 🤝
We welcome contributions! Please read our [Contributing Guidelines]() for details.

1. Fork the repository

2. Create your feature branch (`git checkout -b feature/AmazingFeature`)

3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)

4. Push to the branch (`git push origin feature/AmazingFeature`)

5. Open a Pull Request

# Roadmap 🗺️
* Basic HTTP client functionality
* Interactive TUI mode
* WebSocket support
* GraphQL query builder
* Plugin system for extensions

# License 📜
This project is licensed under the MIT License - see the LICENSE file for details.

# Acknowledgments 🙏

* Inspired by tools like Postman, httpie, and curl
* Built with amazing Rust crates: reqwest, clap, serde, tui-rs