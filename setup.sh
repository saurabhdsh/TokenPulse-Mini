#!/usr/bin/env bash
#
# TokenPulse Mini — macOS setup & credential checks
#
# Usage:
#   ./setup.sh                  Full setup (deps + credential tests + optional build)
#   ./setup.sh --deps-only      Install/check prerequisites + npm install only
#   ./setup.sh --credentials    Run credential tests only (no install)
#   ./setup.sh --build          Also run production build (npm run tauri build)
#   ./setup.sh --help
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CREDENTIALS_FILE="${HOME}/.config/tokenpulse-mini/credentials.env"
MARKER_START="# >>> tokenpulse-mini credentials >>>"
MARKER_END="# <<< tokenpulse-mini credentials <<<"

RUN_DEPS=true
RUN_CREDENTIALS=true
RUN_BUILD=false

# ── Colors ────────────────────────────────────────────────────────────────────
if [[ -t 1 ]]; then
  RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
  BLUE='\033[0;34m'; CYAN='\033[0;36m'; BOLD='\033[1m'; NC='\033[0m'
else
  RED=''; GREEN=''; YELLOW=''; BLUE=''; CYAN=''; BOLD=''; NC=''
fi

pass() { echo -e "${GREEN}✓${NC} $*"; }
fail() { echo -e "${RED}✗${NC} $*"; }
warn() { echo -e "${YELLOW}!${NC} $*"; }
info() { echo -e "${BLUE}→${NC} $*"; }
section() { echo -e "\n${BOLD}${CYAN}══ $* ══${NC}\n"; }

usage() {
  sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
  exit 0
}

for arg in "$@"; do
  case "$arg" in
    --help|-h) usage ;;
    --deps-only) RUN_CREDENTIALS=false ;;
    --credentials) RUN_DEPS=false ;;
    --build) RUN_BUILD=true ;;
    *) echo "Unknown option: $arg"; usage ;;
  esac
done

require_macos() {
  if [[ "$(uname -s)" != "Darwin" ]]; then
    fail "TokenPulse Mini is macOS-only. Detected: $(uname -s)"
    exit 1
  fi
  pass "macOS detected ($(sw_vers -productVersion 2>/dev/null || echo unknown))"
}

command_exists() { command -v "$1" >/dev/null 2>&1; }

