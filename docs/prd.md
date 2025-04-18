# \[PRD\] alignment-protocol: A Decentralized Data-Alignment Protocol

**Product Requirements Document (PRD)**

## Table of Contents

1. [Purpose & Scope](#1-purpose--scope)
2. [Key Stakeholders & Roles](#2-key-stakeholders--roles)
3. [High-Level System Overview](#3-high-level-system-overview)
4. [Detailed Functional Requirements](#4-detailed-functional-requirements)
   - [4.1. Initialize Protocol & Token Mints](#41-initialize-protocol--token-mints)
   - [4.2. Topic Management](#42-topic-management)
   - [4.3. User Setup](#43-user-setup)
   - [4.4. Submit Data & Link to Topics](#44-submit-data--link-to-topics)
   - [4.5. Stake Temporary Alignment Tokens for Temporary Reputation (Topic-Specific)](#45-stake-temporary-alignment-tokens-for-temporary-reputation-topic-specific)
   - [4.6. Commit & Reveal Votes (Topic-Specific)](#46-commit--reveal-votes-topic-specific)
   - [4.7. Finalize Submission & Votes (Topic-Specific)](#47-finalize-submission--votes-topic-specific)
   - [4.8. AI Validation (Optional)](#48-ai-validation-optional)
5. [Data & Account Structures](#5-data--account-structures)
6. [Non-Functional Requirements](#6-non-functional-requirements)
7. [User Workflows & UI/UX](#7-user-workflows--uiux)
8. [Testing & Validation Strategy](#8-testing--validation-strategy)
9. [Milestones & Roadmap](#9-milestones--roadmap)
10. [Open Questions & Future Enhancements](#10-open-questions--future-enhancements)

---

## 1. Purpose & Scope

### 1.1 Purpose

- Build a **community-driven data alignment protocol** on Solana, where:
  1. **Contributors** submit alignment data (via reference) and earn **temporary Alignment Tokens (tempAlign)** within specific **Topics**.
  2. **Validators** stake topic-specific tempAlign tokens to acquire **temporary Reputation Tokens (tempRep)** and vote on data quality within those topics (via commit-reveal). Validators can also use permanent Reputation tokens for voting.
  3. **Accepted** submissions yield permanent tokens (Align/Rep) to contributors & correct validators for that specific topic's vote.
  4. The entire system is transparent and on-chain, with no single point of control.

### 1.2 Scope

- **On-chain Program** (smart contract) with core functionalities organized around Topics:

  1. **Initialize** protocol state, token mints, and configurations (including an AI Oracle).
  2. **Manage Topics**: Create and configure topics for organizing submissions.
  3. **Manage Users**: Create user profiles and associated token accounts (including protocol-owned temporary accounts).
  4. **Submit Data**: Create a `Submission` record containing a data reference (e.g., IPFS hash) and link it to one or more `Topic`s via a `SubmissionTopicLink`. Mint topic-specific `tempAlign` tokens.
  5. **Stake**: Burn topic-specific `tempAlign` tokens for topic-specific `tempRep`.
  6. **Vote**: Commit & Reveal votes on a `SubmissionTopicLink`.
  7. **Finalize**: Determine submission outcome within a topic, reward/burn contributor tokens, and finalize individual validator votes (reward/burn).
  8. **AI Validation**: Optionally allow contributors to request AI validation via an Oracle.

- **Off-chain Components**:
  - A sample CLI or web **client** to invoke on-chain instructions.
  - Data storage relies on **off-chain references** stored in the `Submission` account (e.g., IPFS/Arweave). On-chain data storage is not the current approach.

---

## 2. Key Stakeholders & Roles

1. **Contributors**

   - Submit new data samples via data references (e.g., IPFS hash) to specific `Topic`s.
   - Receive topic-specific `tempAlign` tokens upon submission.
   - Can optionally request AI validation for their submissions within a topic by spending `tempRep`.

2. **Validators (Inspectors)**

   - Stake topic-specific `tempAlign` tokens to gain topic-specific `tempRep` tokens.
   - Vote on submissions within specific topics using `tempRep` or permanent `Rep` (commit-reveal mechanism).
   - Earn permanent tokens (`Align` and `Rep`) if their votes align with the final consensus for that `SubmissionTopicLink`.

3. **Protocol Authority (DAO / Admin)**

   - Initially, could be a single admin or a small multisig.
   - Sets global parameters (e.g., token mint amounts, default phase durations, AI Oracle key) via `initialize_state` and `update_tokens_to_mint`.
   - Creates `Topic`s.
   - Can manually set voting phase timings using `set_voting_phases` (primarily for testing/admin).
   - Eventually potentially replaced by a decentralized DAO governance system.

4. **AI Oracle**

   - An authorized off-chain service (identified by `oracle_pubkey` in `State`).
   - Receives `AiValidationRequest`s.
   - Submits AI decisions (`submit_ai_vote`) which contribute to the voting outcome.

5. **End Users**
   - Anyone who consumes or queries the accepted data (identified by `data_reference`) for fine-tuning AI models.

---

## 3. High-Level System Overview

The system uses a **dual-layer token system** (temporary/permanent Alignment and Reputation tokens) implemented as four separate SPL token mints. Interaction is organized around **Topics**, where submissions are categorized, and voting occurs independently.

1.  **Alignment Tokens** - Economic Incentives:

    - **tempAlignMint**: Produces temporary Alignment tokens.
      - Minted to contributors' topic-specific balance (`UserTopicBalance`) upon submission to that topic.
      - Convertible to permanent Align tokens only if the submission is accepted within that topic.
      - Can be staked within a topic to obtain `tempRep` for that topic.
      - Held in protocol-owned accounts (`UserProfile.user_temp_align_account`) associated with the user for easier burning/transfer by the program.
    - **AlignMint**: Produces permanent Alignment tokens with full transferability.
      - Created through conversion when submissions are accepted within a topic.
      - Held in user's Associated Token Account (ATA).

2.  **Reputation Tokens** - Governance Incentives:
    - **tempRepMint**: Produces temporary Reputation tokens (non-transferable).
      - Acquired by staking `tempAlign` within a specific topic (`UserTopicBalance`).
      - Used for voting on submissions within that topic.
      - Held in protocol-owned accounts (`UserProfile.user_temp_rep_account`).
      - Non-transferable (soulbound).
    - **RepMint**: Produces permanent Reputation tokens (non-transferable).
      - Created through conversion when votes are correct within a topic.
      - Can also be used directly for voting.
      - Held in user's Associated Token Account (ATA).
      - Non-transferable (soulbound).

**Core Workflow**:

1.  **Initialize** – Deploy program, run `initialize_state` (creates `State` PDA with authority, token mints, oracle key, defaults), then `initialize_*_mint` for each of the four mints.
2.  **Create Topic** - Authority calls `create_topic`.
3.  **User Setup** - User calls `create_user_profile`, `create_user_temp_align_account`, `create_user_temp_rep_account`, `initialize_user_topic_balance` (for relevant topics), and potentially `create_user_ata` for permanent tokens.
4.  **Submit** – Contributor calls `submit_data_to_topic`, creating a `Submission` and a `SubmissionTopicLink`. Program mints `tempAlign` to the contributor's `UserTopicBalance` for that topic.
5.  **Stake** – Validator calls `stake_topic_specific_tokens` to burn `tempAlign` and mint `tempRep` within their `UserTopicBalance` for a specific topic.
6.  **Vote** – Validator calls `commit_vote` (using `tempRep` or `Rep` from `UserTopicBalance` or ATA) on a `SubmissionTopicLink`, then `reveal_vote` during the appropriate phase.
7.  **Finalize** – After voting ends for a `SubmissionTopicLink`:
    - Anyone can call `finalize_submission` to determine the Accepted/Rejected status for that link and reward/burn the contributor's topic-specific `tempAlign`.
    - Each validator (or anyone) calls `finalize_vote` for each vote commit to reward/burn the validator's `tempRep`/`Rep` based on correctness for that link.
8.  **(Optional) AI Validation** - Contributor calls `request_ai_validation`, Oracle calls `submit_ai_vote`.

---

## 4. Detailed Functional Requirements

### 4.1 Initialize Protocol & Token Mints

**Description**

- A multi-step process initiated by the authority.
- Creates the global `State` account and the four distinct SPL token mints owned by PDA authorities.

**Requirements**

1.  **`initialize_state` Instruction**:
    - Creates the `State` account (PDA).
    - Stores `authority`, `oracle_pubkey`.
    - Sets initial `topic_count`, `tokens_to_mint` per submission, default commit/reveal phase durations.
    - Requires the authority signature.
2.  **`initialize_*_mint` Instructions** (separate for `tempAlign`, `Align`, `tempRep`, `Rep`):
    - Creates the respective SPL token mint with correct decimals and the program (`State` PDA) as the mint/freeze authority.
    - Stores the mint pubkey in the `State` account.
    - Requires the authority signature.
3.  **`update_tokens_to_mint` Instruction**:
    - Allows authority to change the number of `tempAlign` tokens minted per submission.

**Success Criteria**

- `State` PDA is created and initialized.
- The four SPL mints exist with program-derived authorities and are referenced in `State`.

### 4.2 Topic Management

**Description**

- Allows the authority to create and manage topics for organizing submissions.

**Requirements**

1.  **`create_topic` Instruction**:
    - Creates a `Topic` account (PDA seeded with `topic_count` from `State`).
    - Stores `name`, `description`, `authority` (creator), initial `submission_count` (0), specific commit/reveal phase durations (or defaults from `State`), and `is_active` flag.
    - Increments `topic_count` in `State`.
    - Requires authority signature.

**Success Criteria**

- `Topic` PDA is created and initialized.
- `State.topic_count` is incremented.

### 4.3 User Setup

**Description**

- Instructions for users to create necessary accounts before participating.

**Requirements**

1.  **`create_user_profile` Instruction**:
    - Creates a `UserProfile` account (PDA seeded with user's key).
    - Stores `user` pubkey, initial `user_submission_count` (0), and placeholders for token account pubkeys.
2.  **`create_user_temp_align_account` / `create_user_temp_rep_account` Instructions**:
    - Creates SPL Token accounts for `tempAlign` / `tempRep` mints respectively.
    - Sets the **owner** of these accounts to the `State` PDA, allowing the program to burn tokens without user signature.
    - Stores the created account pubkey in the user's `UserProfile`.
3.  **`initialize_user_topic_balance` Instruction**:
    - Creates a `UserTopicBalance` account (PDA seeded with user's key and topic key).
    - Initializes `temp_align_amount`, `temp_rep_amount`, `locked_temp_rep_amount` to 0.
    - Links `user` and `topic`.
4.  **`create_user_ata` Instruction** (Optional but recommended):
    - Creates the user's Associated Token Accounts (ATAs) for the permanent `Align` and `Rep` mints via CPI. User owns these directly.
    - Stores ATA pubkeys in `UserProfile`.

**Success Criteria**

- User has a `UserProfile` PDA.
- User has protocol-owned `tempAlign` and `tempRep` token accounts linked in their profile.
- User has `UserTopicBalance` PDAs for topics they interact with.
- User potentially has ATAs for permanent tokens linked in their profile.

### 4.4 Submit Data & Link to Topics

**Description**

- Contributors create a core `Submission` record and link it to a specific `Topic` for voting.

**Requirements**

1.  **`submit_data_to_topic` Instruction**:
    - Creates a `Submission` account (PDA seeded with contributor key and their `user_submission_count`). Stores `contributor`, `timestamp`, and `data_reference` (String, e.g., IPFS hash). Increments `user_submission_count` in `UserProfile`.
    - Creates a `SubmissionTopicLink` account (PDA seeded with topic key and topic's `submission_count`). Stores references to `submission` and `topic`, sets initial `status` (Pending), calculates and stores commit/reveal phase timestamps based on `Topic` settings, initializes vote counters/powers to 0. Increments `submission_count` in `Topic`.
    - Mints `State.tokens_to_mint` amount of `tempAlign` tokens to the contributor's `UserTopicBalance` account for the specified topic (CPI to token program, using `State` PDA authority).
2.  **`link_submission_to_topic` Instruction**:
    - Allows anyone to link an _existing_ `Submission` to _another_ `Topic`.
    - Creates a new `SubmissionTopicLink` account for the new combination (similar to above, but references the existing `Submission`).
    - Does _not_ mint additional `tempAlign` tokens.

**Success Criteria**

- `Submission` PDA exists containing data reference.
- `SubmissionTopicLink` PDA exists linking the submission and topic, with voting phases set.
- Contributor receives topic-specific `tempAlign` tokens in their `UserTopicBalance` (for `submit_data_to_topic`).

### 4.5 Stake Temporary Alignment Tokens for Temporary Reputation (Topic-Specific)

**Description**

- Users stake (burn) topic-specific `tempAlign` tokens in exchange for topic-specific `tempRep` tokens to gain voting rights within that topic.

**Requirements**

1.  **`stake_topic_specific_tokens` Instruction**:
    - Requires user's `UserProfile` and `UserTopicBalance` for the specific topic.
    - Burns the specified `amount` of `tempAlign` from the user's `UserTopicBalance` (checking sufficient balance). Requires CPI to token program using `State` PDA authority over the user's temp account.
    - Mints an equivalent `amount` of `tempRep` to the user's `UserTopicBalance` for the same topic. Requires CPI to token program using `State` PDA authority.
    - Updates `temp_align_amount` and `temp_rep_amount` fields in the `UserTopicBalance` PDA.

**Success Criteria**

- User's `tempAlign` balance for the topic decreases.
- User's `tempRep` balance for the topic increases.
- Balances in `UserTopicBalance` PDA are updated.

### 4.6 Commit & Reveal Votes (Topic-Specific)

**Description**

- A two-phase voting process on a specific `SubmissionTopicLink`. Validators can use topic-specific `tempRep` or permanent `Rep`.

**Requirements**

1.  **`commit_vote` Instruction**:
    - Creates a `VoteCommit` account (PDA seeded with `SubmissionTopicLink` key and validator key).
    - Requires the validator's `UserProfile` and `UserTopicBalance` (if using `tempRep`).
    - Takes `vote_hash = hash(validator_key, submission_topic_link_key, vote_choice, nonce)`, `vote_amount`, `is_permanent_rep` flag as input.
    - Stores `submission_topic_link`, `validator`, `vote_hash`, `commit_timestamp`, `vote_amount`, `is_permanent_rep`, and sets `revealed`/`finalized` to false, `vote_choice` to None.
    - If using `tempRep` (`is_permanent_rep` is false):
      - Checks sufficient `temp_rep_amount` in `UserTopicBalance`.
      - Decrements `temp_rep_amount` and increments `locked_temp_rep_amount` in `UserTopicBalance` by `vote_amount`.
    - If using `Rep` (`is_permanent_rep` is true):
      - Checks sufficient balance in user's permanent `Rep` ATA (requires ATA passed to instruction).
      - Transfers `vote_amount` of `Rep` from user's ATA to a program-controlled escrow or vault (Mechanism needs confirmation - currently seems to just check balance, locking might happen implicitly or off-chain).
    - Increments `total_committed_votes` in `SubmissionTopicLink`.
    - Must be called during the commit phase defined in `SubmissionTopicLink`.
2.  **`reveal_vote` Instruction**:
    - Requires the existing `VoteCommit` account, validator signature.
    - Takes `vote_choice` (Yes/No) and `nonce` (String) as input.
    - Re-calculates the hash using provided inputs and verifies it matches `vote_hash` stored in `VoteCommit`.
    - If valid:
      - Sets `revealed = true` and stores the `vote_choice` in `VoteCommit`.
      - Calculates voting power (e.g., quadratic function of `vote_amount`).
      - Adds calculated voting power to `yes_voting_power` or `no_voting_power` in the associated `SubmissionTopicLink`.
      - Increments `total_revealed_votes` in `SubmissionTopicLink`.
    - Must be called during the reveal phase defined in `SubmissionTopicLink`.
3.  **`set_voting_phases` Instruction** (Admin/Testing):
    - Allows the `authority` to manually set the commit/reveal start/end timestamps on a `SubmissionTopicLink`.

**Edge Cases**

- Late reveal or no reveal → VoteCommit remains `revealed = false`. `finalize_vote` should handle burning staked tokens.
- Hash mismatch on reveal → Instruction fails.

**Success Criteria**

- `VoteCommit` PDA created during commit phase, tokens locked/escrowed.
- `VoteCommit` updated during reveal phase, `SubmissionTopicLink` counters/powers updated if valid reveal.

### 4.7 Finalize Submission & Votes (Topic-Specific)

**Description**

- A two-step process after the reveal phase ends for a `SubmissionTopicLink`. First, the overall outcome for the link is determined, rewarding the contributor. Second, individual validator votes are processed.

**Requirements**

1.  **`finalize_submission` Instruction**:
    - Can be called by anyone after `reveal_phase_end`.
    - Requires the `SubmissionTopicLink` and the original `Submission` account.
    - Compares `yes_voting_power` and `no_voting_power` in `SubmissionTopicLink`.
    - Determines outcome: `Accepted` if yes > no, `Rejected` otherwise (handle ties).
    - Updates `status` in `SubmissionTopicLink`.
    - If `Accepted`:
      - Requires contributor's `UserProfile` and `UserTopicBalance` for the topic.
      - Burns _all_ remaining `temp_align_amount` for this topic from contributor's `UserTopicBalance`.
      - Mints an equivalent amount of permanent `Align` tokens to the contributor's `user_align_ata` (requires ATA).
    - If `Rejected`:
      - Burns _all_ remaining `temp_align_amount` for this topic from contributor's `UserTopicBalance` with no replacement.
2.  **`finalize_vote` Instruction**:
    - Can be called by anyone for a specific `VoteCommit` after `finalize_submission` has run on the associated `SubmissionTopicLink`.
    - Requires `VoteCommit`, `SubmissionTopicLink`, validator's `UserProfile`, `UserTopicBalance` (if `tempRep` used), and permanent `Rep` ATA (if `Rep` used).
    - Checks `VoteCommit.revealed` and `VoteCommit.finalized` (should be true and false respectively).
    - Compares `VoteCommit.vote_choice` with the final `SubmissionTopicLink.status`.
    - If vote was correct (e.g., Voted Yes and Status is Accepted):
      - If `is_permanent_rep` is false (used `tempRep`): Burns `vote_amount` from `locked_temp_rep_amount` in `UserTopicBalance`. Mints equivalent permanent `Rep` to validator's `user_rep_ata`.
      - If `is_permanent_rep` is true (used `Rep`): Returns escrowed `Rep` tokens to validator's `user_rep_ata`. (Mechanism needs confirmation). _May_ also mint additional `Rep` as reward.
    - If vote was incorrect:
      - If `is_permanent_rep` is false (used `tempRep`): Burns `vote_amount` from `locked_temp_rep_amount` in `UserTopicBalance` with no replacement.
      - If `is_permanent_rep` is true (used `Rep`): Escrowed `Rep` tokens are _not_ returned (burned or sent to treasury?). (Mechanism needs confirmation).
    - Sets `finalized = true` in `VoteCommit`.

**Success Criteria**

- `SubmissionTopicLink.status` updated to Accepted/Rejected.
- Contributor's topic-specific `tempAlign` converted to `Align` or burned.
- Individual `VoteCommit`s marked finalized.
- Validator's locked `tempRep` converted to `Rep` or burned, or escrowed `Rep` returned/kept based on vote correctness.

### 4.8 AI Validation (Optional)

**Description**

- Allows a contributor to request an AI vote on their submission via an authorized Oracle.

**Requirements**

1.  **`request_ai_validation` Instruction**:
    - Called by the original contributor of the `Submission`.
    - Requires the `SubmissionTopicLink`, contributor's `UserProfile`, `UserTopicBalance`.
    - Takes `temp_rep_to_stake` amount as input.
    - Checks sufficient `temp_rep_amount` in contributor's `UserTopicBalance`.
    - Decrements `temp_rep_amount` and increments `locked_temp_rep_amount` by `temp_rep_to_stake`.
    - Creates an `AiValidationRequest` account (PDA seeded with `SubmissionTopicLink` key and a request index).
    - Stores `submission_topic_link`, `requester`, `temp_rep_staked`, `request_timestamp`, sets `status` to Pending.
2.  **`submit_ai_vote` Instruction**:
    - Called only by the `oracle_pubkey` defined in `State`.
    - Requires the `AiValidationRequest` and associated `SubmissionTopicLink`.
    - Takes `ai_request_index`, `ai_decision` (VoteChoice) as input.
    - Verifies caller is the `oracle_pubkey`.
    - Updates `AiValidationRequest`: sets `status` (Completed/Failed), stores `ai_decision`, calculates and stores `ai_voting_power` (based on `temp_rep_staked`).
    - Adds `ai_voting_power` to `yes_voting_power` or `no_voting_power` in the `SubmissionTopicLink`.
    - (Needs clarification: How is the contributor's staked `temp_rep` handled? Returned on completion? Burned on failure? Affected by final submission outcome?)

**Success Criteria**

- `AiValidationRequest` PDA created, contributor's `tempRep` locked.
- `AiValidationRequest` updated by Oracle, AI vote power added to `SubmissionTopicLink`.

---

## 5. Data & Account Structures

1.  **State** (PDA, Seed: `b"state"`)

    - `authority: Pubkey` - Admin/DAO controlling settings.
    - `oracle_pubkey: Pubkey` - Authorized key for AI validation service.
    - `temp_align_mint: Pubkey`
    - `align_mint: Pubkey`
    - `temp_rep_mint: Pubkey`
    - `rep_mint: Pubkey`
    - `bump: u8`
    - `topic_count: u64` - Counter for seeding new topics.
    - `tokens_to_mint: u64` - Amount of `tempAlign` minted per submission.
    - `default_commit_phase_duration: u64` - Default seconds for commit phase.
    - `default_reveal_phase_duration: u64` - Default seconds for reveal phase.

2.  **Topic** (PDA, Seed: `b"topic", state.topic_count.to_le_bytes()`)

    - `name: String` (Max 64 bytes)
    - `description: String` (Max 256 bytes)
    - `authority: Pubkey` - Who created the topic.
    - `submission_count: u64` - Counter for seeding new links in this topic.
    - `commit_phase_duration: u64` - Specific duration for this topic.
    - `reveal_phase_duration: u64` - Specific duration for this topic.
    - `is_active: bool`
    - `bump: u8`

3.  **UserProfile** (PDA, Seed: `b"user", user.key().as_ref()`)

    - `user: Pubkey` - The user's wallet address.
    - `user_submission_count: u64` - Counter for seeding user's submissions.
    - `user_temp_align_account: Pubkey` - User's protocol-owned tempAlign account.
    - `user_temp_rep_account: Pubkey` - User's protocol-owned tempRep account.
    - `user_align_ata: Pubkey` - User's permanent Align ATA (optional).
    - `user_rep_ata: Pubkey` - User's permanent Rep ATA (optional).
    - `bump: u8`

4.  **UserTopicBalance** (PDA, Seed: `b"user_topic", user.key().as_ref(), topic.key().as_ref()`)

    - `user: Pubkey`
    - `topic: Pubkey`
    - `temp_align_amount: u64` - Balance for this topic.
    - `temp_rep_amount: u64` - Available staking/voting balance for this topic.
    - `locked_temp_rep_amount: u64` - Amount currently committed in votes for this topic.
    - `bump: u8`

5.  **Submission** (PDA, Seed: `b"submission", contributor.key().as_ref(), user_profile.user_submission_count.to_le_bytes()`)

    - `contributor: Pubkey`
    - `timestamp: u64`
    - `data_reference: String` (Max 128 bytes) - e.g., IPFS hash.
    - `bump: u8`

6.  **SubmissionTopicLink** (PDA, Seed: `b"link", topic.key().as_ref(), topic.submission_count.to_le_bytes()`)

    - `submission: Pubkey` - Reference to the core `Submission`.
    - `topic: Pubkey` - Reference to the `Topic`.
    - `status: SubmissionStatus { Pending, Accepted, Rejected }` - Outcome within this topic.
    - `commit_phase_start: u64`
    - `commit_phase_end: u64`
    - `reveal_phase_start: u64`
    - `reveal_phase_end: u64`
    - `yes_voting_power: u64` - Accumulated quadratic voting power.
    - `no_voting_power: u64` - Accumulated quadratic voting power.
    - `total_committed_votes: u64`
    - `total_revealed_votes: u64`
    - `bump: u8`

7.  **VoteCommit** (PDA, Seed: `b"vote", submission_topic_link.key().as_ref(), validator.key().as_ref()`)

    - `submission_topic_link: Pubkey`
    - `validator: Pubkey`
    - `vote_hash: [u8; 32]`
    - `revealed: bool`
    - `finalized: bool`
    - `vote_choice: Option<VoteChoice { Yes, No }>`
    - `commit_timestamp: u64`
    - `vote_amount: u64` - Amount of `tempRep` or `Rep` used.
    - `is_permanent_rep: bool` - True if `Rep` was used, False if `tempRep`.
    - `bump: u8`

8.  **AiValidationRequest** (PDA, Seed: `b"ai_request", submission_topic_link.key().as_ref(), request_index.to_le_bytes()`)
    - `submission_topic_link: Pubkey`
    - `requester: Pubkey` (Original contributor)
    - `temp_rep_staked: u64` - Amount risked by contributor.
    - `request_timestamp: u64`
    - `status: AiValidationStatus { Pending, Processing, Completed, Failed }`
    - `ai_decision: Option<VoteChoice>`
    - `ai_voting_power: u64` - Calculated power based on stake.
    - `request_index: u64` - Unique index for seeding.
    - `bump: u8`

---

## 6. Non-Functional Requirements

1.  **Performance**: Solana transaction speed goals remain. Consider potential bottlenecks with many topics or links.
2.  **Security**: PDAs for authorities/accounts, commit-reveal, token burning remain key. Audit AI Oracle interactions and topic-specific token handling.
3.  **Scalability**: Must handle many topics, submissions, links, and votes efficiently. Topic-specific balances help partition state.
4.  **Maintainability**: Anchor framework, clear code, well-documented tests (including topic interactions and AI flows).

---

## 7. User Workflows & UI/UX

**1. Contributor Workflow**

- Ensure `UserProfile` and necessary token accounts/balances exist (`create_user_profile`, `create_user_*_account`, `initialize_user_topic_balance`).
- Select a `Topic`.
- Upload data off-chain (e.g., IPFS), get reference hash.
- Call `submit_data_to_topic` with topic key and data reference → receive topic-specific `tempAlign` tokens in `UserTopicBalance`.
- (Optional) Call `request_ai_validation` by spending topic-specific `tempRep`.

**2. Validator Workflow**

- Ensure `UserProfile` and necessary token accounts/balances exist.
- Acquire topic-specific `tempAlign` (e.g., via contribution or transfer if enabled later).
- Call `stake_topic_specific_tokens` to get `tempRep` for a desired topic.
- Browse open `SubmissionTopicLink`s for topics they have `tempRep` or `Rep` for.
- During commit phase: Call `commit_vote` with hash, amount, and token type (tempRep/Rep).
- During reveal phase: Call `reveal_vote` with choice and nonce.
- After finalization: Call (or wait for someone to call) `finalize_vote` for their vote to process rewards/penalties.

**3. Admin / DAO Workflow**

- Call `initialize_state` and `initialize_*_mint` instructions.
- Call `create_topic` to add new categories.
- Call `update_tokens_to_mint` or `set_voting_phases` as needed.

---

## 8. Testing & Validation Strategy

1.  **Unit Tests**: For each instruction (`initialize_state`, `initialize_*_mint`, `create_topic`, `create_user_profile`, `create_user_*_account`, `initialize_user_topic_balance`, `submit_data_to_topic`, `link_submission_to_topic`, `stake_topic_specific_tokens`, `commit_vote`, `reveal_vote`, `finalize_submission`, `finalize_vote`, `request_ai_validation`, `submit_ai_vote`).
2.  **Integration Tests**: Full flows within a topic: Initialize → Create Topic → User Setup → Submit → Stake → Vote → Finalize → Check balances (`UserTopicBalance`, ATAs). Test cross-topic linking. Test AI validation flow.
3.  **Edge Cases**: Reveal mismatch, missed phases, finalization order, empty topics, zero votes, AI oracle failure, interactions between topics.
4.  **Security Audits**: Focus on PDA authorities, token mint/burn logic (especially protocol-owned accounts), topic/link state transitions, oracle interaction, permanent Rep voting logic.

---

## 9. Milestones & Roadmap

1.  **Core Topic-Based MVP Implementation** (Largely Achieved)

    - Step 1: `initialize` (state & mints).
    - Step 2: `create_topic`.
    - Step 3: User setup (`UserProfile`, temp accounts, `UserTopicBalance`).
    - Step 4: `submit_data_to_topic` & `tempAlign` minting.
    - Step 5: `stake_topic_specific_tokens`.
    - Step 6: `commit_vote` & `reveal_vote` (using `tempRep`).
    - Step 7: `finalize_submission` & `finalize_vote` → reward/burn (temp tokens to permanent).
    - Step 8: Basic `link_submission_to_topic`.

2.  **Advanced Features & Refinements** (Partially Implemented / Next Steps)

    - Voting with permanent `Rep`. (Implemented)
    - AI Oracle Validation flow. (Implemented)
    - Quadratic voting power calculation. (Implemented - assumed by `*_voting_power` fields)
    - Challenge/Dispute windows. (Future)
    - Weighted or random subset voting. (Future)
    - Bicameral DAO governance. (Future)
    - Off-chain indexing/aggregator. (Future)
    - Refine permanent Rep voting mechanism (escrow/reward).

3.  **Production Rollout**
    - Comprehensive testing (multi-user, stress, security audit).
    - CLI completion.
    - Client UI/dApp development.
    - Devnet → Testnet → Mainnet migration.

---

## 10. Open Questions & Future Enhancements

1.  **Reward/Stake Formulas**:

    - Is `tokens_to_mint` static or dynamic? How is it set?
    - How is voting power calculated from `vote_amount` (confirm quadratic)?
    - How is `ai_voting_power` calculated from `temp_rep_staked`?
    - Permanent Rep voting: Is there an additional reward for using/risking Rep? How is escrow handled? How is burning handled on incorrect votes?
    - AI Validation: What happens to the contributor's staked `tempRep` in `AiValidationRequest` upon completion/failure or final submission outcome?

2.  **Burning/Slashing Config**:

    - Confirm: Incorrect `tempRep` votes are fully burned?
    - Confirm: Incorrect permanent `Rep` votes lead to loss of staked amount? Sent where?
    - Confirm: Contributor's `tempAlign` fully burned on rejection?

3.  **DAO Integration**:

    - Plan for transitioning `authority` from single key to DAO?
    - Which parameters will be DAO-controlled? (`tokens_to_mint`, phase durations, oracle key, topic creation rules?)

4.  **Spam & Sybil Resistance**:

    - Currently relies on transaction fees. Consider minimum stake for submission/topic creation?
    - Rate limiting? Reputation gating?

5.  **Topic Lifecycle**:

    - Can topics be deactivated (`is_active`)? Archived?
    - Limits on number of topics or links?

6.  **Tokenomics**:
    - Utility/value accrual for permanent `Align` and `Rep` tokens?
    - Transferability constraints on `tempAlign` (seems limited via protocol-owned accounts)?

---

# End of Document
