#!/bin/bash

# Check if URL argument is provided
if [ $# -eq 0 ]; then
    echo "Error: URL argument is required"
    echo "Usage: $0 <url>"
    exit 1
fi

URL=$1
TIMEOUT=30
START_TIME=$(date +%s)

while true; do
    # Check if we've exceeded timeout
    CURRENT_TIME=$(date +%s)
    ELAPSED_TIME=$((CURRENT_TIME - START_TIME))
    
    if [ $ELAPSED_TIME -ge $TIMEOUT ]; then
        echo "Error: Timed out after $TIMEOUT seconds waiting for $URL"
        exit 1
    fi

    # Try to curl the URL with a 5 second timeout per request
    HTTP_CODE=$(curl -s --max-time 5 -o /dev/null -w "%{http_code}" "$URL")
    
    if [ $? -ne 0 ]; then
        sleep 1
        continue
    fi
    
    if [ $HTTP_CODE -eq 200 ]; then
        echo "Success: $URL is responding with HTTP 200"
        exit 0
    fi
    
    echo "Waiting for $URL (got HTTP $HTTP_CODE)..."
    sleep 1
done
