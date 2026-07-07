# TokenPulse Mini

A premium macOS menu bar widget for tracking AI API token usage and cost across **OpenAI**, **Anthropic**, **AWS Bedrock**, **Azure OpenAI**, and **Google Gemini**.

![TokenPulse Mini](https://img.shields.io/badge/Tauri-2.0-blue) ![React](https://img.shields.io/badge/React-19-61dafb) ![SQLite](https://img.shields.io/badge/SQLite-local-green)

## Features

- **Floating widget** (320×220) — dark glassmorphism UI with live stats, budget ring, sparkline, and provider badges
- **Menu bar app** — lives in the macOS menu bar; click to show/hide widget
- **Always-on-top pin** — keep the widget visible over other windows
- **Expanded dashboard** — full executive-grade analytics view
- **SQLite database** — local storage for providers, models, usage events, daily summaries, and budget settings
- **Provider adapters** — mock connectors with real interfaces ready for live API integration
- **Cost engine** — model-wise pricing, input/output cost split, burn rate, monthly estimates
- **Smart alerts** — daily budget thresholds (50/80/100%), spike detection, expensive model warnings

## Pages

| Page | Description |
|------|-------------|
| Mini Widget | Compact always-on-top floating card |
| Dashboard | Overview with period stats and alerts |
| Providers | 7-day provider cost breakdown |
| Models | Model-level usage table |
| Budget | Daily/monthly limits, timezone, alert thresholds |
| API Keys | Per-provider key management |
| Pricing | Editable model pricing table |
| History | Full usage event log with sync |

## Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install) (required for Tauri)
- macOS (for menu bar / accessory mode)

## Quick Start

```bash
# Install dependencies
npm install

# Run in development (requires Rust)
npm run tauri dev

# Build production app
npm run tauri build
```

## Project Structure

```
src/                    # React + TypeScript frontend
  components/           # MiniWidget, ProgressRing, Sparkline, etc.
  pages/                # Dashboard, Settings, History pages
  hooks/                # useWidgetStats
  lib/api.ts            # Tauri invoke wrappers

src-tauri/src/
  adapters/             # OpenAI, Anthropic, Bedrock, Azure, Gemini
  db/                   # SQLite schema, seeding, queries
  engine/               # Cost calculation + alert engine
  commands/             # Tauri IPC commands
  models/               # Shared Rust/TS data types
```

## Live OpenAI Integration

TokenPulse Mini auto-detects OpenAI credentials from your **macOS environment**:

| Variable | Purpose |
|----------|---------|
| `OPENAI_API_KEY` | Required — validates connection |
| `OPENAI_ADMIN_KEY` | Required for usage history (Organization Usage API) |
| `OPENAI_ORG_ID` | Optional — multi-org accounts |

Detection order: process environment → `launchctl getenv` → login shell (`zsh -ilc`).

Add to `~/.zshrc` or set via launchctl:

```bash
export OPENAI_API_KEY="sk-..."
export OPENAI_ADMIN_KEY="sk-admin-..."  # from platform.openai.com → Organization → Admin keys
```

On launch, the app imports env keys, syncs live OpenAI usage, and replaces mock OpenAI data in the widget/dashboard.

**Note:** Run `npm run tauri dev` from a terminal (or restart the app after setting env vars) so keys are picked up.


Each adapter implements the `ProviderAdapter` trait with a `fetch_usage()` method. Currently returns mock data when an API key is present. Replace the mock implementation with real HTTP calls when ready:

```rust
pub trait ProviderAdapter: Send + Sync {
    fn provider_name(&self) -> &'static str;
    fn fetch_usage(&self, api_key: Option<&str>) -> Result<Vec<UsageEvent>, AdapterError>;
    fn mock_fetch_usage(&self) -> Vec<UsageEvent>;
}
```

## Database

SQLite database stored at `~/Library/Application Support/com.tokenpulse.mini/tokenpulse.db`

Tables: `providers`, `models`, `usage_events`, `daily_summary`, `budget_settings`, `alerts`

Mock data is seeded automatically on first launch.

## License

MIT