read_env_var() {
  local name="$1"
  local value=""

  # 1) Current shell
  value="${!name-}"
  if [[ -n "${value// }" ]]; then
    echo "$value"
    return 0
  fi

  # 2) launchctl (GUI apps on macOS)
  if command_exists launchctl; then
    value="$(launchctl getenv "$name" 2>/dev/null || true)"
    if [[ -n "${value// }" ]]; then
      echo "$value"
      return 0
    fi
  fi

  # 3) Login shell (~/.zshrc etc.)
  if command_exists zsh; then
    value="$(zsh -ilc "print -r -- \${$name}" 2>/dev/null || true)"
    if [[ -n "${value// }" ]]; then
      echo "$value"
      return 0
    fi
  fi

  # 4) Saved credentials file from a previous setup run
  if [[ -f "$CREDENTIALS_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$CREDENTIALS_FILE"
    value="${!name-}"
    if [[ -n "${value// }" ]]; then
      echo "$value"
      return 0
    fi
  fi

  return 1
}

mask_secret() {
  local s="$1"
  local len=${#s}
  if (( len <= 8 )); then
    echo "••••••••"
  else
    echo "${s:0:4}…${s: -4}"
  fi
}

prompt_secret() {
  local prompt="$1"
  local var_name="$2"
  local value=""
  echo -en "${YELLOW}?${NC} ${prompt}: "
  read -rs value
  echo
  if [[ -z "${value// }" ]]; then
    return 1
  fi
  printf -v "$var_name" '%s' "$value"
}

prompt_value() {
  local prompt="$1"
  local var_name="$2"
  local value=""
  echo -en "${YELLOW}?${NC} ${prompt}: "
  read -r value
  if [[ -z "${value// }" ]]; then
    return 1
  fi
  printf -v "$var_name" '%s' "$value"
}

save_credentials_file() {
  mkdir -p "$(dirname "$CREDENTIALS_FILE")"
  cat >"$CREDENTIALS_FILE" <<EOF
# TokenPulse Mini — local credentials (chmod 600)
# Sourced by setup.sh for tests. The app also reads macOS env vars.
# Add to ~/.zshrc:  source "$CREDENTIALS_FILE"

export OPENAI_API_KEY="${OPENAI_API_KEY:-}"
export OPENAI_ADMIN_KEY="${OPENAI_ADMIN_KEY:-}"
export OPENAI_ORG_ID="${OPENAI_ORG_ID:-}"
export AWS_ACCESS_KEY_ID="${AWS_ACCESS_KEY_ID:-}"
export AWS_SECRET_ACCESS_KEY="${AWS_SECRET_ACCESS_KEY:-}"
export AWS_SESSION_TOKEN="${AWS_SESSION_TOKEN:-}"
export AWS_REGION="${AWS_REGION:-us-east-1}"
export AWS_PROFILE="${AWS_PROFILE:-}"
EOF
  chmod 600 "$CREDENTIALS_FILE"
  pass "Saved credentials to $CREDENTIALS_FILE (mode 600)"
}

offer_zshrc_install() {
  local zshrc="${HOME}/.zshrc"
  echo
  info "TokenPulse reads keys from macOS environment (not this file directly)."
  echo -en "${YELLOW}?${NC} Add 'source $CREDENTIALS_FILE' to ~/.zshrc? [y/N] "
  read -r answer
  if [[ ! "$answer" =~ ^[Yy]$ ]]; then
    warn "Skipped ~/.zshrc. You can save keys in the app under API Keys instead."
    return 0
  fi

  touch "$zshrc"
  if grep -qF "$MARKER_START" "$zshrc" 2>/dev/null; then
    # Replace existing block
    local tmp
    tmp="$(mktemp)"
    awk -v start="$MARKER_START" -v end="$MARKER_END" -v src="source \"$CREDENTIALS_FILE\"" '
      $0 == start { skip=1; print start; print src; next }
      $0 == end { skip=0; print end; next }
      !skip { print }
    ' "$zshrc" >"$tmp" && mv "$tmp" "$zshrc"
  else
    cat >>"$zshrc" <<EOF

$MARKER_START
source "$CREDENTIALS_FILE"
$MARKER_END
EOF
  fi
  pass "Updated ~/.zshrc — run: source ~/.zshrc"
}

set_launchctl_env() {
  local name="$1" value="$2"
  if command_exists launchctl; then
    launchctl setenv "$name" "$value" 2>/dev/null || true
  fi
}

export_env_for_gui() {
  [[ -n "${OPENAI_API_KEY:-}" ]] && set_launchctl_env OPENAI_API_KEY "$OPENAI_API_KEY"
  [[ -n "${OPENAI_ADMIN_KEY:-}" ]] && set_launchctl_env OPENAI_ADMIN_KEY "$OPENAI_ADMIN_KEY"
  [[ -n "${OPENAI_ORG_ID:-}" ]] && set_launchctl_env OPENAI_ORG_ID "$OPENAI_ORG_ID"
  [[ -n "${AWS_ACCESS_KEY_ID:-}" ]] && set_launchctl_env AWS_ACCESS_KEY_ID "$AWS_ACCESS_KEY_ID"
  [[ -n "${AWS_SECRET_ACCESS_KEY:-}" ]] && set_launchctl_env AWS_SECRET_ACCESS_KEY "$AWS_SECRET_ACCESS_KEY"
  [[ -n "${AWS_SESSION_TOKEN:-}" ]] && set_launchctl_env AWS_SESSION_TOKEN "$AWS_SESSION_TOKEN"
  [[ -n "${AWS_REGION:-}" ]] && set_launchctl_env AWS_REGION "$AWS_REGION"
  [[ -n "${AWS_PROFILE:-}" ]] && set_launchctl_env AWS_PROFILE "$AWS_PROFILE"
  pass "Exported vars via launchctl (GUI apps can read them until logout)"
}

# ── Prerequisite checks ───────────────────────────────────────────────────────
check_xcode_cli() {
  if xcode-select -p >/dev/null 2>&1; then
    pass "Xcode Command Line Tools installed"
    return 0
  fi
  fail "Xcode Command Line Tools missing (required for Rust/Tauri build)"
  echo -en "${YELLOW}?${NC} Install now? [y/N] "
  read -r answer
  if [[ "$answer" =~ ^[Yy]$ ]]; then
    xcode-select --install || true
    warn "Complete the Xcode CLT dialog, then re-run ./setup.sh"
    exit 1
  fi
  return 1
}

check_node() {
  if command_exists node && command_exists npm; then
    local nv
    nv="$(node -v | sed 's/^v//')"
    pass "Node.js v${nv} ($(command -v node))"
    return 0
  fi
  fail "Node.js 18+ not found"
  if command_exists brew; then
    echo -en "${YELLOW}?${NC} Install Node via Homebrew (brew install node)? [y/N] "
    read -r answer
    if [[ "$answer" =~ ^[Yy]$ ]]; then
      brew install node
      pass "Node.js installed"
      return 0
    fi
  fi
  info "Install from https://nodejs.org/ (18+)"
  return 1
}

check_rust() {
  if command_exists cargo && command_exists rustc; then
    pass "Rust $(rustc --version | awk '{print $2}')"
    return 0
  fi
  fail "Rust/Cargo not found (required for Tauri)"
  echo -en "${YELLOW}?${NC} Install Rust via rustup? [y/N] "
  read -r answer
  if [[ "$answer" =~ ^[Yy]$ ]]; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck disable=SC1091
    source "${HOME}/.cargo/env"
    pass "Rust installed"
    return 0
  fi
  info "Install from https://rustup.rs/"
  return 1
}

install_npm_deps() {
  section "Installing npm dependencies"
  cd "$SCRIPT_DIR"
  npm install
  pass "npm install complete"
}

build_app() {
  section "Building production app"
  cd "$SCRIPT_DIR"
  npm run tauri build
  pass "Build complete — check src-tauri/target/release/bundle/"
}

# ── OpenAI credential tests ───────────────────────────────────────────────────
test_openai_api_key() {
  local key="$1"
  local http_code
  http_code="$(curl -sS -o /dev/null -w "%{http_code}" \
    --max-time 20 \
    -H "Authorization: Bearer ${key}" \
    "https://api.openai.com/v1/models" || echo "000")"

  case "$http_code" in
    200) pass "OpenAI API Key valid (HTTP 200)"; return 0 ;;
    401) fail "OpenAI API Key rejected (HTTP 401 — invalid key)"; return 1 ;;
    403) warn "OpenAI API Key returned HTTP 403 (key valid but may lack permissions)"; return 0 ;;
    *)   fail "OpenAI API Key test failed (HTTP ${http_code})"; return 1 ;;
  esac
}

