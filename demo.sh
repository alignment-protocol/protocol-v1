#!/bin/bash

# Include the "demo-magic" helpers
source demo-magic.sh

DEMO_PROMPT="${GREEN}➜ ${CYAN}\W ${COLOR_RESET}"
TYPE_SPEED=30

function comment() {
  cmd=$DEMO_COMMENT_COLOR$1$COLOR_RESET
  echo -en "$cmd"; echo ""
}

clear

comment "# First, let's look at the CLI help to understand available commands:"
pe './align --help'
echo ""

comment "# Check if protocol is already initialized on devnet:"
pe './align query state'

comment "# Initialize the protocol (admin operation):"
pe './align init all'

comment "# Create a user profile (required for all operations):"
pe './align user create-profile'

comment "# Create a test topic (admin operation):"
pe './align topic create "Test Topic" "A test topic for protocol demonstration"'

comment "# List topics to confirm creation and get topic ID:"
pe './align topic list'

comment "# Submit data to the topic:"
pe './align submission submit 0 "ipfs://QmTestHash123"'

comment "# Check submission status:"
pe './align query submission 0'

comment "# Check your token balances:"
pe './align user profile'

comment "#  Enter interactive mode..."
cmd  # Run additional commands interactively
