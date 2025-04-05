# Protocol Workflow Diagram

This diagram illustrates the complete workflow of the Alignment Protocol, from protocol initialization to submission finalization.

```mermaid
---
config:
  layout: fixed
---
flowchart TD
 subgraph subGraph0["Submission Phase"]
        Submission["Create Submission"]
        CrossLink["Cross-Topic Linking"]
        SubmissionResult["Submission Created Temp ALIGN Tokens Minted"]
  end
 subgraph subGraph1["Staking Phase"]
        Staking["Stake tempAlign Tokens"]
        StakingResult["Temp ALIGN Burned Temp REP Minted"]
  end
 subgraph subGraph2["Voting Phase"]
        VoteCommit["Commit Vote: Hash of Choice + Nonce"]
        VoteReveal["Reveal Vote: Choice + Nonce"]
        VoteTally["Vote Tallied: Quadratic Voting Power Applied"]
  end
 subgraph subGraph3["Finalization Phase"]
        SubmissionFinalize["Finalize Submission"]
        VoteFinalize["Finalize Vote"]
        SubmissionAccepted{"Submission Accepted?"}
        VoteCorrect{"Vote with Consensus?"}
        ContributorPermanent["Convert to Permanent ALIGN"]
        ContributorBurn["Burn Temp ALIGN"]
        ValidatorPermanent["Convert to Permanent REP"]
        ValidatorBurn["Burn Temp REP"]
  end
    Authority(["Protocol Authority"]) -- Initializes --> Init["Protocol Initialization"]
    Init -- Creates token mints and state --> TopicCreation["Topic Creation"]
    TopicCreation -- Defines topics with parameters --> UserSetup["User Setup"]
    UserSetup -- Creates user profiles and token accounts --> Submission
    Contributor(["Data Contributor"]) -- Submits data to topic --> Submission
    Submission --> SubmissionResult
    SubmissionResult -- Optional --> CrossLink
    CrossLink -- Links to additional topics --> SubmissionResult
    SubmissionResult --> Staking
    Validator(["Validator"]) -- Stakes tokens --> Staking
    Staking --> StakingResult
    StakingResult --> VoteCommit
    VoteCommit -- During Commit Phase --> VoteReveal
    VoteReveal -- During Reveal Phase --> VoteTally
    VoteTally --> SubmissionFinalize & VoteFinalize
    SubmissionFinalize --> SubmissionAccepted
    SubmissionAccepted -- Yes --> ContributorPermanent
    SubmissionAccepted -- No --> ContributorBurn
    VoteFinalize --> VoteCorrect
    VoteCorrect -- Yes --> ValidatorPermanent
    VoteCorrect -- No --> ValidatorBurn
     Authority:::participants
     Contributor:::participants
     Validator:::participants
     Init:::setup
     TopicCreation:::setup
     UserSetup:::setup
     Submission:::submission
     CrossLink:::submission
     SubmissionResult:::submission
     Staking:::staking
     StakingResult:::staking
     VoteCommit:::voting
     VoteReveal:::voting
     VoteTally:::voting
     SubmissionFinalize:::finalization
     VoteFinalize:::finalization
     SubmissionAccepted:::decision
     VoteCorrect:::decision
     ContributorPermanent:::outcome
     ContributorBurn:::outcome
     ValidatorPermanent:::outcome
     ValidatorBurn:::outcome
    classDef participants fill:#d8f3dc,stroke:#333,stroke-width:1px
    classDef setup fill:#ade8f4,stroke:#333,stroke-width:1px
    classDef submission fill:#f9d5e5,stroke:#333,stroke-width:1px
    classDef staking fill:#fcf6bd,stroke:#333,stroke-width:1px
    classDef voting fill:#d0d1ff,stroke:#333,stroke-width:1px
    classDef finalization fill:#ffddd2,stroke:#333,stroke-width:1px
    classDef decision fill:#f5cb5c,stroke:#333,stroke-width:1px
    classDef outcome fill:#e4c1f9,stroke:#333,stroke-width:1px
```

## Protocol Workflow Explanation

The Alignment Protocol workflow consists of the following key phases:

### 1. Protocol Initialization

- Protocol Authority initializes the protocol state
- Creates four token mints:
  - Temporary ALIGN token mint (for contributors)
  - Permanent ALIGN token mint
  - Temporary REP token mint (for validators)
  - Permanent REP token mint
- Sets up configuration parameters (e.g., tokens per submission)

### 2. Topic Creation

- Protocol Authority creates topics with specific parameters
- Each topic has dedicated commit and reveal phase durations
- Topics are uniquely identified and organize submissions by category

### 3. User Setup

- Contributors and Validators create user profiles
- Protocol establishes token accounts:
  - Protocol-owned temporary token accounts
  - User-owned permanent token accounts (ATAs)

### 4. Submission Phase

- Contributors submit data to topics
- Submission data is stored in on-chain PDAs
- Submission-topic links are created
- Contributors receive temporary ALIGN tokens (100 per submission)
- Optional: Cross-topic linking for multi-topic submissions

### 5. Staking Phase

- Validators stake temporary ALIGN tokens to participate in voting
- Staked temporary ALIGN is burned
- Temporary REP is minted in 1:1 ratio
- This REP represents voting power in the protocol

### 6. Voting Phase

- Two-phase voting system:
  - **Commit Phase**: Validators submit encrypted vote commitments
    - Hash of (vote choice + secret nonce)
    - Vote power locked based on tempREP tokens
  - **Reveal Phase**: Validators reveal actual vote choice with nonce
    - Vote power applied quadratically (sqrt of token amount)
    - Each validator's vote updates the submission's tallies

### 7. Finalization Phase

- **Submission Finalization**:

  - If accepted (more YES than NO votes):
    - Convert contributor's tempALIGN to permanent ALIGN
    - Send to contributor's personal ATA
  - If rejected:
    - Burn temporary tokens without replacement

- **Vote Finalization**:
  - If validator voted correctly (with majority):
    - Convert tempREP to permanent REP
    - Send to validator's personal ATA
  - If validator voted incorrectly:
    - Burn tempREP tokens without replacement

### Key Considerations

- Time-bound phases enforce structured participation
- Two-stage token lifecycle ensures accountability
- Quadratic voting prevents governance capture by large token holders
- Multiple topics allow cross-subject organization of data
