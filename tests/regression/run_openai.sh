#!/usr/bin/env bash
# Run OpenAI live API regression tests
set -euo pipefail
cd "$(dirname "$0")"

if [[ ! -f .env ]]; then
  echo "Create .env from .env.example and add your keys:"
  echo "  cp .env.example .env"
  exit 1
fi

if ! python3 -c "import requests" 2>/dev/null; then
  echo "Installing Python dependencies…"
  pip3 install -r requirements.txt
fi

python3 openai_agent.py "$@"
# With live hello prompt + usage capture check:
#   ./run_openai.sh --trigger
