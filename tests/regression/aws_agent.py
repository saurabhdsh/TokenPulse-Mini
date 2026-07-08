#!/usr/bin/env python3
"""
TokenPulse Mini — AWS Bedrock live regression agent.

Mirrors the Rust app's AWS Bedrock integration:
  - STS get-caller-identity          (credential validation)
  - bedrock-runtime converse         (optional hello prompt → real tokens)
  - Cost Explorer get-cost-and-usage (same filter as src-tauri/src/aws_config.rs)

The app widget syncs Bedrock via Cost Explorer (usage quantity → tokens, cost → $).
Cost Explorer can lag several hours; the hello prompt reports tokens immediately
from the Bedrock API response.

Standalone — no .env required. Edit AWS_CONFIG and TRIGGER_CONFIG below, then:

  pip install boto3
  python aws_agent.py --trigger
"""

from __future__ import annotations

import json
import sys
import time
from dataclasses import dataclass, field
from datetime import date, datetime, timedelta, timezone
from typing import Any

import boto3
from botocore.exceptions import BotoCoreError, ClientError

# ── AWS credentials — edit these (standalone, no .env) ───────────────────────
AWS_CONFIG: dict[str, str] = {
    "access_key_id": "AKIAxxxxxxxxxxxxxxxx",
    "secret_access_key": "your-secret-access-key",
    "session_token": "",  # optional, for temporary creds
    "region": "ap-south-1",
    "profile": "",  # only used if access_key_id / secret_access_key are empty
}

# ── Test options — edit these ─────────────────────────────────────────────────
TRIGGER_CONFIG: dict[str, str | int | bool] = {
    "enabled": True,  # send hello prompt to Bedrock (bills your account)
    "model": "anthropic.claude-3-haiku-20240307-v1:0",
    "prompt": "Hello from TokenPulse AWS regression test. Reply with one word only.",
    "max_tokens": 10,
    "usage_days": 30,
    "cost_poll_seconds": 90,
    "cost_poll_interval": 15,
}

# Fallback models if the default is not enabled in your account/region
MODEL_FALLBACKS = [
    "anthropic.claude-3-haiku-20240307-v1:0",
    "amazon.nova-micro-v1:0",
    "amazon.titan-text-lite-v1",
    "meta.llama3-8b-instruct-v1:0",
    "mistral.mistral-7b-instruct-v0:2",
]


@dataclass
class CheckResult:
    name: str
    passed: bool
    message: str
    details: dict[str, Any] = field(default_factory=dict)


@dataclass
class CostMetrics:
    days: int = 0
    total_cost_usd: float = 0.0
    total_usage_quantity: float = 0.0
    today_cost_usd: float = 0.0
    today_usage_quantity: float = 0.0
    top_usage_type: str = "—"
    top_usage_quantity: float = 0.0


def mask_key(key: str) -> str:
    if len(key) <= 8:
        return "••••••••"
    return f"{key[:4]}…{key[-4:]}"


def json_out_quiet() -> bool:
    return "--json" in sys.argv


def resolve_config() -> dict[str, str]:
    """Read credentials and region from AWS_CONFIG only."""
    return {
        "access_key_id": str(AWS_CONFIG.get("access_key_id", "")).strip(),
        "secret_access_key": str(AWS_CONFIG.get("secret_access_key", "")).strip(),
        "session_token": str(AWS_CONFIG.get("session_token", "")).strip(),
        "region": str(AWS_CONFIG.get("region", "us-east-1")).strip() or "us-east-1",
        "profile": str(AWS_CONFIG.get("profile", "")).strip(),
    }


def make_session(cfg: dict[str, str]) -> boto3.Session:
    if cfg["access_key_id"] and cfg["secret_access_key"]:
        kwargs: dict[str, Any] = {
            "aws_access_key_id": cfg["access_key_id"],
            "aws_secret_access_key": cfg["secret_access_key"],
            "region_name": cfg["region"],
        }
        if cfg["session_token"]:
            kwargs["aws_session_token"] = cfg["session_token"]
        return boto3.Session(**kwargs)

    if cfg["profile"]:
        return boto3.Session(profile_name=cfg["profile"], region_name=cfg["region"])

    return boto3.Session(region_name=cfg["region"])


