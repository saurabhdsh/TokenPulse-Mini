#!/usr/bin/env python3
"""
TokenPulse Mini — OpenAI live API regression agent.

Mirrors the Rust app's OpenAI integration:
  - GET /v1/models                    (API key validation)
  - POST /v1/chat/completions         (optional hello prompt → real tokens)
  - GET /v1/organization/usage/completions  (Admin key + api.usage.read)
  - Billing endpoints (optional)

Usage:
  cd tests/regression
  cp .env.example .env   # fill keys
  pip install -r requirements.txt
  python openai_agent.py
  python openai_agent.py --trigger   # send hello prompt + verify today tokens
"""

from __future__ import annotations

import json
import os
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

import requests
from dotenv import load_dotenv

SCRIPT_DIR = Path(__file__).resolve().parent
load_dotenv(SCRIPT_DIR / ".env")

BASE = "https://api.openai.com"

# Matches src-tauri/src/adapters/openai.rs default_openai_pricing()
DEFAULT_PRICING: dict[str, tuple[float, float]] = {
    "gpt-4o": (2.5, 10.0),
    "gpt-4o-mini": (0.15, 0.6),
    "gpt-4o-2024-08-06": (2.5, 10.0),
    "gpt-4o-mini-2024-07-18": (0.15, 0.6),
    "o1-preview": (15.0, 60.0),
    "o1-mini": (3.0, 12.0),
    "o3-mini": (1.1, 4.4),
    "gpt-4.1": (2.0, 8.0),
    "gpt-4.1-mini": (0.4, 1.6),
    "gpt-4.1-nano": (0.1, 0.4),
}


@dataclass
class CheckResult:
    name: str
    passed: bool
    message: str
    details: dict[str, Any] = field(default_factory=dict)


@dataclass
class UsageMetrics:
    buckets: int = 0
    events: int = 0
    total_tokens: int = 0
    prompt_tokens: int = 0
    completion_tokens: int = 0
    estimated_cost_usd: float = 0.0
    top_model: str = "—"
    top_model_tokens: int = 0
    today_tokens: int = 0
    today_cost_usd: float = 0.0


def mask_key(key: str) -> str:
    if len(key) <= 8:
        return "••••••••"
    return f"{key[:4]}…{key[-4:]}"


def classify_key(key: str) -> str:
    if key.startswith("sk-admin-"):
        return "admin"
    if key.startswith("sk-proj-"):
        return "project"
    if key.startswith("sk-"):
        return "api"
    return "unknown"


def headers(api_key: str, org_id: str | None = None) -> dict[str, str]:
    h = {"Authorization": f"Bearer {api_key}"}
    if org_id:
        h["OpenAI-Organization"] = org_id
    return h


def calculate_cost(
    prompt_tokens: int, completion_tokens: int, model: str
) -> tuple[float, float, float]:
    input_ppm, output_ppm = DEFAULT_PRICING.get(model, (2.5, 10.0))
    input_cost = (prompt_tokens / 1_000_000) * input_ppm
    output_cost = (completion_tokens / 1_000_000) * output_ppm
    return input_cost, output_cost, input_cost + output_cost


def parse_error(response: requests.Response) -> str:
    try:
        body = response.json()
        return body.get("error", {}).get("message", response.text[:200])
    except Exception:
        return response.text[:200] or f"HTTP {response.status_code}"


def check_env() -> list[CheckResult]:
    results: list[CheckResult] = []
    api = os.getenv("OPENAI_API_KEY", "").strip()
    admin = os.getenv("OPENAI_ADMIN_KEY", "").strip() or os.getenv(
        "OPENAI_ADMIN_API_KEY", ""
    ).strip()
    org = os.getenv("OPENAI_ORG_ID", "").strip() or None

    if not api:
        results.append(
            CheckResult("env:OPENAI_API_KEY", False, "Missing OPENAI_API_KEY in .env")
        )
    else:
        results.append(
            CheckResult(
                "env:OPENAI_API_KEY",
                True,
                f"Set ({mask_key(api)}, type={classify_key(api)})",
            )
        )

    if not admin:
        results.append(
            CheckResult(
                "env:OPENAI_ADMIN_KEY",
                False,
                "Missing OPENAI_ADMIN_KEY — usage sync will fail in app",
            )
        )
    else:
        kt = classify_key(admin)
        results.append(
            CheckResult(
                "env:OPENAI_ADMIN_KEY",
                kt == "admin",
                f"Set ({mask_key(admin)}, type={kt})"
                + (" ✓" if kt == "admin" else " — need sk-admin-… key"),
            )
        )

    if org:
        results.append(CheckResult("env:OPENAI_ORG_ID", True, org))
    else:
        results.append(
            CheckResult("env:OPENAI_ORG_ID", True, "Optional — not set", {})
        )

    return results