test_openai_admin_key() {
  local key="$1"
  local org="${2:-}"
  local http_code
  local args=(-sS -o /dev/null -w "%{http_code}" --max-time 20
    -H "Authorization: Bearer ${key}"
    -H "Content-Type: application/json"
    "https://api.openai.com/v1/organization/usage/completions?start_time=1&limit=1")
  if [[ -n "$org" ]]; then
    args+=(-H "OpenAI-Organization: ${org}")
  fi
  http_code="$(curl "${args[@]}" || echo "000")"

  case "$http_code" in
    200) pass "OpenAI Admin Key valid — usage API accessible (HTTP 200)"; return 0 ;;
    401) fail "OpenAI Admin Key rejected (HTTP 401)"; return 1 ;;
    403)
      fail "OpenAI Admin Key missing api.usage.read scope (HTTP 403)"
      info "Create an Admin Key at platform.openai.com → Settings → API Keys → Admin keys"
      return 1
      ;;
    *) fail "OpenAI Admin Key test failed (HTTP ${http_code})"; return 1 ;;
  esac
}

setup_openai_credentials() {
  section "OpenAI credentials"

  OPENAI_API_KEY="$(read_env_var OPENAI_API_KEY 2>/dev/null || true)"
  OPENAI_ADMIN_KEY="$(read_env_var OPENAI_ADMIN_KEY 2>/dev/null || true)"
  OPENAI_ORG_ID="$(read_env_var OPENAI_ORG_ID 2>/dev/null || true)"

  if [[ -n "${OPENAI_API_KEY:-}" ]]; then
    pass "OPENAI_API_KEY found in environment ($(mask_secret "$OPENAI_API_KEY"))"
  else
    warn "OPENAI_API_KEY not found in env / ~/.zshrc / launchctl"
    prompt_secret "Enter OpenAI API Key (sk-…)" OPENAI_API_KEY || true
  fi

  if [[ -n "${OPENAI_ADMIN_KEY:-}" ]]; then
    pass "OPENAI_ADMIN_KEY found in environment ($(mask_secret "$OPENAI_ADMIN_KEY"))"
  else
    warn "OPENAI_ADMIN_KEY not found — required for usage sync"
    prompt_secret "Enter OpenAI Admin Key (sk-admin-…)" OPENAI_ADMIN_KEY || true
  fi

  if [[ -z "${OPENAI_ORG_ID:-}" ]]; then
    info "OPENAI_ORG_ID optional — press Enter to skip"
    prompt_value "Organization ID (optional)" OPENAI_ORG_ID || true
  else
    pass "OPENAI_ORG_ID found (${OPENAI_ORG_ID})"
  fi

  local tests_ok=true

  if [[ -n "${OPENAI_API_KEY:-}" ]]; then
    test_openai_api_key "$OPENAI_API_KEY" || tests_ok=false
  else
    fail "No OpenAI API Key — skip API test"
    tests_ok=false
  fi

  if [[ -n "${OPENAI_ADMIN_KEY:-}" ]]; then
    test_openai_admin_key "$OPENAI_ADMIN_KEY" "${OPENAI_ORG_ID:-}" || tests_ok=false
  else
    fail "No OpenAI Admin Key — usage sync will not work in the app"
    tests_ok=false
  fi

  if [[ -n "${OPENAI_API_KEY:-}" || -n "${OPENAI_ADMIN_KEY:-}" ]]; then
    save_credentials_file
    offer_zshrc_install
    export_env_for_gui
  fi

  $tests_ok && return 0 || return 1
}