def client_error_message(exc: ClientError) -> str:
    err = exc.response.get("Error", {})
    code = err.get("Code", "ClientError")
    msg = err.get("Message", str(exc))
    return f"{code}: {msg}"


def check_env(cfg: dict[str, str]) -> list[CheckResult]:
    results: list[CheckResult] = []
    has_keys = bool(cfg["access_key_id"] and cfg["secret_access_key"])
    has_profile = bool(cfg["profile"])

    if has_keys:
        results.append(
            CheckResult(
                "env:aws_keys",
                True,
                f"AWS_CONFIG keys set ({mask_key(cfg['access_key_id'])})",
            )
        )
    elif has_profile:
        results.append(
            CheckResult(
                "env:aws_profile",
                True,
                f"Using AWS profile {cfg['profile']!r} (no inline keys)",
            )
        )
    else:
        results.append(
            CheckResult(
                "env:aws_credentials",
                False,
                "Set access_key_id and secret_access_key in AWS_CONFIG at top of aws_agent.py",
            )
        )

    results.append(
        CheckResult("env:aws_region", True, cfg["region"], {"region": cfg["region"]})
    )
    return results


def test_sts(session: boto3.Session) -> CheckResult:
    try:
        sts = session.client("sts")
        identity = sts.get_caller_identity()
    except (ClientError, BotoCoreError) as exc:
        msg = client_error_message(exc) if isinstance(exc, ClientError) else str(exc)
        return CheckResult("aws:sts", False, msg)

    arn = identity.get("Arn", "?")
    account = identity.get("Account", "?")
    return CheckResult(
        "aws:sts",
        True,
        f"Connected as {arn} ({account})",
        {"arn": arn, "account": account},
    )


def list_on_demand_models(session: boto3.Session) -> tuple[list[str], CheckResult | None]:
    try:
        bedrock = session.client("bedrock")
        response = bedrock.list_foundation_models()
    except (ClientError, BotoCoreError) as exc:
        msg = client_error_message(exc) if isinstance(exc, ClientError) else str(exc)
        return [], CheckResult("aws:models", False, msg)

    models: list[str] = []
    for summary in response.get("modelSummaries", []):
        if summary.get("modelLifecycle", {}).get("status") == "LEGACY":
            continue
        inference_types = summary.get("inferenceTypesSupported") or []
        if "ON_DEMAND" in inference_types or not inference_types:
            model_id = summary.get("modelId")
            if model_id:
                models.append(model_id)

    if not models:
        return [], CheckResult(
            "aws:models",
            False,
            "No on-demand foundation models visible in this region/account",
        )

    return models, CheckResult(
        "aws:models",
        True,
        f"{len(models)} on-demand models available in region",
        {"model_count": len(models), "sample": models[:5]},
    )


def pick_model(requested: str, available: list[str]) -> str:
    if requested in available:
        return requested
    for candidate in MODEL_FALLBACKS:
        if candidate in available:
            return candidate
    return available[0] if available else requested


def send_hello_prompt(
    session: boto3.Session,
    model_id: str,
    prompt: str,
    max_tokens: int,
    available: list[str],
) -> CheckResult:
    chosen = pick_model(model_id, available) if available else model_id
    runtime = session.client("bedrock-runtime")

    try:
        response = runtime.converse(
            modelId=chosen,
            messages=[{"role": "user", "content": [{"text": prompt}]}],
            inferenceConfig={"maxTokens": max_tokens},
        )
    except (ClientError, BotoCoreError) as exc:
        msg = client_error_message(exc) if isinstance(exc, ClientError) else str(exc)
        hint = ""
        if "AccessDenied" in msg or "not authorized" in msg.lower():
            hint = " — enable model access in AWS Console → Bedrock → Model access"
        return CheckResult(
            "agent:hello",
            False,
            f"{msg}{hint}",
            {"model_requested": model_id, "model_used": chosen},
        )

    usage = response.get("usage") or {}
    output = response.get("output") or {}
    reply = ""
    for block in output.get("message", {}).get("content", []):
        if "text" in block:
            reply = block["text"].strip()
            break

    input_tokens = int(usage.get("inputTokens") or 0)
    output_tokens = int(usage.get("outputTokens") or 0)
    total_tokens = int(usage.get("totalTokens") or input_tokens + output_tokens)

    model_note = f" (requested {model_id})" if chosen != model_id else ""
    return CheckResult(
        "agent:hello",
        total_tokens > 0,
        (
            f"Invoked {chosen}{model_note} → {total_tokens} tokens "
            f"(input {input_tokens} + output {output_tokens}) · reply: {reply!r}"
        ),
        {
            "model_requested": model_id,
            "model_used": chosen,
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "total_tokens": total_tokens,
            "reply": reply,
        },
    )