def test_api_key(api_key: str, org_id: str | None) -> CheckResult:
    try:
        r = requests.get(
            f"{BASE}/v1/models",
            headers=headers(api_key, org_id),
            timeout=20,
        )
    except requests.RequestException as e:
        return CheckResult("api:models", False, str(e))

    if r.status_code == 200:
        data = r.json().get("data", [])
        return CheckResult(
            "api:models",
            True,
            f"API key valid — {len(data)} models visible",
            {"model_count": len(data)},
        )
    return CheckResult("api:models", False, parse_error(r), {"status": r.status_code})


def fetch_usage_buckets(
    admin_key: str, org_id: str | None, days: int
) -> tuple[list[dict], CheckResult | None]:
    start_time = int((datetime.now(timezone.utc) - timedelta(days=days)).timestamp())
    all_buckets: list[dict] = []
    page: str | None = None

    while True:
        params: dict[str, str | int] = {
            "start_time": start_time,
            "bucket_width": "1d",
            "group_by": "model",
            "limit": 31,
        }
        if page:
            params["page"] = page

        try:
            r = requests.get(
                f"{BASE}/v1/organization/usage/completions",
                headers=headers(admin_key, org_id),
                params=params,
                timeout=30,
            )
        except requests.RequestException as e:
            return [], CheckResult("api:usage", False, str(e))

        if r.status_code != 200:
            msg = parse_error(r)
            if "api.usage.read" in msg:
                msg = (
                    "Admin Key missing api.usage.read scope — create new Admin key "
                    "at platform.openai.com → Admin keys"
                )
            return [], CheckResult(
                "api:usage",
                False,
                msg,
                {"status": r.status_code},
            )

        body = r.json()
        all_buckets.extend(body.get("data", []))
        if body.get("has_more") and body.get("next_page"):
            page = body["next_page"]
        else:
            break

    return all_buckets, None


def today_start_local() -> datetime:
    """Match app: local midnight (src-tauri/db get_today_start)."""
    local = datetime.now().astimezone()
    return local.replace(hour=0, minute=0, second=0, microsecond=0)


def get_today_metrics(admin_key: str, org_id: str | None) -> UsageMetrics | None:
    buckets, err = fetch_usage_buckets(admin_key, org_id, days=2)
    if err:
        return None
    return aggregate_usage(buckets)


def send_hello_prompt(
    api_key: str,
    org_id: str | None,
    model: str,
    prompt: str,
    max_tokens: int,
) -> CheckResult:
    """Tiny chat completion — triggers billable tokens like any agent app."""
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens,
    }
    hdrs = {**headers(api_key, org_id), "Content-Type": "application/json"}
    try:
        r = requests.post(
            f"{BASE}/v1/chat/completions",
            headers=hdrs,
            json=payload,
            timeout=60,
        )
    except requests.RequestException as e:
        return CheckResult("agent:hello", False, str(e))

    if r.status_code != 200:
        return CheckResult("agent:hello", False, parse_error(r), {"status": r.status_code})

    body = r.json()
    usage = body.get("usage") or {}
    reply = (
        body.get("choices", [{}])[0]
        .get("message", {})
        .get("content", "")
        .strip()
    )
    prompt_t = int(usage.get("prompt_tokens") or 0)
    completion_t = int(usage.get("completion_tokens") or 0)
    total_t = int(usage.get("total_tokens") or prompt_t + completion_t)

    return CheckResult(
        "agent:hello",
        total_t > 0,
        f"Sent hello → {total_t} tokens (prompt {prompt_t} + completion {completion_t}) · reply: {reply!r}",
        {
            "model": model,
            "prompt_tokens": prompt_t,
            "completion_tokens": completion_t,
            "total_tokens": total_t,
            "reply": reply,
        },
    )


