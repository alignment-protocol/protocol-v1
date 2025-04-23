#!/bin/bash

# Include the "demo-magic" helpers
source demo-magic.sh

DEMO_PROMPT="${GREEN}➜ ${CYAN}\W ${COLOR_RESET}"
TYPE_SPEED=30

function comment() {
	cmd=${DEMO_COMMENT_COLOR}$1${COLOR_RESET}
	echo -en "${cmd}"
	echo ""
}

clear

comment "# First, let's look at the CLI help to understand available commands:"
pe './align --help'
echo ""

comment "# Check if protocol is already initialized on devnet:"
pe './align query state'

comment "# (Admin) Initialize the protocol:"
pe './align init all'

comment "# (Admin) Set tokens-to-mint for submissions in the state:"
pe './align config update-tokens-to-mint 100'

comment "# (User #1) Create a user profile:"
pe './align user create-profile'

comment "# (Admin) Create a test topic:"
pe './align topic create "Test Topic" "A test topic for protocol demonstration"'

comment "# List topics to confirm creation and get topic index:"
pe './align topic list'

comment "# (User #1) Initialize user topic balance before making submissions:"
pe './align user initialize-topic-balance --topic-id 0'

comment "# (User #1) Submit data to the topic:"
pe './align submission submit 0 "User #1 submission"'

comment "# Check token balances after submission:"
pe './align user profile'

comment "# Check submission status:"
pe './align query submission 9hubnWkfubMaqGZvtRE9NthaGCkscxTMvJJz6URga4Tr'

comment "# (User #1) Stake tALIGN tokens to the topic #0 to earn tREP:"
pe './align token stake 0 10'

comment "# Check topic-specific token balances:"
pe './align query topic-balance 0'

comment "# Airdrop some SOL to the user2:"
pe 'solana airdrop 10 ~/.config/solana/user2.json'

comment "# (User #2) Create a profile:"
pe './align --keypair ~/.config/solana/user2.json user create-profile'

comment "# (User #2) Initialize topic balance:"
pe './align --keypair ~/.config/solana/user2.json user initialize-topic-balance --topic-id 0'

comment "# (User #2) Submit data to the topic:"
pe './align --keypair ~/.config/solana/user2.json submission submit 0 "User #2 submission"'

comment "# (User #2) Check submission status:"
pe './align --keypair ~/.config/solana/user2.json query submission zmT4iXKET62TXYonYbx4tC47Fot6jDmb4uJNRpw2m4K'

comment "# (User #2) Stake tALIGN tokens to the topic #0 to earn tREP:"
pe './align --keypair ~/.config/solana/user2.json token stake 0 10'

comment "# (User #2) Check topic-specific token balances:"
pe './align --keypair ~/.config/solana/user2.json query topic-balance 0'

comment "# (User #1) Commit a vote for the submission:"
pe './align vote commit zmT4iXKET62TXYonYbx4tC47Fot6jDmb4uJNRpw2m4K 0 yes 1 "test-secret-nonce"'

comment "# Check token balances after voting:"
pe './align user profile'

comment "#  Enter interactive mode..."
cmd # Run additional commands interactively
