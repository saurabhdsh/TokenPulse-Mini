#!/usr/bin/env bash
# Run AWS Bedrock regression (standalone — credentials in aws_agent.py)
set -euo pipefail
cd "$(dirname "$0")"

if ! python3 -c "import boto3" 2>/dev/null; then
  echo "Installing boto3…"
  pip3 install boto3
fi

python3 aws_agent.py "$@"