def fetch_bedrock_cost_days(session: boto3.Session, days: int) -> tuple[list[dict], CheckResult | None]:
    """Same Cost Explorer query as src-tauri/src/aws_config.rs fetch_bedrock_cost_events."""
    end = date.today()
    start = end - timedelta(days=days)
    ce = session.client("ce")

    try:
        response = ce.get_cost_and_usage(
            TimePeriod={"Start": start.isoformat(), "End": end.isoformat()},
            Granularity="DAILY",
            Metrics=["UnblendedCost", "UsageQuantity"],
            Filter={"Dimensions": {"Key": "SERVICE", "Values": ["Amazon Bedrock"]}},
            GroupBy=[{"Type": "DIMENSION", "Key": "USAGE_TYPE"}],
        )
    except (ClientError, BotoCoreError) as exc:
        msg = client_error_message(exc) if isinstance(exc, ClientError) else str(exc)
        if "AccessDenied" in msg or "not authorized" in msg.lower():
            msg += " — IAM needs ce:GetCostAndUsage"
        return [], CheckResult("aws:cost_explorer", False, msg)

    return response.get("ResultsByTime", []), None


def aggregate_cost(results_by_time: list[dict]) -> CostMetrics:
    metrics = CostMetrics(days=len(results_by_time))
    today = date.today().isoformat()
    usage_by_type: dict[str, float] = {}

    for bucket in results_by_time:
        period_start = bucket.get("TimePeriod", {}).get("Start", "")
        is_today = period_start == today

        for group in bucket.get("Groups") or []:
            usage_type = (group.get("Keys") or ["bedrock"])[0]
            group_metrics = group.get("Metrics") or {}
            cost = float(group_metrics.get("UnblendedCost", {}).get("Amount", "0") or 0)
            quantity = float(group_metrics.get("UsageQuantity", {}).get("Amount", "0") or 0)

            metrics.total_cost_usd += cost
            metrics.total_usage_quantity += quantity
            usage_by_type[usage_type] = usage_by_type.get(usage_type, 0.0) + quantity

            if is_today:
                metrics.today_cost_usd += cost
                metrics.today_usage_quantity += quantity

        if not bucket.get("Groups"):
            total = bucket.get("Total") or {}
            cost = float(total.get("UnblendedCost", {}).get("Amount", "0") or 0)
            quantity = float(total.get("UsageQuantity", {}).get("Amount", "0") or 0)
            metrics.total_cost_usd += cost
            metrics.total_usage_quantity += quantity
            if is_today:
                metrics.today_cost_usd += cost
                metrics.today_usage_quantity += quantity

    if usage_by_type:
        top_type, top_qty = max(usage_by_type.items(), key=lambda item: item[1])
        metrics.top_usage_type = top_type
        metrics.top_usage_quantity = top_qty

    return metrics


