# Alignment Protocol

A decentralized data alignment protocol on Solana where contributors submit data and validators vote on its quality using a dual-token system of alignment and reputation tokens.

## Overview

The Alignment Protocol implements a community-driven data quality validation system with the following core workflow:

1. **Contributors** submit data and earn temporary Alignment Tokens (tempAlign)
2. **Validators** stake tempAlign tokens to acquire temporary Reputation Tokens (tempRep) for voting
3. Submissions with positive consensus yield permanent tokens (Align/Rep) to contributors and correct validators
4. The entire system is transparent and on-chain

## Features

- Four token system (tempAlign, Align, tempRep, Rep) to incentivize quality contributions and validation
- Two-phase voting (commit-reveal) to prevent collusion
- Topic-based organization of submissions
- Cross-topic linking to categorize submissions across multiple topics
- CLI for both admin and user interactions

## Project Structure

- `programs/`: Solana on-chain program written in Rust with Anchor framework
- `tests/`: End-to-end tests organized in sections
- `cli/`: Command-line interface for protocol interaction
- `migrations/`: Deployment scripts
- `docs/`: Protocol documentation and diagrams

## Protocol Workflow

1. **Initialize Protocol**: Deploy program, create tokens, and initialize state
2. **Create Topics**: Define categories for data submissions
3. **User Setup**: Create profiles and token accounts
4. **Submit Data**: Contributors submit data to topics and receive tempAlign tokens
5. **Stake Tokens**: Validators stake tempAlign to get tempRep for voting
6. **Vote**: Two-phase commit-reveal voting process
7. **Finalize**: Convert temporary tokens to permanent tokens based on voting outcomes

## Deployment

The protocol is deployed on Solana devnet at address:
```
3NYwpp7xv6m35he2sbWTDKPyzepwQiJq5YkD2AzdHmRc
```

## CLI Usage

The CLI provides a unified interface for both user and admin operations:

```bash
# User operations
./alignment-protocol-cli user create-profile
./alignment-protocol-cli submission submit 0 "ipfs://QmHash"
./alignment-protocol-cli vote commit 0 0 yes 100 "secret-nonce"

# Admin operations
./alignment-protocol-cli init all
./alignment-protocol-cli topic create "Topic Name" "Description"

# Interacting with devnet deployment
./alignment-protocol-cli --cluster devnet --program-id 3NYwpp7xv6m35he2sbWTDKPyzepwQiJq5YkD2AzdHmRc query state
```

Use `./alignment-protocol-cli --help` to see all available commands.