# ── AWS credential tests ──────────────────────────────────────────────────────
find_aws_cli() {
  for candidate in aws /opt/homebrew/bin/aws /usr/local/bin/aws; do
    if command -v "$candidate" >/dev/null 2>&1; then
      if "$candidate" --version >/dev/null 2>&1; then
        echo "$candidate"
        return 0
      fi
    fi
  done
  return 1
}

test_aws_cli() {
  local aws_bin
  if aws_bin="$(find_aws_cli)"; then
    pass "AWS CLI found: $("$aws_bin" --version 2>&1 | head -1)"
    echo "$aws_bin"
    return 0
  fi
  fail "AWS CLI not installed (required for Bedrock cost sync)"
  if command_exists brew; then
    echo -en "${YELLOW}?${NC} Install AWS CLI via Homebrew (brew install awscli)? [y/N] "
    read -r answer
    if [[ "$answer" =~ ^[Yy]$ ]]; then
      brew install awscli
      aws_bin="$(find_aws_cli)" && pass "AWS CLI installed" && echo "$aws_bin" && return 0
    fi
  fi
  info "Install: brew install awscli  OR  https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html"
  return 1
}

load_aws_profile_from_file() {
  local profile="${1:-default}"
  local creds="${HOME}/.aws/credentials"
  [[ -f "$creds" ]] || return 1

  AWS_ACCESS_KEY_ID="$(awk -v p="[$profile]" '
    $0 == p { found=1; next }
    /^\[/ { found=0 }
    found && $1 == "aws_access_key_id" { sub(/^aws_access_key_id[[:space:]]*=[[:space:]]*/, ""); gsub(/ /,""); print; exit }
  ' "$creds")"
  AWS_SECRET_ACCESS_KEY="$(awk -v p="[$profile]" '
    $0 == p { found=1; next }
    /^\[/ { found=0 }
    found && $1 == "aws_secret_access_key" { sub(/^aws_secret_access_key[[:space:]]*=[[:space:]]*/, ""); gsub(/ /,""); print; exit }
  ' "$creds")"
  AWS_SESSION_TOKEN="$(awk -v p="[$profile]" '
    $0 == p { found=1; next }
    /^\[/ { found=0 }
    found && $1 == "aws_session_token" { sub(/^aws_session_token[[:space:]]*=[[:space:]]*/, ""); gsub(/ /,""); print; exit }
  ' "$creds")"

  [[ -n "${AWS_ACCESS_KEY_ID:-}" && -n "${AWS_SECRET_ACCESS_KEY:-}" ]]
}

test_aws_sts() {
  local aws_bin="$1"
  local output
  if output="$("$aws_bin" sts get-caller-identity --output json 2>&1)"; then
    local arn account
    arn="$(echo "$output" | grep -o '"Arn": "[^"]*"' | head -1 | cut -d'"' -f4)"
    account="$(echo "$output" | grep -o '"Account": "[^"]*"' | head -1 | cut -d'"' -f4)"
    pass "AWS credentials valid — ${arn:-connected} (account ${account:-?})"
    return 0
  fi
  fail "AWS STS check failed: $(echo "$output" | tr '\n' ' ' | head -c 200)"
  if echo "$output" | grep -q "SignatureDoesNotMatch"; then
    info "Access Key and Secret Key do not match — re-enter the correct pair"
  fi
  return 1
}

test_aws_cost_explorer() {
  local aws_bin="$1"
  local region="${AWS_REGION:-us-east-1}"
  local end start
  end="$(date -u +%Y-%m-%d)"
  start="$(date -u -v-2d +%Y-%m-%d 2>/dev/null || date -u -d '2 days ago' +%Y-%m-%d 2>/dev/null || echo "$end")"

  local output
  if output="$("$aws_bin" ce get-cost-and-usage \
    --time-period "Start=${start},End=${end}" \
    --granularity DAILY \
    --metrics UnblendedCost \
    --filter '{"Dimensions":{"Key":"SERVICE","Values":["Amazon Bedrock"]}}' \
    --region "$region" \
    --output json 2>&1)"; then
    pass "Cost Explorer (Bedrock) API accessible"
    return 0
  fi
  if echo "$output" | grep -qi "AccessDenied\|not authorized"; then
    warn "AWS keys work but lack ce:GetCostAndUsage permission (Bedrock sync may fail)"
    info "Add Cost Explorer read permission to the IAM user/role"
    return 0
  fi
  warn "Cost Explorer test inconclusive: $(echo "$output" | tr '\n' ' ' | head -c 160)"
  return 0
}

setup_aws_credentials() {
  section "AWS / Bedrock credentials"

  local aws_bin=""
  aws_bin="$(test_aws_cli)" || true
  [[ -n "$aws_bin" ]] || return 1

  AWS_ACCESS_KEY_ID="$(read_env_var AWS_ACCESS_KEY_ID 2>/dev/null || true)"
  AWS_SECRET_ACCESS_KEY="$(read_env_var AWS_SECRET_ACCESS_KEY 2>/dev/null || true)"
  AWS_SESSION_TOKEN="$(read_env_var AWS_SESSION_TOKEN 2>/dev/null || true)"
  AWS_REGION="$(read_env_var AWS_REGION 2>/dev/null || true)"
  AWS_PROFILE="$(read_env_var AWS_PROFILE 2>/dev/null || true)"

  if [[ -f "${HOME}/.aws/credentials" ]]; then
    pass "~/.aws/credentials found"
  else
    warn "No ~/.aws/credentials file"
  fi

  # Use CLI profile if no explicit keys in env
  if [[ -z "${AWS_ACCESS_KEY_ID:-}" || -z "${AWS_SECRET_ACCESS_KEY:-}" ]]; then
    local profile="${AWS_PROFILE:-default}"
    if load_aws_profile_from_file "$profile"; then
      pass "Loaded AWS profile '${profile}' from ~/.aws/credentials"
      export AWS_ACCESS_KEY_ID AWS_SECRET_ACCESS_KEY AWS_SESSION_TOKEN
    fi
  fi

  if [[ -n "${AWS_ACCESS_KEY_ID:-}" ]]; then
    pass "AWS_ACCESS_KEY_ID set ($(mask_secret "$AWS_ACCESS_KEY_ID"))"
  else
    warn "AWS_ACCESS_KEY_ID not found"
    prompt_value "Enter AWS Access Key ID" AWS_ACCESS_KEY_ID || true
  fi

  if [[ -n "${AWS_SECRET_ACCESS_KEY:-}" ]]; then
    pass "AWS_SECRET_ACCESS_KEY set"
  else
    warn "AWS_SECRET_ACCESS_KEY not found"
    prompt_secret "Enter AWS Secret Access Key" AWS_SECRET_ACCESS_KEY || true
  fi

  if [[ -z "${AWS_REGION:-}" ]]; then
    prompt_value "AWS Region [us-east-1]" AWS_REGION || AWS_REGION="us-east-1"
  else
    pass "AWS_REGION=${AWS_REGION}"
  fi

  # Export for aws CLI subprocess tests
  export AWS_ACCESS_KEY_ID AWS_SECRET_ACCESS_KEY AWS_SESSION_TOKEN AWS_REGION
  unset AWS_PROFILE  # prefer explicit keys for STS test when we have them

  local tests_ok=true
  test_aws_sts "$aws_bin" || tests_ok=false

  if $tests_ok; then
    test_aws_cost_explorer "$aws_bin" || true
  fi

  if [[ -n "${AWS_ACCESS_KEY_ID:-}" || -n "${AWS_SECRET_ACCESS_KEY:-}" ]]; then
    save_credentials_file
    offer_zshrc_install
    export_env_for_gui
  fi

  $tests_ok && return 0 || return 1
}

# ── Summary ───────────────────────────────────────────────────────────────────
print_summary() {
  section "Setup summary"
  echo "  Project:     $SCRIPT_DIR"
  echo "  Credentials: $CREDENTIALS_FILE"
  echo ""
  info "Start dev app:    cd \"$SCRIPT_DIR\" && npm run tauri dev"
  info "Or save keys in:  App → API Keys (takes priority over env)"
  echo ""
  warn "Keys saved in the app override env/CLI credentials for sync."
}

# ── Main ──────────────────────────────────────────────────────────────────────
main() {
  echo -e "${BOLD}TokenPulse Mini — Setup${NC}"
  require_macos

  local cred_ok=true

  if $RUN_DEPS; then
    section "Prerequisites"
    check_xcode_cli || true
    check_node || exit 1
    check_rust || exit 1
    install_npm_deps
  fi

  if $RUN_CREDENTIALS; then
    setup_openai_credentials || cred_ok=false
    setup_aws_credentials || cred_ok=false
  fi

  if $RUN_BUILD; then
    build_app
  fi

  print_summary

  if ! $cred_ok; then
    warn "Some credential tests failed — fix keys and re-run: ./setup.sh --credentials"
    exit 1
  fi

  pass "Setup complete"
}

main "$@"
