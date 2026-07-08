use crate::env;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ResolvedAwsCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
    pub region: String,
    pub profile: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct AwsCliProfile {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

pub fn get_var(name: &str) -> Option<String> {
    env::get_var(name)
}

pub fn get_access_key_id() -> Option<String> {
    get_var("AWS_ACCESS_KEY_ID")
}

pub fn get_secret_access_key() -> Option<String> {
    get_var("AWS_SECRET_ACCESS_KEY")
}

pub fn get_session_token() -> Option<String> {
    get_var("AWS_SESSION_TOKEN")
}

pub fn get_region() -> Option<String> {
    get_var("AWS_REGION").or_else(|| get_var("AWS_DEFAULT_REGION"))
}

pub fn get_profile_name() -> Option<String> {
    get_var("AWS_PROFILE")
}

pub fn aws_config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".aws"))
}

pub fn credentials_file_exists() -> bool {
    aws_config_dir()
        .map(|dir| dir.join("credentials").is_file())
        .unwrap_or(false)
}

pub fn parse_ini_file(path: &Path) -> HashMap<String, HashMap<String, String>> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };

    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current = "default".to_string();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current = line[1..line.len() - 1].trim().to_string();
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            sections
                .entry(current.clone())
                .or_default()
                .insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    sections
}

pub fn load_profile_from_files(profile: &str) -> Option<AwsCliProfile> {
    let config_dir = aws_config_dir()?;
    let credentials = parse_ini_file(&config_dir.join("credentials"));
    let section = credentials.get(profile)?;

    let access_key_id = section.get("aws_access_key_id")?.clone();
    let secret_access_key = section.get("aws_secret_access_key")?.clone();
    if access_key_id.is_empty() || secret_access_key.is_empty() {
        return None;
    }

    Some(AwsCliProfile {
        access_key_id,
        secret_access_key,
        session_token: section.get("aws_session_token").cloned(),
    })
}

pub fn load_region_from_config(profile: &str) -> Option<String> {
    let config_dir = aws_config_dir()?;
    let config = parse_ini_file(&config_dir.join("config"));
    config
        .get(profile)
        .and_then(|section| section.get("region").cloned())
        .or_else(|| config.get("default").and_then(|s| s.get("region").cloned()))
}

pub fn resolve_aws_credentials(
    app_access_key: Option<String>,
    app_secret_key: Option<String>,
    app_session_token: Option<String>,
    app_region: Option<String>,
    app_profile: Option<String>,
) -> Result<ResolvedAwsCredentials, String> {
    if let (Some(access_key_id), Some(secret_access_key)) = (app_access_key, app_secret_key) {
        if !access_key_id.is_empty() && !secret_access_key.is_empty() {
            return Ok(ResolvedAwsCredentials {
                access_key_id,
                secret_access_key,
                session_token: app_session_token.filter(|t| !t.is_empty()),
                region: app_region
                    .filter(|r| !r.is_empty())
                    .or_else(get_region)
                    .unwrap_or_else(|| "us-east-1".into()),
                profile: app_profile.filter(|p| !p.is_empty()),
                source: "app".into(),
            });
        }
    }

    if let Some(profile) = app_profile.filter(|p| !p.is_empty()) {
        if let Some(cli) = load_profile_from_files(&profile) {
            return Ok(ResolvedAwsCredentials {
                access_key_id: cli.access_key_id,
                secret_access_key: cli.secret_access_key,
                session_token: cli.session_token,
                region: app_region
                    .filter(|r| !r.is_empty())
                    .or_else(|| load_region_from_config(&profile))
                    .or_else(get_region)
                    .unwrap_or_else(|| "us-east-1".into()),
                profile: Some(profile),
                source: "app".into(),
            });
        }
    }

    if let (Some(access_key_id), Some(secret_access_key)) = (get_access_key_id(), get_secret_access_key())
    {
        return Ok(ResolvedAwsCredentials {
            access_key_id,
            secret_access_key,
            session_token: get_session_token(),
            region: get_region().unwrap_or_else(|| "us-east-1".into()),
            profile: get_profile_name(),
            source: "env".into(),
        });
    }

    let profile = get_profile_name().unwrap_or_else(|| "default".into());
    if let Some(cli) = load_profile_from_files(&profile) {
        return Ok(ResolvedAwsCredentials {
            access_key_id: cli.access_key_id,
            secret_access_key: cli.secret_access_key,
            session_token: cli.session_token,
            region: load_region_from_config(&profile)
                .or_else(get_region)
                .unwrap_or_else(|| "us-east-1".into()),
            profile: Some(profile),
            source: "cli".into(),
        });
    }

    Err(
        "AWS credentials not found. Save keys in the app, set AWS env vars, or run `aws configure`."
            .into(),
    )
}