def test_cost_explorer(session: boto3.Session, days: int) -> CheckResult:
    buckets, err = fetch_bedrock_cost_days(session, days)
    if err:
        return err

    metrics = aggregate_cost(buckets)
    # App maps usage_quantity → widget tokens (see adapters/bedrock.rs cost_day_event)
    today_tokens = int(round(metrics.today_usage_quantity))

    if metrics.total_cost_usd <= 0 and metrics.total_usage_quantity <= 0:
        return CheckResult(
            "aws:cost_explorer",
            True,
            (
                f"No Bedrock cost rows in last {days}d yet — normal before first CE sync "
                "(can take up to 24h after invoke)"
            ),
            {
                "days": metrics.days,
                "today_tokens_widget": today_tokens,
                "today_cost_usd": round(metrics.today_cost_usd, 6),
            },
        )

    return CheckResult(
        "aws:cost_explorer",
        True,
        (
            f"{metrics.days} day buckets · usage qty {metrics.total_usage_quantity:,.2f} · "
            f"${metrics.total_cost_usd:.6f} ({days}d)"
        ),
        {
            "days": metrics.days,
            "total_usage_quantity": round(metrics.total_usage_quantity, 4),
            "total_cost_usd": round(metrics.total_cost_usd, 6),
            "today_tokens_widget": today_tokens,
            "today_cost_usd": round(metrics.today_cost_usd, 6),
            "top_usage_type": metrics.top_usage_type,
            "top_usage_quantity": round(metrics.top_usage_quantity, 4),
            "burn_rate_per_hour_usd": round(
                metrics.today_cost_usd / max(1.0, datetime.now(timezone.utc).hour or 1), 6
            ),
        },
    )


def test_cost_capture_after_invoke(
    session: boto3.Session,
    before: CostMetrics,
    poll_seconds: int,
    poll_interval: int,
) -> CheckResult:
    """
    Poll Cost Explorer for today's usage quantity increase.
    CE often lags hours — failure here does not mean the invoke failed.
    """
    deadline = time.time() + poll_seconds
    baseline_qty = before.today_usage_quantity
    baseline_tokens = int(round(baseline_qty))

    while time.time() < deadline:
        buckets, err = fetch_bedrock_cost_days(session, days=2)
        if err:
            return err
        after = aggregate_cost(buckets)
        after_tokens = int(round(after.today_usage_quantity))
        if after.today_usage_quantity > baseline_qty + 1e-9:
            delta = after_tokens - baseline_tokens
            return CheckResult(
                "agent:cost_capture",
                True,
                (
                    f"Cost Explorer today usage {baseline_tokens:,} → {after_tokens:,} "
                    f"(+{delta:,} widget tokens) — app Sync Now should match"
                ),
                {
                    "before_today_tokens": baseline_tokens,
                    "after_today_tokens": after_tokens,
                    "delta_tokens": delta,
                },
            )

        remaining = int(deadline - time.time())
        if remaining > 0 and not json_out_quiet():
            print(
                f"      … polling Cost Explorer ({remaining}s left, "
                f"today widget tokens={after_tokens})"
            )
        time.sleep(poll_interval)

    buckets, _ = fetch_bedrock_cost_days(session, days=2)
    after = aggregate_cost(buckets or [])
    after_tokens = int(round(after.today_usage_quantity))
    return CheckResult(
        "agent:cost_capture",
        False,
        (
            f"Bedrock invoke succeeded but Cost Explorer still shows {after_tokens:,} "
            f"today tokens (was {baseline_tokens:,}) after {poll_seconds}s. "
            "This is normal — CE can lag hours. Use agent:hello token counts now; "
            "retry Sync Now in TokenPulse later today."
        ),
        {
            "before_today_tokens": baseline_tokens,
            "after_today_tokens": after_tokens,
            "poll_seconds": poll_seconds,
            "cost_explorer_lag_expected": True,
        },
    )


