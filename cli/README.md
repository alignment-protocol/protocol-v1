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
2. Initialize the protocol (if not already initialized):
   ```bash
   ./alignment-protocol-cli init state
   ./alignment-protocol-cli init temp-align-mint
   ./alignment-protocol-cli init align-mint
   ./alignment-protocol-cli init temp-rep-mint
   ./alignment-protocol-cli init rep-mint
   ```
3. Create a topic:
   ```bash
   ./alignment-protocol-cli topic create "Climate Data" "Repository for validated climate datasets"
   ```
4. Set up your user accounts:
   ```bash
   ./alignment-protocol-cli user create-profile
   ./alignment-protocol-cli user create-temp-account temp-align
   ./alignment-protocol-cli user create-temp-account temp-rep
   ./alignment-protocol-cli user create-ata align
   ./alignment-protocol-cli user create-ata rep
   ```
5. Submit data to a topic:
   ```bash
   ./alignment-protocol-cli submission submit 0 "ipfs://QmUNLLsPACCz1vLxQVkXqqLX5R1X345qqfHbsf67hvA3Nn"
   ```

## Command Reference

The CLI is organized into logical command groups that mirror the protocol's functionality.

### Main Command Groups

1. Init - Protocol initialization commands
   - State: Initialize the protocol state account
   - TempAlignMint/AlignMint/TempRepMint/RepMint: Initialize token mints
   - UpdateTokensToMint: Update number of tokens to mint per submission
2. Topic - Topic management
   - Create: Create a new topic with name, description, custom voting phases
   - List: View all topics in the protocol
   - View: View details of a specific topic
3. User - User account setup
   - CreateProfile: Create a user profile
   - CreateAta: Create associated token accounts for each token type
   - CreateTempAccount: Create temporary protocol-owned token accounts
   - Profile: View user profile details and token balances
4. Submission - Data submission management
   - Submit: Submit data to a specific topic
   - Link: Link existing submission to another topic
   - Finalize: Finalize a submission after voting to handle token conversion
5. Vote - Voting operations
   - Commit: First phase of voting (commit a hidden vote)
   - Reveal: Second phase of voting (reveal previously committed vote)
   - Finalize: Finalize a vote to handle token conversion
   - SetPhases: Admin function to set custom voting phase timestamps
6. Token - Token operations
   - Stake: Stake temp alignment tokens for a topic to earn reputation
7. Query - Data query and exploration
   - State: View protocol state
   - Submission/Submissions: View specific or all submissions
   - SubmissionTopic: Check submission status in a specific topic
   - Vote: Check vote details
8. Debug - Debugging helpers
   - TokenAccount: Debug token account status
   - Tx: View detailed transaction logs

### Global Options

- `--keypair <PATH>`: Path to your Solana keypair (default: ~/.config/solana/id.json)
- `--cluster <URL>`: Solana cluster to use (default: devnet)
- `--program-id <PUBKEY>`: Program ID for the Alignment Protocol

### Protocol Initialization

```bash
# Initialize state account
alignment-protocol-cli init state

# Initialize token mints
alignment-protocol-cli init temp-align-mint
alignment-protocol-cli init align-mint
alignment-protocol-cli init temp-rep-mint
alignment-protocol-cli init rep-mint

# Update tokens to mint per submission
alignment-protocol-cli init update-tokens-to-mint 1000
```

### Topic Management

```bash
# Create a new topic
alignment-protocol-cli topic create "Topic Name" "Topic Description" --commit-duration 86400 --reveal-duration 86400

# List all topics
alignment-protocol-cli topic list

# View a specific topic
alignment-protocol-cli topic view 0
```

### User Account Setup

```bash
# Create a user profile
alignment-protocol-cli user create-profile

# Create associated token account
alignment-protocol-cli user create-ata temp-align
alignment-protocol-cli user create-ata align
alignment-protocol-cli user create-ata temp-rep
alignment-protocol-cli user create-ata rep

# Create temporary token account (protocol-owned)
alignment-protocol-cli user create-temp-account temp-align
alignment-protocol-cli user create-temp-account temp-rep

# View user profile
alignment-protocol-cli user profile
alignment-protocol-cli user profile <PUBKEY>
```

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

# Set voting phases (admin function)
alignment-protocol-cli vote set-phases 0 0 --commit-start 1715000000 --commit-end 1715086400 --reveal-start 1715086400 --reveal-end 1715172800
```

### Token Operations

```bash
# Stake temporary alignment tokens for a topic
alignment-protocol-cli token stake 0 500
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

1. **Initialization**: Set up state and token mints
2. **Topic Creation**: Create topics for data submissions 
3. **User Setup**: Create user profiles and token accounts
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

## Contributing

Contributions to the Alignment Protocol CLI are welcome! Please see our [CONTRIBUTING.md](../CONTRIBUTING.md) for details.

## License

[MIT License](../LICENSE)