use crate::aws_config::{
    self, credentials_file_exists, get_access_key_id, get_profile_name, get_region,
    get_secret_access_key, get_session_token, resolve_aws_credentials,
};
use crate::azure_config::{
    self, get_api_key as azure_env_api_key, get_api_version as azure_env_api_version,
    get_deployment_name as azure_env_deployment, get_endpoint as azure_env_endpoint,
    get_resource_group as azure_env_resource_group, get_subscription_id as azure_env_subscription,
    resolve_azure_openai_credentials, ResolvedAzureOpenAICredentials,
};
use crate::db::Database;
use crate::env;
use crate::models::{
    AwsCredentialsStatus, CredentialFieldStatus, OpenAICredentialsStatus, ResolvedAwsCredentials,
    ResolvedOpenAICredentials,
};

const SECRET_ADMIN: &str = "openai_admin_key";
const SECRET_BILLING: &str = "openai_billing_token";
const SECRET_ORG: &str = "openai_org_id";

const AWS_ACCESS_KEY: &str = "aws_access_key_id";
const AWS_SECRET_KEY: &str = "aws_secret_access_key";
const AWS_SESSION: &str = "aws_session_token";
const AWS_REGION: &str = "aws_region";
const AWS_PROFILE: &str = "aws_profile";

const AZURE_ENDPOINT: &str = "azure_openai_endpoint";
const AZURE_API_VERSION: &str = "azure_openai_api_version";
const AZURE_DEPLOYMENT: &str = "azure_openai_deployment";
const AZURE_SUBSCRIPTION: &str = "azure_subscription_id";
const AZURE_RESOURCE_GROUP: &str = "azure_openai_resource_group";

pub fn resolve_openai_credentials(db: &Database) -> Result<ResolvedOpenAICredentials, String> {
    let provider = db.get_provider_by_name("OpenAI").map_err(|e| e.to_string())?;

    let api_key = resolve_api_key(db, &provider)?;
    let admin_key = resolve_secret(db, SECRET_ADMIN, env::get_openai_admin_key)?;
    let billing_token = resolve_secret(db, SECRET_BILLING, env::get_openai_billing_token)?;
    let org_id = resolve_secret(db, SECRET_ORG, env::get_openai_org_id)?;

    Ok(ResolvedOpenAICredentials {
        api_key,
        admin_key,
        billing_token,
        org_id,
    })
}

pub fn get_openai_credentials_status(db: &Database) -> Result<OpenAICredentialsStatus, String> {
    let provider = db.get_provider_by_name("OpenAI").map_err(|e| e.to_string())?;
    let app_admin = db.get_secret(SECRET_ADMIN).map_err(|e| e.to_string())?;
    let app_billing = db.get_secret(SECRET_BILLING).map_err(|e| e.to_string())?;
    let app_org = db.get_secret(SECRET_ORG).map_err(|e| e.to_string())?;

    let env_api = env::get_openai_api_key();
    let env_admin = env::get_openai_admin_key();
    let env_billing = env::get_openai_billing_token();
    let env_org = env::get_openai_org_id();

    let api_from_app = provider
        .key_source
        .as_deref()
        .is_some_and(|s| s == "app" || s == "manual")
        && provider
            .api_key
            .as_ref()
            .map(|k| !k.is_empty())
            .unwrap_or(false);

    let api_preview = db
        .get_provider_api_key("OpenAI")
        .ok()
        .flatten()
        .or(env_api.clone())
        .map(|k| env::mask_key(&k));

    let api_key = field_status(
        "api_key",
        "API Key",
        "Required for OpenAI connection",
        "OPENAI_API_KEY",
        api_from_app,
        env_api.is_some(),
        api_preview,
    );

    let admin_key = field_status(
        "admin_key",
        "Admin Key",
        "Required for usage sync — needs api.usage.read scope (not your regular API key)",
        "OPENAI_ADMIN_KEY",
        app_admin.is_some(),
        env_admin.is_some(),
        app_admin
            .as_deref()
            .or(env_admin.as_deref())
            .map(env::mask_key),
    );

    let billing_token = field_status(
        "billing_token",
        "Billing Token",
        "Prepaid credit balance (sess-… from browser)",
        "OPENAI_BILLING_TOKEN",
        app_billing.is_some(),
        env_billing.is_some(),
        app_billing
            .as_deref()
            .or(env_billing.as_deref())
            .map(env::mask_key),
    );

    let org_id = field_status(
        "org_id",
        "Organization ID",
        "Optional — multi-org accounts",
        "OPENAI_ORG_ID",
        app_org.is_some(),
        env_org.is_some(),
        app_org
            .as_deref()
            .or(env_org.as_deref())
            .map(|id| {
                if id.len() <= 12 {
                    id.to_string()
                } else {
                    format!("{}…{}", &id[..6], &id[id.len() - 4..])
                }
            }),
    );

    let resolved_admin = app_admin.as_deref().or(env_admin.as_deref());
    let active_admin = crate::models::ActiveCredentialSummary {
        configured: resolved_admin.is_some(),
        source: if app_admin.is_some() {
            "app".into()
        } else if env_admin.is_some() {
            "env".into()
        } else {
            "none".into()
        },
        key_type: resolved_admin.map(|k| env::classify_openai_key_type(k).to_string()),
        preview: resolved_admin.map(env::mask_key),
    };

    Ok(OpenAICredentialsStatus {
        api_key,
        admin_key,
        billing_token,
        org_id,
        active_admin,
    })
}