use std::sync::OnceLock;

const AWS_CLI_CANDIDATES: &[&str] = &[
    "aws",
    "/opt/homebrew/bin/aws",
    "/usr/local/bin/aws",
    "/usr/local/aws-cli/v2/current/bin/aws",
];

fn aws_binary_works(path: &str) -> bool {
    Command::new(path)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn shell_which_aws() -> Option<String> {
    let output = Command::new("zsh")
        .args(["-ilc", "command -v aws"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() || !aws_binary_works(&path) {
        None
    } else {
        Some(path)
    }
}

#[cfg(not(target_os = "macos"))]
fn shell_which_aws() -> Option<String> {
    None
}

pub fn aws_cli_missing_message() -> String {
    "AWS CLI not installed — run: brew install awscli".into()
}

pub fn find_aws_cli() -> Option<String> {
    for candidate in AWS_CLI_CANDIDATES {
        if aws_binary_works(candidate) {
            return Some((*candidate).into());
        }
    }
    shell_which_aws()
}

pub fn aws_cli_available() -> bool {
    static CACHE: OnceLock<bool> = OnceLock::new();
    *CACHE.get_or_init(|| find_aws_cli().is_some())
}

pub fn build_aws_command(creds: &ResolvedAwsCredentials) -> Result<Command, String> {
    let aws = find_aws_cli().ok_or_else(|| aws_cli_missing_message())?;
    let mut cmd = Command::new(aws);
    cmd.env("AWS_ACCESS_KEY_ID", &creds.access_key_id);
    cmd.env("AWS_SECRET_ACCESS_KEY", &creds.secret_access_key);
    cmd.env("AWS_REGION", &creds.region);
    cmd.env("AWS_DEFAULT_REGION", &creds.region);

    if let Some(token) = &creds.session_token {
        cmd.env("AWS_SESSION_TOKEN", token);
    } else {
        cmd.env_remove("AWS_SESSION_TOKEN");
    }

    // Explicit keys are always set above — profile would override the chain (SSO, etc.)
    cmd.env_remove("AWS_PROFILE");

    Ok(cmd)
}

pub fn validate_aws_credentials(creds: &ResolvedAwsCredentials) -> Result<String, String> {
    let output = build_aws_command(creds)?
        .args(["sts", "get-caller-identity", "--output", "json"])
        .output()
        .map_err(|e| format!("Failed to run AWS CLI: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("AWS credential check failed: {}", stderr.trim()));
    }

    let body: CallerIdentity = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Invalid STS response: {e}"))?;

    Ok(format!("Connected as {} ({})", body.arn, body.account))
}

pub fn widget_aws_sync_hint(message: &str) -> String {
    if message.contains("No such file or directory") || message.contains("AWS CLI not installed") {
        "Install AWS CLI: brew install awscli · then Sync Now".to_string()
    } else if message.contains("SignatureDoesNotMatch") {
        "AWS secret key mismatch · re-save keys in API Keys".to_string()
    } else if message.contains("Unable to locate credentials") || message.contains("No credentials") {
        "AWS not configured · API Keys or run aws configure".to_string()
    } else if message.contains("credential check failed") {
        "AWS credentials invalid · check API Keys".to_string()
    } else if message.len() > 72 {
        format!("{}…", &message[..69])
    } else {
        message.to_string()
    }
}

pub fn widget_sync_hint(provider: &str, message: &str) -> String {
    match provider {
        "OpenAI" => crate::adapters::widget_openai_sync_hint(message),
        "AWS Bedrock" => widget_aws_sync_hint(message),
        "Azure OpenAI" if message.len() > 72 => format!("Azure · {}…", &message[..64]),
        "Azure OpenAI" => format!("Azure · {message}"),
        _ if message.len() > 72 => format!("{}…", &message[..69]),
        _ => message.to_string(),
    }
}

#[derive(Debug, Deserialize)]
struct CallerIdentity {
    #[serde(rename = "Arn")]
    arn: String,
    #[serde(rename = "Account")]
    account: String,
}

#[derive(Debug, Deserialize)]
struct CostExplorerResponse {
    #[serde(rename = "ResultsByTime", default)]
    results_by_time: Vec<CostResultByTime>,
}

#[derive(Debug, Deserialize)]
struct CostResultByTime {
    #[serde(rename = "TimePeriod")]
    time_period: TimePeriod,
    #[serde(rename = "Groups")]
    groups: Option<Vec<CostGroup>>,
    #[serde(rename = "Total")]
    total: Option<HashMap<String, CostMetric>>,
}

#[derive(Debug, Deserialize)]
struct TimePeriod {
    #[serde(rename = "Start")]
    start: String,
}

#[derive(Debug, Deserialize)]
struct CostGroup {
    #[serde(rename = "Keys")]
    keys: Vec<String>,
    #[serde(rename = "Metrics")]
    metrics: HashMap<String, CostMetric>,
}

#[derive(Debug, Deserialize)]
struct CostMetric {
    #[serde(rename = "Amount")]
    amount: String,
}

pub fn fetch_bedrock_cost_events(
    creds: &ResolvedAwsCredentials,
    days: i64,
) -> Result<Vec<BedrockCostDay>, String> {
    if !aws_cli_available() {
        return Err(aws_cli_missing_message());
    }

    let end = chrono::Utc::now().date_naive();
    let start = end - chrono::Duration::days(days);
    let filter = r#"{"Dimensions":{"Key":"SERVICE","Values":["Amazon Bedrock"]}}"#;

    let output = build_aws_command(creds)?
        .args([
            "ce",
            "get-cost-and-usage",
            "--time-period",
            &format!("Start={},End={}", start, end),
            "--granularity",
            "DAILY",
            "--metrics",
            "UnblendedCost",
            "UsageQuantity",
            "--filter",
            filter,
            "--group-by",
            "Type=DIMENSION,Key=USAGE_TYPE",
            "--output",
            "json",
        ])
        .output()
        .map_err(|e| format!("Failed to run AWS Cost Explorer: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Cost Explorer request failed: {}. Ensure the IAM user/role has ce:GetCostAndUsage permission.",
            stderr.trim()
        ));
    }

    let response: CostExplorerResponse = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Invalid Cost Explorer response: {e}"))?;

    let mut days_map: HashMap<String, BedrockCostDay> = HashMap::new();

    for bucket in response.results_by_time {
        let date = bucket.time_period.start;
        if let Some(groups) = bucket.groups {
            for group in groups {
                let usage_type = group.keys.first().cloned().unwrap_or_else(|| "bedrock".into());
                let cost = group
                    .metrics
                    .get("UnblendedCost")
                    .and_then(|m| m.amount.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let quantity = group
                    .metrics
                    .get("UsageQuantity")
                    .and_then(|m| m.amount.parse::<f64>().ok())
                    .unwrap_or(0.0);

                let entry = days_map.entry(date.clone()).or_insert(BedrockCostDay {
                    date: date.clone(),
                    cost: 0.0,
                    usage_quantity: 0.0,
                    models: Vec::new(),
                });
                entry.cost += cost;
                entry.usage_quantity += quantity;
                if cost > 0.0 {
                    entry.models.push(BedrockModelDay {
                        usage_type,
                        cost,
                        usage_quantity: quantity,
                    });
                }
            }
        } else if let Some(total) = bucket.total {
            let cost = total
                .get("UnblendedCost")
                .and_then(|m| m.amount.parse::<f64>().ok())
                .unwrap_or(0.0);
            let quantity = total
                .get("UsageQuantity")
                .and_then(|m| m.amount.parse::<f64>().ok())
                .unwrap_or(0.0);
            days_map.insert(
                date.clone(),
                BedrockCostDay {
                    date,
                    cost,
                    usage_quantity: quantity,
                    models: Vec::new(),
                },
            );
        }
    }

    let mut days: Vec<BedrockCostDay> = days_map.into_values().collect();
    days.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(days)
}

#[derive(Debug, Clone)]
pub struct BedrockCostDay {
    pub date: String,
    pub cost: f64,
    pub usage_quantity: f64,
    pub models: Vec<BedrockModelDay>,
}

#[derive(Debug, Clone)]
pub struct BedrockModelDay {
    pub usage_type: String,
    pub cost: f64,
    pub usage_quantity: f64,
}