def test_usage_capture_after_prompt(
    admin_key: str,
    org_id: str | None,
    before: UsageMetrics,
    poll_seconds: int,
    poll_interval: int,
) -> CheckResult:
    """
    Poll organization usage API until today's token count increases.
    OpenAI usage buckets can lag by 1–3 minutes.
    """
    deadline = time.time() + poll_seconds
    baseline = before.today_tokens
    target_min = baseline + 1

    while time.time() < deadline:
        after = get_today_metrics(admin_key, org_id)
        if after and after.today_tokens >= target_min:
            delta = after.today_tokens - baseline
            return CheckResult(
                "agent:usage_capture",
                True,
                (
                    f"Today tokens {baseline:,} → {after.today_tokens:,} "
                    f"(+{delta:,}) — usage API picked up the hello prompt"
                ),
                {
                    "before_today_tokens": baseline,
                    "after_today_tokens": after.today_tokens,
                    "delta_tokens": delta,
                },
            )
        remaining = int(deadline - time.time())
        if remaining > 0 and not json_out_quiet():
            print(f"      … polling usage API ({remaining}s left, today={after.today_tokens if after else '?'} tokens)")
        time.sleep(poll_interval)

    after = get_today_metrics(admin_key, org_id)
    after_today = after.today_tokens if after else baseline
    return CheckResult(
        "agent:usage_capture",
        False,
        (
            f"Hello prompt sent but today's usage still {after_today:,} tokens "
            f"(was {baseline:,}) after {poll_seconds}s — OpenAI usage API may lag; "
            "retry Sync Now in the app in a few minutes"
        ),
        {
            "before_today_tokens": baseline,
            "after_today_tokens": after_today,
            "poll_seconds": poll_seconds,
        },
    )


def json_out_quiet() -> bool:
    return "--json" in sys.argv


def aggregate_usage(buckets: list[dict]) -> UsageMetrics:
    metrics = UsageMetrics(buckets=len(buckets))
    model_tokens: dict[str, int] = {}
    today_start = today_start_local()

    for bucket in buckets:
        start_time = bucket.get("start_time", 0)
        bucket_dt = datetime.fromtimestamp(start_time, tz=timezone.utc).astimezone()
        is_today = bucket_dt >= today_start

        for result in bucket.get("results", []):
            prompt = int(result.get("input_tokens") or 0)
            completion = int(result.get("output_tokens") or 0)
            if prompt == 0 and completion == 0:
                continue

            model = (result.get("model") or "unknown").strip() or "unknown"
            _, _, total_cost = calculate_cost(prompt, completion, model)

            metrics.events += 1
            metrics.prompt_tokens += prompt
            metrics.completion_tokens += completion
            metrics.total_tokens += prompt + completion
            metrics.estimated_cost_usd += total_cost
            model_tokens[model] = model_tokens.get(model, 0) + prompt + completion

            if is_today:
                metrics.today_tokens += prompt + completion
                metrics.today_cost_usd += total_cost

    if model_tokens:
        top = max(model_tokens.items(), key=lambda x: x[1])
        metrics.top_model = top[0]
        metrics.top_model_tokens = top[1]

    return metrics