pub fn update_openai_credentials(
    db: &Database,
    api_key: Option<&str>,
    admin_key: Option<&str>,
    billing_token: Option<&str>,
    org_id: Option<&str>,
) -> Result<(), String> {
    let mut updated = false;

    if let Some(value) = api_key {
        if value.is_empty() {
            db.clear_provider_api_key("OpenAI").map_err(|e| e.to_string())?;
        } else {
            db.set_provider_key("OpenAI", value, true, "app")
                .map_err(|e| e.to_string())?;
        }
        updated = true;
    }

    if let Some(value) = admin_key {
        update_secret(db, SECRET_ADMIN, value)?;
        updated = true;
    }
    if let Some(value) = billing_token {
        update_secret(db, SECRET_BILLING, value)?;
        updated = true;
    }
    if let Some(value) = org_id {
        update_secret(db, SECRET_ORG, value)?;
        updated = true;
    }

    if !updated {
        return Err("No credential fields provided to save.".into());
    }

    Ok(())
}

pub fn resolve_bedrock_credentials(db: &Database) -> Result<ResolvedAwsCredentials, String> {
    let app_access = db.get_secret(AWS_ACCESS_KEY).map_err(|e| e.to_string())?;
    let app_secret = db.get_secret(AWS_SECRET_KEY).map_err(|e| e.to_string())?;
    let app_session = db.get_secret(AWS_SESSION).map_err(|e| e.to_string())?;
    let app_region = db.get_secret(AWS_REGION).map_err(|e| e.to_string())?;
    let app_profile = db.get_secret(AWS_PROFILE).map_err(|e| e.to_string())?;

    resolve_aws_credentials(app_access, app_secret, app_session, app_region, app_profile)
}

