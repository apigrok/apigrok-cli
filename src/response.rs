use colored::*;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub protocol_version: String,
    pub elapsed_ms: u128,
}

impl Response {
    pub fn new(
        status: u16,
        status_text: String,
        headers: HashMap<String, String>,
        body: Vec<u8>,
        protocol_version: String,
        elapsed_ms: u128,
    ) -> Self {
        Self {
            status,
            status_text,
            headers,
            body,
            protocol_version,
            elapsed_ms,
        }
    }

    pub fn display(&self, verbose: bool, output_format: &str) {
        // In verbose mode, status line and headers are already shown by VerboseInfo
        // Only display if not verbose
        if !verbose {
            // Status line
            let status_color = if self.status >= 200 && self.status < 300 {
                self.status.to_string().green()
            } else if self.status >= 300 && self.status < 400 {
                self.status.to_string().yellow()
            } else {
                self.status.to_string().red()
            };

            println!(
                "{} {} {} {}",
                self.protocol_version.bright_black(),
                status_color.bold(),
                self.status_text,
                format!("({}ms)", self.elapsed_ms).bright_black()
            );
        }

        // Body
        if !self.body.is_empty() {
            println!();
            let body_str = String::from_utf8_lossy(&self.body);

            match output_format {
                "json" => self.display_json(&body_str),
                "plain" => println!("{}", body_str),
                "auto" | _ => {
                    // Try to detect content type
                    if let Some(content_type) = self.headers.get("content-type") {
                        if content_type.contains("application/json") {
                            self.display_json(&body_str);
                        } else {
                            println!("{}", body_str);
                        }
                    } else {
                        // Try to parse as JSON
                        if serde_json::from_str::<Value>(&body_str).is_ok() {
                            self.display_json(&body_str);
                        } else {
                            println!("{}", body_str);
                        }
                    }
                }
            }
        }
        println!();
    }

    fn display_json(&self, body_str: &str) {
        match serde_json::from_str::<Value>(body_str) {
            Ok(json) => {
                if let Ok(pretty) = serde_json::to_string_pretty(&json) {
                    println!("{}", Self::colorize_json(&pretty));
                } else {
                    println!("{}", body_str);
                }
            }
            Err(_) => println!("{}", body_str),
        }
    }

    fn colorize_json(json: &str) -> String {
        let mut result = String::new();
        let mut in_string = false;
        let mut escape_next = false;

        for ch in json.chars() {
            if escape_next {
                result.push(ch);
                escape_next = false;
                continue;
            }

            if ch == '\\' && in_string {
                escape_next = true;
                result.push(ch);
                continue;
            }

            if ch == '"' {
                in_string = !in_string;
                result.push_str(&ch.to_string().yellow().to_string());
            } else if in_string {
                result.push_str(&ch.to_string().green().to_string());
            } else if ch.is_numeric() || ch == '-' {
                result.push_str(&ch.to_string().magenta().to_string());
            } else if ch == '{' || ch == '}' || ch == '[' || ch == ']' {
                result.push_str(&ch.to_string().bright_white().bold().to_string());
            } else if ch == ':' {
                result.push_str(&ch.to_string().bright_black().to_string());
            } else {
                result.push(ch);
            }
        }

        result
    }

    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}