def test_usage(admin_key: str, org_id: str | None, days: int) -> CheckResult:
    buckets, err = fetch_usage_buckets(admin_key, org_id, days)
    if err:
        return err

    if not buckets:
        return CheckResult(
            "api:usage",
            False,
            f"No usage buckets in last {days} days",
        )

    metrics = aggregate_usage(buckets)
    if metrics.events == 0:
        return CheckResult(
            "api:usage",
            False,
            "Usage API connected but zero token events (empty buckets)",
            {"buckets": metrics.buckets},
        )

    burn_rate = metrics.today_cost_usd / max(1.0, datetime.now(timezone.utc).hour or 1)
    monthly_est = burn_rate * 24 * 30

    return CheckResult(
        "api:usage",
        True,
        (
            f"{metrics.events} events · {metrics.total_tokens:,} tokens · "
            f"${metrics.estimated_cost_usd:.4f} est. ({days}d)"
        ),
        {
            "buckets": metrics.buckets,
            "events": metrics.events,
            "total_tokens": metrics.total_tokens,
            "prompt_tokens": metrics.prompt_tokens,
            "completion_tokens": metrics.completion_tokens,
            "estimated_cost_usd": round(metrics.estimated_cost_usd, 6),
            "today_tokens": metrics.today_tokens,
            "today_cost_usd": round(metrics.today_cost_usd, 6),
            "burn_rate_per_hour_usd": round(burn_rate, 6),
            "estimated_monthly_usd": round(monthly_est, 2),
            "top_model": metrics.top_model,
            "top_model_tokens": metrics.top_model_tokens,
        },
    )


def test_billing(
    api_key: str,
    admin_key: str | None,
    billing_token: str | None,
    org_id: str | None,
) -> list[CheckResult]:
    results: list[CheckResult] = []
    keys: list[tuple[str, str]] = []
    if billing_token:
        keys.append(("billing_token", billing_token))
    if admin_key:
        keys.append(("admin_key", admin_key))
    keys.append(("api_key", api_key))

    # Prepaid credits
    for label, key in keys:
        try:
            r = requests.get(
                f"{BASE}/dashboard/billing/credit_grants",
                headers=headers(key, org_id),
                timeout=20,
            )
        except requests.RequestException as e:
            results.append(CheckResult("api:credit_grants", False, str(e)))
            break

        if r.status_code == 200:
            body = r.json()
            results.append(
                CheckResult(
                    "api:credit_grants",
                    True,
                    f"${body.get('total_available', 0):.2f} available (via {label})",
                    {
                        "total_available": body.get("total_available"),
                        "total_granted": body.get("total_granted"),
                        "total_used": body.get("total_used"),
                        "key_used": label,
                    },
                )
            )
            return results

    # Subscription limit
    for label, key in keys:
        try:
            r = requests.get(
                f"{BASE}/v1/dashboard/billing/subscription",
                headers=headers(key, org_id),
                timeout=20,
            )
        except requests.RequestException as e:
            results.append(CheckResult("api:subscription", False, str(e)))
            return results

        if r.status_code == 200:
            sub = r.json()
            limit = (
                sub.get("hard_limit_usd")
                or sub.get("system_hard_limit_usd")
                or sub.get("soft_limit_usd")
            )
            results.append(
                CheckResult(
                    "api:subscription",
                    True,
                    f"Monthly limit ${limit:.2f} (via {label})" if limit else "Subscription OK",
                    {"limit_usd": limit, "key_used": label},
                )
            )
            return results

    results.append(
        CheckResult(
            "api:billing",
            False,
            "Billing APIs unavailable with current keys (optional)",
        )
    )
    return results


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
        print("\n══ TokenPulse OpenAI Regression Agent ══\n")
        for r in results:
            icon = "✓" if r.passed else "✗"
            print(f"  {icon} {r.name}: {r.message}")
            if r.name == "api:usage":
                d = r.details
                print(f"      today: {d.get('today_tokens', 0):,} tokens · ${d.get('today_cost_usd', 0):.4f}")
                print(f"      top model: {d.get('top_model')} ({d.get('top_model_tokens', 0):,} tokens)")
                print(f"      burn/hr: ${d.get('burn_rate_per_hour_usd', 0):.4f} · est monthly: ${d.get('estimated_monthly_usd', 0):.2f}")
            if r.name == "agent:usage_capture" and r.details:
                d = r.details
                print(f"      delta today: +{d.get('delta_tokens', 0):,} tokens")

        passed = sum(1 for r in results if r.passed)
        critical = [
            r
            for r in results
            if r.name in ("api:models", "api:usage", "agent:hello", "agent:usage_capture")
            and not r.passed
        ]
        print(f"\n  {passed}/{len(results)} checks passed")
        if critical:
            print("\n  CRITICAL FAILURES — app OpenAI sync will not work until fixed.\n")
            return 1
        print("\n  OpenAI regression OK — metrics match live API.\n")
        return 0

    critical_failed = any(
        not r.passed
        and r.name in ("api:models", "api:usage", "agent:hello", "agent:usage_capture")
        for r in results
    )
    return 1 if critical_failed else 0