pub fn get_aws_credentials_status(db: &Database) -> Result<AwsCredentialsStatus, String> {
    let app_access = db.get_secret(AWS_ACCESS_KEY).map_err(|e| e.to_string())?;
    let app_secret = db.get_secret(AWS_SECRET_KEY).map_err(|e| e.to_string())?;
    let app_session = db.get_secret(AWS_SESSION).map_err(|e| e.to_string())?;
    let app_region = db.get_secret(AWS_REGION).map_err(|e| e.to_string())?;
    let app_profile = db.get_secret(AWS_PROFILE).map_err(|e| e.to_string())?;

    let env_access = get_access_key_id();
    let env_secret = get_secret_access_key();
    let env_session = get_session_token();
    let env_region = get_region();
    let env_profile = get_profile_name();
    let cli_configured = credentials_file_exists();

    let cli_default_profile = aws_config::load_profile_from_files(
        &env_profile.clone().unwrap_or_else(|| "default".into()),
    );

    let access_key = aws_field_status(
        "access_key_id",
        "Access Key ID",
        "IAM access key for Bedrock / Cost Explorer",
        "AWS_ACCESS_KEY_ID",
        app_access.is_some(),
        env_access.is_some(),
        cli_default_profile.as_ref().map(|p| env::mask_key(&p.access_key_id)),
        app_access
            .as_deref()
            .or(env_access.as_deref())
            .map(env::mask_key),
    );

    let secret_access_key = aws_field_status(
        "secret_access_key",
        "Secret Access Key",
        "Paired secret for the access key",
        "AWS_SECRET_ACCESS_KEY",
        app_secret.is_some(),
        env_secret.is_some(),
        cli_default_profile.as_ref().map(|_| "••••••••".into()),
        Some("••••••••".into()).filter(|_| app_secret.is_some() || env_secret.is_some() || cli_default_profile.is_some()),
    );

    let session_token = aws_field_status(
        "session_token",
        "Session Token",
        "Optional — required for temporary / SSO credentials",
        "AWS_SESSION_TOKEN",
        app_session.is_some(),
        env_session.is_some(),
        None,
        app_session.as_deref().or(env_session.as_deref()).map(env::mask_key),
    );

    let region = aws_field_status(
        "region",
        "Region",
        "Bedrock region (e.g. us-east-1)",
        "AWS_REGION",
        app_region.is_some(),
        env_region.is_some(),
        aws_config::load_region_from_config("default").as_deref().map(|r| r.to_string()),
        app_region.or(env_region),
    );

    let profile = aws_field_status(
        "profile",
        "CLI Profile",
        "Optional — reads ~/.aws/credentials when keys not saved",
        "AWS_PROFILE",
        app_profile.is_some(),
        env_profile.is_some(),
        env_profile.clone().or_else(|| if cli_configured { Some("default".into()) } else { None }),
        app_profile.or(env_profile),
    );

    Ok(AwsCredentialsStatus {
        access_key_id: access_key,
        secret_access_key,
        session_token,
        region,
        profile,
        aws_cli_configured: cli_configured,
        aws_cli_available: aws_config::aws_cli_available(),
    })
}

pub fn update_aws_credentials(
    db: &Database,
    access_key_id: Option<&str>,
    secret_access_key: Option<&str>,
    session_token: Option<&str>,
    region: Option<&str>,
    profile: Option<&str>,
) -> Result<(), String> {
    if let Some(value) = access_key_id {
        update_secret(db, AWS_ACCESS_KEY, value)?;
    }
    if let Some(value) = secret_access_key {
        update_secret(db, AWS_SECRET_KEY, value)?;
    }
    if let Some(value) = session_token {
        update_secret(db, AWS_SESSION, value)?;
    }
    if let Some(value) = region {
        update_secret(db, AWS_REGION, value)?;
    }
    if let Some(value) = profile {
        update_secret(db, AWS_PROFILE, value)?;
    }
    Ok(())
}

pub fn resolve_azure_credentials(db: &Database) -> Result<ResolvedAzureOpenAICredentials, String> {
    let provider = db
        .get_provider_by_name("Azure OpenAI")
        .map_err(|e| e.to_string())?;

    let app_endpoint = db.get_secret(AZURE_ENDPOINT).map_err(|e| e.to_string())?;
    let app_api_version = db.get_secret(AZURE_API_VERSION).map_err(|e| e.to_string())?;
    let app_deployment = db.get_secret(AZURE_DEPLOYMENT).map_err(|e| e.to_string())?;
    let app_subscription = db.get_secret(AZURE_SUBSCRIPTION).map_err(|e| e.to_string())?;
    let app_resource_group = db
        .get_secret(AZURE_RESOURCE_GROUP)
        .map_err(|e| e.to_string())?;

    let app_api_key = resolve_azure_api_key(db, &provider);

    resolve_azure_openai_credentials(
        app_api_key,
        app_endpoint,
        app_api_version,
        app_deployment,
        app_subscription,
        app_resource_group,
    )
}

