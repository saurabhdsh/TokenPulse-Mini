/// macOS environment variable detection for API keys.
/// GUI apps don't inherit shell profile vars — we check process env,
/// launchctl, and login shell as fallbacks.
pub fn get_var(name: &str) -> Option<String> {
    if let Ok(value) = std::env::var(name) {
        let trimmed = value.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(value) = launchctl_getenv(name) {
            return Some(value);
        }
        if let Some(value) = shell_getenv(name) {
            return Some(value);
        }
    }

    None
}

pub fn get_openai_api_key() -> Option<String> {
    get_var("OPENAI_API_KEY")
}

pub fn get_openai_admin_key() -> Option<String> {
    get_var("OPENAI_ADMIN_KEY")
        .or_else(|| get_var("OPENAI_ADMIN_API_KEY"))
}

pub fn get_openai_org_id() -> Option<String> {
    get_var("OPENAI_ORG_ID")
}

/// Dashboard session bearer token for billing/credit endpoints (optional).
/// Extract from browser DevTools on platform.openai.com billing page.
pub fn get_openai_billing_token() -> Option<String> {
    get_var("OPENAI_BILLING_TOKEN")
        .or_else(|| get_var("OPENAI_SESSION_TOKEN"))
}

#[cfg(target_os = "macos")]
fn launchctl_getenv(name: &str) -> Option<String> {
    let output = std::process::Command::new("launchctl")
        .args(["getenv", name])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(target_os = "macos")]
fn shell_getenv(name: &str) -> Option<String> {
    let script = format!("print -r -- ${name}");
    let output = std::process::Command::new("zsh")
        .args(["-ilc", &script])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

pub fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        return "••••••••".to_string();
    }
    format!("{}…{}", &key[..4], &key[key.len() - 4..])
}

pub fn classify_openai_key_type(key: &str) -> &'static str {
    let key = key.trim();
    if key.starts_with("sk-admin-") {
        "admin"
    } else if key.starts_with("sk-proj-") {
        "project"
    } else if key.starts_with("sk-") {
        "legacy"
    } else {
        "unknown"
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnvVarProbe {
    pub process: bool,
    pub launchctl: bool,
    pub shell_profile: bool,
    pub key_type: Option<String>,
}

pub fn probe_var(name: &str) -> EnvVarProbe {
    probe_var_sources(name, true)
}

/// Fast probe — process environment only (no shell / launchctl subprocesses).
pub fn probe_var_fast(name: &str) -> EnvVarProbe {
    probe_var_sources(name, false)
}

fn probe_var_sources(name: &str, deep: bool) -> EnvVarProbe {
    let process_val = std::env::var(name).ok().filter(|v| !v.trim().is_empty());
    let launchctl_val = if deep {
        launchctl_getenv_raw(name)
    } else {
        None
    };
    let shell_val = if deep {
        shell_getenv_raw(name)
    } else {
        None
    };

    let key_type = process_val
        .as_deref()
        .or(launchctl_val.as_deref())
        .or(shell_val.as_deref())
        .map(|k| classify_openai_key_type(k).to_string());

    EnvVarProbe {
        process: process_val.is_some(),
        launchctl: launchctl_val.is_some(),
        shell_profile: shell_val.is_some(),
        key_type,
    }
}

#[cfg(target_os = "macos")]
fn launchctl_getenv_raw(name: &str) -> Option<String> {
    launchctl_getenv(name)
}

#[cfg(not(target_os = "macos"))]
fn launchctl_getenv_raw(_name: &str) -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn shell_getenv_raw(name: &str) -> Option<String> {
    shell_getenv(name)
}

#[cfg(not(target_os = "macos"))]
fn shell_getenv_raw(_name: &str) -> Option<String> {
    None
}