def main() -> int:
    json_out = "--json" in sys.argv
    trigger = "--trigger" in sys.argv or "--no-trigger" not in sys.argv and os.getenv(
        "OPENAI_TRIGGER_USAGE", "false"
    ).lower() in ("1", "true", "yes")
    if "--no-trigger" in sys.argv:
        trigger = False

    days = int(os.getenv("OPENAI_USAGE_DAYS", "30"))
    test_billing_flag = os.getenv("OPENAI_TEST_BILLING", "true").lower() in (
        "1",
        "true",
        "yes",
    )
    trigger_model = os.getenv("OPENAI_TRIGGER_MODEL", "gpt-4o-mini").strip()
    trigger_prompt = os.getenv(
        "OPENAI_TRIGGER_PROMPT",
        "Hello from TokenPulse regression test. Reply with one word only.",
    ).strip()
    trigger_max_tokens = int(os.getenv("OPENAI_TRIGGER_MAX_TOKENS", "10"))
    poll_seconds = int(os.getenv("OPENAI_USAGE_POLL_SECONDS", "120"))
    poll_interval = int(os.getenv("OPENAI_USAGE_POLL_INTERVAL", "15"))

    if not (SCRIPT_DIR / ".env").exists():
        print(
            f"Create {SCRIPT_DIR / '.env'} from .env.example and add your keys.",
            file=sys.stderr,
        )
        return 1

    api_key = os.getenv("OPENAI_API_KEY", "").strip()
    admin_key = os.getenv("OPENAI_ADMIN_KEY", "").strip() or os.getenv(
        "OPENAI_ADMIN_API_KEY", ""
    ).strip()
    org_id = os.getenv("OPENAI_ORG_ID", "").strip() or None
    billing_token = os.getenv("OPENAI_BILLING_TOKEN", "").strip() or os.getenv(
        "OPENAI_SESSION_TOKEN", ""
    ).strip() or None

    results: list[CheckResult] = []
    results.extend(check_env())

    if api_key:
        results.append(test_api_key(api_key, org_id))

    usage_before: UsageMetrics | None = None
    if admin_key:
        buckets_pre, _ = fetch_usage_buckets(admin_key, org_id, days=2)
        if buckets_pre:
            usage_before = aggregate_usage(buckets_pre)
        results.append(test_usage(admin_key, org_id, days))
    elif api_key:
        results.append(
            CheckResult(
                "api:usage",
                False,
                "Skipped — set OPENAI_ADMIN_KEY for usage API test",
            )
        )

    if trigger and api_key and admin_key:
        if not json_out:
            print("\n  → Live agent test: sending hello prompt…\n")
        results.append(
            send_hello_prompt(
                api_key, org_id, trigger_model, trigger_prompt, trigger_max_tokens
            )
        )
        hello_ok = results[-1].passed
        if hello_ok and usage_before is not None:
            if not json_out:
                print(f"\n  → Waiting up to {poll_seconds}s for usage API to reflect today…\n")
            results.append(
                test_usage_capture_after_prompt(
                    admin_key, org_id, usage_before, poll_seconds, poll_interval
                )
            )
        elif hello_ok:
            results.append(
                CheckResult(
                    "agent:usage_capture",
                    False,
                    "Could not read baseline usage before prompt",
                )
            )
    elif trigger and not admin_key:
        results.append(
            CheckResult(
                "agent:hello",
                False,
                "Skipped — need OPENAI_ADMIN_KEY to verify usage capture",
            )
        )

    if test_billing_flag and api_key:
        results.extend(test_billing(api_key, admin_key or None, billing_token, org_id))

    return print_results(results, json_out)


if __name__ == "__main__":
    sys.exit(main())