pub fn get_azure_credentials_status(db: &Database) -> Result<crate::models::AzureCredentialsStatus, String> {
    let provider = db
        .get_provider_by_name("Azure OpenAI")
        .map_err(|e| e.to_string())?;

    let app_endpoint = db.get_secret(AZURE_ENDPOINT).map_err(|e| e.to_string())?;
    let app_api_version = db.get_secret(AZURE_API_VERSION).map_err(|e| e.to_string())?;
    let app_deployment = db.get_secret(AZURE_DEPLOYMENT).map_err(|e| e.to_string())?;
    let app_subscription = db.get_secret(AZURE_SUBSCRIPTION).map_err(|e| e.to_string())?;
    let app_resource_group = db
        .get_secret(AZURE_RESOURCE_GROUP)
        .map_err(|e| e.to_string())?;

    let env_endpoint = azure_env_endpoint();
    let env_api_key = azure_env_api_key();
    let env_api_version = azure_env_api_version();
    let env_deployment = azure_env_deployment();
    let env_subscription = azure_env_subscription();
    let env_resource_group = azure_env_resource_group();

    let api_from_app = provider
        .key_source
        .as_deref()
        .is_some_and(|s| s == "app" || s == "manual")
        && provider
            .api_key
            .as_ref()
            .map(|k| !k.is_empty())
            .unwrap_or(false);

    let api_preview = db
        .get_provider_api_key("Azure OpenAI")
        .ok()
        .flatten()
        .or(env_api_key.clone())
        .map(|k| env::mask_key(&k));

    let endpoint_preview = app_endpoint
        .clone()
        .or(env_endpoint.clone())
        .map(|e| {
            if e.len() <= 40 {
                e
            } else {
                format!("{}…", &e[..36])
            }
        });

    Ok(crate::models::AzureCredentialsStatus {
        api_key: field_status(
            "api_key",
            "API Key",
            "Azure OpenAI resource key from Azure AI Foundry",
            "AZURE_OPENAI_API_KEY",
            api_from_app,
            env_api_key.is_some(),
            api_preview,
        ),
        endpoint: field_status(
            "endpoint",
            "Endpoint",
            "Resource endpoint, e.g. https://myresource.openai.azure.com",
            "AZURE_OPENAI_ENDPOINT",
            app_endpoint.is_some(),
            env_endpoint.is_some(),
            endpoint_preview,
        ),
        api_version: field_status(
            "api_version",
            "API Version",
            "Azure OpenAI REST API version",
            "AZURE_OPENAI_API_VERSION",
            app_api_version.is_some(),
            env_api_version.is_some(),
            app_api_version
                .or(env_api_version)
                .or_else(|| Some(azure_config::DEFAULT_API_VERSION.to_string())),
        ),
        deployment_name: field_status(
            "deployment_name",
            "Deployment Name",
            "Optional — limit sync to one deployment",
            "AZURE_OPENAI_DEPLOYMENT_NAME",
            app_deployment.is_some(),
            env_deployment.is_some(),
            app_deployment.or(env_deployment),
        ),
        subscription_id: field_status(
            "subscription_id",
            "Subscription ID",
            "Optional — required for Azure Monitor token metrics via Azure CLI",
            "AZURE_SUBSCRIPTION_ID",
            app_subscription.is_some(),
            env_subscription.is_some(),
            app_subscription.or(env_subscription).map(|id| {
                if id.len() <= 12 {
                    id
                } else {
                    format!("{}…{}", &id[..6], &id[id.len() - 4..])
                }
            }),
        ),
        resource_group: field_status(
            "resource_group",
            "Resource Group",
            "Optional — Azure resource group for the OpenAI account",
            "AZURE_OPENAI_RESOURCE_GROUP",
            app_resource_group.is_some(),
            env_resource_group.is_some(),
            app_resource_group.or(env_resource_group),
        ),
        azure_cli_available: azure_config::az_cli_available(),
    })
}