def print_results(results: list[CheckResult], json_out: bool) -> int:
    if json_out:
        print(
            json.dumps(
                [
                    {
                        "name": r.name,
                        "passed": r.passed,
                        "message": r.message,
                        "details": r.details,
                    }
                    for r in results
                ],
                indent=2,
            )
        )
    else:
        print("\n══ TokenPulse AWS Bedrock Regression Agent ══\n")
        for r in results:
            icon = "✓" if r.passed else "✗"
            print(f"  {icon} {r.name}: {r.message}")
            if r.name == "aws:cost_explorer":
                d = r.details
                print(
                    f"      today (widget): {d.get('today_tokens_widget', 0):,} tokens · "
                    f"${d.get('today_cost_usd', 0):.6f}"
                )
                if d.get("top_usage_type"):
                    print(
                        f"      top usage type: {d.get('top_usage_type')} "
                        f"({d.get('top_usage_quantity', 0):,.2f} qty)"
                    )
            if r.name == "agent:hello" and r.details:
                d = r.details
                print(f"      invoke tokens: {d.get('total_tokens', 0):,}")
            if r.name == "agent:cost_capture" and r.details.get("cost_explorer_lag_expected"):
                print("      note: Cost Explorer lag is expected — widget may update later")

        passed = sum(1 for r in results if r.passed)
        critical = [
            r
            for r in results
            if r.name in ("aws:sts", "aws:models", "agent:hello")
            and not r.passed
        ]
        print(f"\n  {passed}/{len(results)} checks passed")
        if critical:
            print("\n  CRITICAL FAILURES — fix credentials/model access before app sync.\n")
            return 1
        print(
            "\n  AWS regression OK — run TokenPulse v0.1.3+ → API Keys → AWS Bedrock → Sync Now.\n"
            "  Widget tokens come from Cost Explorer usage quantity (may lag after invoke).\n"
        )
        return 0

    critical_failed = any(
        not r.passed and r.name in ("aws:sts", "aws:models", "agent:hello")
        for r in results
    )
    return 1 if critical_failed else 0


def main() -> int:
    json_out = "--json" in sys.argv
    trigger_default = bool(TRIGGER_CONFIG.get("enabled", False))
    trigger = trigger_default
    if "--trigger" in sys.argv:
        trigger = True
    if "--no-trigger" in sys.argv:
        trigger = False

    days = int(TRIGGER_CONFIG.get("usage_days", 30))
    trigger_model = str(TRIGGER_CONFIG.get("model", MODEL_FALLBACKS[0])).strip()
    trigger_prompt = str(TRIGGER_CONFIG.get("prompt", "")).strip()
    trigger_max_tokens = int(TRIGGER_CONFIG.get("max_tokens", 10))
    poll_seconds = int(TRIGGER_CONFIG.get("cost_poll_seconds", 90))
    poll_interval = int(TRIGGER_CONFIG.get("cost_poll_interval", 15))

    cfg = resolve_config()
    results: list[CheckResult] = []
    results.extend(check_env(cfg))

    if not any(r.passed for r in results if r.name.startswith("env:aws")):
        return print_results(results, json_out)

    try:
        session = make_session(cfg)
    except Exception as exc:
        results.append(CheckResult("aws:session", False, str(exc)))
        return print_results(results, json_out)

    results.append(test_sts(session))

    available_models, models_result = list_on_demand_models(session)
    if models_result:
        results.append(models_result)
    if available_models:
        pass  # used for model pick in trigger

    cost_before: CostMetrics | None = None
    buckets_pre, ce_err = fetch_bedrock_cost_days(session, days=2)
    if ce_err:
        results.append(ce_err)
    else:
        cost_before = aggregate_cost(buckets_pre)
        results.append(test_cost_explorer(session, days))

    if trigger:
        if not json_out:
            print("\n  → Live agent test: invoking Bedrock converse…\n")
        results.append(
            send_hello_prompt(
                session,
                trigger_model,
                trigger_prompt,
                trigger_max_tokens,
                available_models,
            )
        )
        hello_ok = results[-1].passed
        if hello_ok and cost_before is not None:
            if not json_out:
                print(
                    f"\n  → Waiting up to {poll_seconds}s for Cost Explorer "
                    "(widget sync source)…\n"
                )
            results.append(
                test_cost_capture_after_invoke(
                    session, cost_before, poll_seconds, poll_interval
                )
            )
        elif hello_ok:
            results.append(
                CheckResult(
                    "agent:cost_capture",
                    False,
                    "Could not read Cost Explorer baseline before invoke",
                )
            )
    elif not trigger:
        results.append(
            CheckResult(
                "agent:hello",
                True,
                "Skipped — set TRIGGER_CONFIG['enabled']=True or pass --trigger",
            )
        )

    return print_results(results, json_out)


if __name__ == "__main__":
    sys.exit(main())
