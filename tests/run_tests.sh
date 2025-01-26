#!/bin/bash


# Check for required dependencies
check_dependency() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Error: Required command '$1' is not available."
        echo "Please install $1 and try again."
        exit 1
    fi
}

check_dependency npx
check_dependency curl

# Create temporary file for wrangler output
WRANGLER_LOG=$(mktemp)
echo "Wrangler logs will be written to: $WRANGLER_LOG"

# Build the worker
# (cd rgeometry-cloudflare && wrangler build)

# Start wrangler in the background
(cd rgeometry-cloudflare && npx wrangler dev --env prebuilt > "$WRANGLER_LOG" 2>&1) &
WRANGLER_PID=$!

# Function to clean up resources
cleanup() {
    cat "$WRANGLER_LOG"
    rm -f "$WRANGLER_LOG"
    kill -- -$$
}
trap cleanup EXIT

./tests/test_server.sh http://localhost:8787