pub fn update_azure_credentials(
    db: &Database,
    api_key: Option<&str>,
    endpoint: Option<&str>,
    api_version: Option<&str>,
    deployment_name: Option<&str>,
    subscription_id: Option<&str>,
    resource_group: Option<&str>,
) -> Result<(), String> {
    let mut updated = false;

    if let Some(value) = api_key {
        if value.is_empty() {
            db.clear_provider_api_key("Azure OpenAI")
                .map_err(|e| e.to_string())?;
        } else {
            db.set_provider_key("Azure OpenAI", value, true, "app")
                .map_err(|e| e.to_string())?;
        }
        updated = true;
    }

    if let Some(value) = endpoint {
        update_secret(db, AZURE_ENDPOINT, value)?;
        updated = true;
    }
    if let Some(value) = api_version {
        update_secret(db, AZURE_API_VERSION, value)?;
        updated = true;
    }
    if let Some(value) = deployment_name {
        update_secret(db, AZURE_DEPLOYMENT, value)?;
        updated = true;
    }
    if let Some(value) = subscription_id {
        update_secret(db, AZURE_SUBSCRIPTION, value)?;
        updated = true;
    }
    if let Some(value) = resource_group {
        update_secret(db, AZURE_RESOURCE_GROUP, value)?;
        updated = true;
    }

    if !updated {
        return Err("No Azure OpenAI credential fields provided to save.".into());
    }

    Ok(())
}

fn resolve_azure_api_key(
    db: &Database,
    provider: &crate::models::Provider,
) -> Option<String> {
    let db_key = db
        .get_provider_api_key("Azure OpenAI")
        .ok()
        .flatten()
        .filter(|k| !k.is_empty());

    let app_saved = provider
        .key_source
        .as_deref()
        .is_some_and(|s| s == "app" || s == "manual");

    if app_saved {
        return db_key;
    }

    if let Some(env_key) = azure_env_api_key() {
        return Some(env_key);
    }

    db_key
}

fn aws_field_status(
    field: &str,
    label: &str,
    hint: &str,
    env_var: &str,
    app_set: bool,
    env_set: bool,
    cli_preview: Option<String>,
    preview: Option<String>,
) -> CredentialFieldStatus {
    let (is_configured, source) = if app_set {
        (true, "app".into())
    } else if env_set {
        (true, "env".into())
    } else if cli_preview.is_some() || preview.is_some() {
        (true, "cli".into())
    } else {
        (false, "none".into())
    };

    CredentialFieldStatus {
        field: field.into(),
        label: label.into(),
        hint: hint.into(),
        env_var: env_var.into(),
        is_configured,
        source,
        preview: preview.or(cli_preview),
    }
}

fn resolve_api_key(
    db: &Database,
    provider: &crate::models::Provider,
) -> Result<Option<String>, String> {
    let db_key = db
        .get_provider_api_key("OpenAI")
        .map_err(|e| e.to_string())?
        .filter(|k| !k.is_empty());

    let app_saved = provider
        .key_source
        .as_deref()
        .is_some_and(|s| s == "app" || s == "manual");

    if app_saved {
        return Ok(db_key);
    }

    if let Some(env_key) = env::get_openai_api_key() {
        return Ok(Some(env_key));
    }

    Ok(db_key)
}

fn resolve_secret<F>(db: &Database, key: &str, env_fn: F) -> Result<Option<String>, String>
where
    F: FnOnce() -> Option<String>,
{
    if let Some(value) = db.get_secret(key).map_err(|e| e.to_string())? {
        if !value.is_empty() {
            return Ok(Some(value));
        }
    }
    Ok(env_fn())
}

fn update_secret(db: &Database, key: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        db.delete_secret(key).map_err(|e| e.to_string())?;
    } else {
        db.set_secret(key, value).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn field_status(
    field: &str,
    label: &str,
    hint: &str,
    env_var: &str,
    app_set: bool,
    env_set: bool,
    preview: Option<String>,
) -> CredentialFieldStatus {
    let (is_configured, source) = if app_set {
        (true, "app".into())
    } else if env_set {
        (true, "env".into())
    } else {
        (false, "none".into())
    };

    CredentialFieldStatus {
        field: field.into(),
        label: label.into(),
        hint: hint.into(),
        env_var: env_var.into(),
        is_configured,
        source,
        preview,
    }
}
