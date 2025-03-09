# Alignment Protocol CLI

A command-line interface for interacting with the Alignment Protocol on Solana.

The Alignment Protocol is a decentralized data quality validation system, enabling contributors to submit data and validators to vote on its quality. It uses a dual-token system with temporary and permanent alignment and reputation tokens.

## Installation

### Prerequisites
- [Rust and Cargo](https://www.rust-lang.org/tools/install)
- [Solana CLI tools](https://docs.solana.com/cli/install-solana-cli-tools)

### Building from Source

```bash
git clone https://github.com/your-organization/alignment-protocol.git
cd alignment-protocol/protocol-v1
cargo build --package alignment-protocol-cli
```

The binary will be located at `./target/debug/alignment-protocol-cli`.

## Quick Start

1. Make sure you have a Solana keypair (default at `~/.config/solana/id.json`)
2. Set up your user profile (creates all required token accounts automatically):
   ```bash
   ./alignment-protocol-cli user create-profile
   ```
3. Submit data to a topic:
   ```bash
   ./alignment-protocol-cli submission submit 0 "ipfs://QmUNLLsPACCz1vLxQVkXqqLX5R1X345qqfHbsf67hvA3Nn"
   ```
4. (Admin only) Create a new topic:
   ```bash
   ./alignment-protocol-cli topic create "Climate Data" "Repository for validated climate datasets"
   ```
5. (Admin only) Update token minting parameters:
   ```bash
   ./alignment-protocol-cli config update-tokens-to-mint 1000
   ```

## Command Reference

The CLI is organized into logical command groups that mirror the protocol's functionality. Admin commands are clearly marked with **[ADMIN]**.

### Main Command Groups

1. Topic - Topic management
   - List: View all topics in the protocol
   - View: View details of a specific topic
   - **[ADMIN]** Create: Create a new topic with name, description, custom voting phases
2. User - User account setup
   - CreateProfile: Create a user profile with all necessary token accounts
   - Profile: View user profile details and token balances
3. Submission - Data submission management
   - Submit: Submit data to a specific topic
   - Link: Link existing submission to another topic
   - Finalize: Finalize a submission after voting to handle token conversion
4. Vote - Voting operations
   - Commit: First phase of voting (commit a hidden vote)
   - Reveal: Second phase of voting (reveal previously committed vote)
   - Finalize: Finalize a vote to handle token conversion
   - **[ADMIN]** SetPhases: Set custom voting phase timestamps
5. Token - Token operations
   - Stake: Stake temp alignment tokens for a topic to earn reputation
   - **[ADMIN]** Mint: Mint tokens to a specific user
6. Query - Data query and exploration
   - State: View protocol state
   - Submission/Submissions: View specific or all submissions
   - SubmissionTopic: Check submission status in a specific topic
   - Vote: Check vote details
7. Debug - Debugging helpers
   - TokenAccount: Debug token account status
   - Tx: View detailed transaction logs
8. **[ADMIN]** Init - Protocol initialization
   - State: Initialize protocol state
   - TempAlignMint/AlignMint/TempRepMint/RepMint: Initialize specific token mints
   - All: Initialize all accounts at once
9. **[ADMIN]** Config - Protocol configuration
   - UpdateTokensToMint: Update number of tokens to mint per submission

### Global Options

- `--keypair <PATH>`: Path to your Solana keypair (default: ~/.config/solana/id.json)
- `--cluster <URL>`: Solana cluster to use (default: devnet)
- `--program-id <PUBKEY>`: Program ID for the Alignment Protocol

### Topic Management

```bash
# List all topics
alignment-protocol-cli topic list

# View a specific topic
alignment-protocol-cli topic view 0

# [ADMIN] Create a new topic
alignment-protocol-cli topic create "Topic Name" "Topic Description" --commit-duration 86400 --reveal-duration 86400
```

### User Account Setup

```bash
# Create a user profile (all token accounts are created automatically)
alignment-protocol-cli user create-profile

# View user profile
alignment-protocol-cli user profile
alignment-protocol-cli user profile <PUBKEY>
```

#### What Happens During User Profile Creation

When you run `alignment-protocol-cli user create-profile`, the CLI performs several operations to set up everything you need to interact with the protocol:

1. **User Profile PDA**: Creates a program-derived address (PDA) that stores your on-chain profile information, including balances of topic-specific tokens and permanent reputation.

2. **Permanent Alignment Token Account**: Creates an Associated Token Account (ATA) linked to your wallet that can hold permanent Align tokens. These tokens are received when your submitted data is validated and accepted.

3. **Permanent Reputation Token Account**: Creates an ATA for permanent Rep tokens. These tokens are earned when you vote correctly on submitted data.

4. **Temporary Alignment Token Vault**: Creates a protocol-owned PDA (not an ATA) that holds temporary alignment tokens. This account is controlled by the protocol to ensure tokens can only be converted to permanent tokens when submissions are validated.

5. **Temporary Reputation Token Vault**: Creates a protocol-owned PDA for temporary reputation tokens. These tokens are staked during voting and can be converted to permanent tokens based on voting outcomes.

The CLI handles all of these steps in a single command, making it simple to get started with the protocol. All of these accounts are created for you automatically, so you don't need to manage token accounts manually.

Additionally, commands like `submission submit`, `vote commit`, and `token stake` automatically check whether you have a profile set up. If you try to use these commands without first running `user create-profile`, the CLI will show an error message directing you to create a profile first.

### Data Submission

```bash
# Submit data to a topic
alignment-protocol-cli submission submit 0 "ipfs://QmHash"

# Link an existing submission to another topic
alignment-protocol-cli submission link 0 1

# Finalize a submission after voting
alignment-protocol-cli submission finalize 0 0
```

### Voting

```bash
# Commit a vote (first phase)
alignment-protocol-cli vote commit 0 0 yes 100 "secret-nonce" --permanent

# Reveal a vote (second phase)
alignment-protocol-cli vote reveal 0 0 yes "secret-nonce"

# Finalize a vote
alignment-protocol-cli vote finalize 0 0

# [ADMIN] Set voting phases
alignment-protocol-cli vote set-phases 0 0 --commit-start 1715000000 --commit-end 1715086400 --reveal-start 1715086400 --reveal-end 1715172800
```

### Token Operations

```bash
# Stake temporary alignment tokens for a topic
alignment-protocol-cli token stake 0 500

# [ADMIN] Mint tokens to a user
alignment-protocol-cli token mint temp-align Gn5Wz88RK2qCsJAPUyE9gThvFWjUTvXXYCdjfvJZk5Ge 1000
```

### Protocol Initialization (Admin)

```bash
# [ADMIN] Initialize protocol state
alignment-protocol-cli init state

# [ADMIN] Initialize all accounts (state and all token mints)
alignment-protocol-cli init all
```

### Protocol Configuration (Admin)

```bash
# [ADMIN] Update tokens to mint per submission
alignment-protocol-cli config update-tokens-to-mint 1000
```

### Querying Data

```bash
# Query protocol state
alignment-protocol-cli query state

# Query a specific submission
alignment-protocol-cli query submission 0

# Query all submissions
alignment-protocol-cli query submissions
alignment-protocol-cli query submissions --by <PUBKEY> --topic 0

# Query submission in a specific topic
alignment-protocol-cli query submission-topic 0 0

# Query vote information
alignment-protocol-cli query vote 0 0
alignment-protocol-cli query vote 0 0 <VALIDATOR_PUBKEY>
```

### Debugging

```bash
# Debug token account status
alignment-protocol-cli debug token-account temp-align
alignment-protocol-cli debug token-account align <USER_PUBKEY>

# Get transaction logs
alignment-protocol-cli debug tx <TX_SIGNATURE>
```

## Token System

The protocol uses four types of tokens:

1. **tempAlign** (temporary alignment tokens): Given to data contributors
2. **Align** (permanent alignment tokens): Converted from tempAlign when data is accepted
3. **tempRep** (temporary reputation tokens): Earned by validators for specific topics
4. **Rep** (permanent reputation tokens): Converted from tempRep for correct votes

## Protocol Workflow

1. **Protocol Deployment**: Protocol is deployed and initialized by administrators
2. **Topic Creation**: Create topics for data submissions 
3. **User Setup**: Create user profile with all necessary token accounts
4. **Data Submission**: Contributors submit data to topics and receive tempAlign tokens
5. **Voting**:
   - **Commit Phase**: Validators commit hidden votes using a hash
   - **Reveal Phase**: Validators reveal their votes with the original data
6. **Submission Finalization**: 
   - If accepted, contributor's tempAlign tokens convert to permanent Align tokens
   - If rejected, tempAlign tokens remain locked
7. **Vote Finalization**:
   - Correct votes: Validator's tempRep tokens convert to permanent Rep tokens
   - Incorrect votes: Validator's tempRep tokens are burned

## Advanced Usage

### Creating a Commit Hash Manually

The CLI automatically generates the commit hash for you, but if you need to create it manually:

```bash
# Format: SHA-256(validator_pubkey + submission_topic_link_pubkey + vote_choice + nonce)
# Where vote_choice is "yes" or "no"
```

### Testing the Protocol

For testing, you can use the `vote set-phases` command to set arbitrary timestamps for voting phases:

```bash
# Set all phases to be active now for testing
alignment-protocol-cli vote set-phases 0 0 --commit-start $(date +%s) --commit-end $(($(date +%s) + 3600)) --reveal-start $(date +%s) --reveal-end $(($(date +%s) + 3600))
```

## Code Structure

The CLI is structured into logical modules:

1. `main.rs` - Entry point that routes commands to the appropriate handlers
2. `cli.rs` - CLI command structure and argument parsing
3. `client.rs` - Client setup and program connection 
4. `utils/` - Utility functions
   - `pda.rs` - PDA derivation functions
   - `time.rs` - Timestamp helper functions
   - `vote.rs` - Vote-related helper functions
5. `commands/` - Command implementations
   - `user/` - User commands implementations
     - `topic.rs` - User topic-related commands
     - `user.rs` - User profile management commands
     - `submission.rs` - Submission-related commands
     - `vote.rs` - Voting-related commands
     - `token.rs` - Token staking commands
     - `query.rs` - Query commands
     - `debug.rs` - Debug commands
   - `admin/` - Admin commands implementations
     - `init.rs` - Protocol initialization commands
     - `config.rs` - Protocol configuration commands
     - `topic.rs` - Topic creation commands
     - `token.rs` - Token minting commands
     - `vote.rs` - Admin vote phase commands

This modular structure makes the codebase more maintainable and easier to extend. Admin commands are clearly marked in the help text with an [ADMIN] prefix to indicate that they require admin privileges and will fail if executed by regular users.

## Contributing

Contributions to the Alignment Protocol CLI are welcome! Please see our [CONTRIBUTING.md](../CONTRIBUTING.md) for details.

## License

[MIT License](../LICENSE)