# TokenPulse Mini — macOS releases

Pre-built installers so you **don't need Rust or Node** on the target Mac.

## Latest

| Platform | File | Notes |
|----------|------|--------|
| Apple Silicon (M1/M2/M3/M4) | [TokenPulse-Mini_0.1.2_aarch64.dmg](./TokenPulse-Mini_0.1.2_aarch64.dmg) | macOS 11+ · **v0.1.2** — performance, Azure OpenAI, timezone fix |

### v0.1.2
- Performance: async sync, faster widget/menu navigation, expand/collapse fixes
- Azure OpenAI live platform (endpoint, API key, deployment, optional Monitor metrics)
- Today tokens timezone fix (UTC midnight for SQLite queries)
- Provider widgets restore correctly after expanding dashboard

### v0.1.1
- Better AWS CLI path detection for menu bar app (Homebrew + login shell PATH)
- Clearer error when AWS CLI is not installed (`brew install awscli`)

### v0.1.0
- Initial release

## Install

1. Download the `.dmg` from this folder or [on GitHub](https://github.com/saurabhdsh/TokenPulse-Mini/tree/main/releases)
2. Open the DMG and drag **TokenPulse Mini** to Applications
3. First launch: right-click → **Open** if macOS blocks unsigned apps
4. **AWS Bedrock:** install AWS CLI (`brew install awscli`), then **Sync Now** in API Keys
5. Add other keys in the app under **API Keys**, or run `./setup.sh --credentials` from source

## Build from source

See the root [README.md](../README.md) and [setup.sh](../setup.sh).
