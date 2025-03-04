# Decentralized Data-Alignment Protocol  
**Product Requirements Document (PRD)**  

## Table of Contents
1. [Purpose & Scope](#1-purpose--scope)  
2. [Key Stakeholders & Roles](#2-key-stakeholders--roles)  
3. [High-Level System Overview](#3-high-level-system-overview)  
4. [Detailed Functional Requirements](#4-detailed-functional-requirements)  
   - [4.1. Initialize Protocol & Token Mints](#41-initialize-protocol--token-mints)  
   - [4.2. Submit Data & Mint Temporary Alignment Tokens](#42-submit-data--mint-temporary-alignment-tokens)  
   - [4.3. Stake Temporary Alignment Tokens for Temporary Reputation](#43-stake-temporary-alignment-tokens-for-temporary-reputation)  
   - [4.4. Commit & Reveal Votes](#44-commit--reveal-votes)  
   - [4.5. Finalize Submission & Convert Temporary Tokens to Permanent Tokens](#45-finalize-submission--convert-temporary-tokens-to-permanent-tokens)  
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
  1. **Contributors** submit alignment data and earn **temporary Alignment Tokens (tempAlign)**.
  2. **Validators** stake tempAlign tokens to acquire **temporary Reputation Tokens (tempRep)** and vote on data quality (via commit-reveal).
  3. **Accepted** submissions yield permanent tokens (Align/Rep) to contributors & correct validators.
  4. The entire system is transparent and on-chain, with no single point of control.

### 1.2 Scope
- **On-chain Program** (smart contract) with five core functionalities:
  1. **Initialize** protocol & create token mints.
  2. **Submit** data & mint **tempAlign** tokens.
  3. **Stake** tempAlign tokens for **tempRep**.
  4. **Commit & Reveal** votes.
  5. **Finalize** the submission (reward or burn, converting temporary tokens to permanent tokens or burning them).

- **Off-chain Components**:
  - A sample CLI or web **client** to invoke on-chain instructions.
  - Data storage approach:
    - MVP: Store data directly on-chain as a string field in the Submission account
    - Future: Optional IPFS/Arweave integration for storing larger data sets

---

## 2. Key Stakeholders & Roles

1. **Contributors**  
   - Submit new data samples (text, images, code, etc.).
   - Receive `tempAlign` tokens upon submission (which initially have no real value).

2. **Validators (Inspectors)**  
   - Stake `tempAlign` tokens to gain `tempRep` tokens.
   - Vote on submissions (commit-reveal mechanism).
   - Earn permanent tokens (`Align` and `Rep`) if their votes align with final consensus.

3. **Protocol Authority (DAO / Admin)**  
   - Initially, could be a single admin or a small multisig.
   - Sets global parameters (e.g., reward formulas, burning rates).
   - Eventually replaced by a decentralized DAO governance system.

4. **End Users**  
   - Anyone who consumes or queries the accepted data for fine-tuning AI models.

---

## 3. High-Level System Overview

The system uses a **dual-layer token system** with both temporary and permanent variants, implemented as four separate token mints:

1. **Alignment Tokens** - Economic Incentives:
   - **tempAlignMint**: Produces temporary Alignment tokens with limited transferability
     - Minted to contributors at data submission time
     - Convertible to permanent Align tokens only if the submission is accepted
     - Can be staked to obtain tempRep for governance participation

   - **AlignMint**: Produces permanent Alignment tokens with full transferability
     - Created through conversion when submissions are accepted
     - Fully transferable, offering economic flexibility
     - Used for economic incentives and revenue sharing

2. **Reputation Tokens** - Governance Incentives:
   - **tempRepMint**: Produces temporary Reputation tokens (non-transferable)
     - Acquired by staking tempAlign tokens
     - Used for voting on submissions
     - Non-transferable (soulbound)

   - **RepMint**: Produces permanent Reputation tokens (non-transferable)
     - Created through conversion when votes are correct
     - Used for long-term governance rights
     - Non-transferable (soulbound), ensuring governance accountability

**Core Workflow**:
1. **Initialize** – Deploy program, create `GlobalState`, define the four token mints.  
2. **Submit** – Create a `Submission` record, store the data link, mint temporary `tempAlign` tokens.  
3. **Stake** – Burn `tempAlign` and mint `tempRep` for voting.  
4. **Vote** – Use commit-reveal to mitigate collusion.  
5. **Finalize** – Tally results:
   - If accepted: Burn contributor's `tempAlign` and mint permanent `Align`; Burn correct validators' `tempRep` and mint permanent `Rep`
   - If rejected: Burn contributor's `tempAlign` and incorrect validators' `tempRep` are burned with no replacement.

---

## 4. Detailed Functional Requirements

### 4.1 Initialize Protocol & Token Mints

**Description**  
- Deploy a `GlobalState` account referencing four SPL token mints: `tempAlignMint`, `AlignMint`, `tempRepMint`, and `RepMint`.
- These mints are owned by **PDA authorities** so only the protocol can mint/burn them.

**Requirements**  
1. **GlobalState** must store:
   - `authority` (admin or DAO).
   - References to all four token mints.
   - Any global config fields (like burning rates, epoch times, etc.).
2. **init** logic to create the four mints with correct decimals and mint authorities:
   - `tempAlignMint` - Limited transferability 
   - `AlignMint` - Full transferability
   - `tempRepMint` - Non-transferable
   - `RepMint` - Non-transferable
3. Only the recognized `authority` can call `initialize`, to prevent re-initialization.

**Success Criteria**  
- `GlobalState` is created.
- The four SPL mints exist with program-derived authorities.

---

### 4.2 Submit Data & Mint Temporary Alignment Tokens

**Description**  
- Contributors upload data to be stored directly on-chain in the Submission account.
- Program creates a `Submission` account and mints `tempAlign` tokens to the contributor.

**Requirements**  
1. **Submission Account** contains:
   - `contributor` Pubkey.
   - `data` - String field to store the actual data on-chain.
   - Vote counters (`yes_count`, `no_count`).
   - Time window or epoch data for commit-reveal phases.
   - `status` enum (Pending, Accepted, Rejected).
2. `X` `tempAlign` tokens minted upon successful creation.

**Spam Mitigation** (Optional)  
- A small deposit or partial stake to create a submission might be required.

---

### 4.3 Stake Temporary Alignment Tokens for Temporary Reputation

**Description**  
- Users stake and burn `tempAlign` tokens in exchange for `tempRep` tokens.
- Only users with `tempRep` can participate in voting.

**Requirements**  
1. Must track staked amounts in a `UserProfile` account or similar structure.
2. Implement diminishing returns on large Rep accumulations to prevent governance centralization.
3. tempRep tokens should be non-transferable (soulbound).
4. Temporary tokens are burned if the user votes incorrectly.

**Outcome**  
- `tempRep` acts as a gating function for who can vote.
- More staked tokens = higher reputation, with diminishing returns.
- Quadratic voting ensures balanced governance influence.

---

### 4.4 Commit & Reveal Votes

**Description**  
- A two-phase voting process to hide the validator's vote until the reveal phase.

**Requirements**  
1. **Commit**:
   - Store `vote_hash = hash(submission_id, vote_choice, nonce, validator)`.
   - No one sees the actual vote choice.
2. **Reveal**:
   - Validator discloses `(vote_choice, nonce)`.
   - The program verifies it matches the earlier commit hash.
   - Tally votes in the `Submission` account: `yes_count` or `no_count`.

**Edge Cases**  
- Late reveal or no reveal → vote is invalid and tempRep tokens are burned.
- The program must strictly respect the time windows for commit and reveal.

---

### 4.5 Finalize Submission & Convert Temporary Tokens to Permanent Tokens

**Description**  
- Once the reveal phase ends, the protocol calculates the final outcome:
  - If "Accepted," the contributor's `tempAlign` tokens are burned and equivalent `Align` tokens are minted.
  - Validators with correct votes see their `tempRep` burned and equivalent `Rep` tokens minted.
  - If "Rejected," the contributor's `tempAlign` and incorrect validators' `tempRep` are burned with no replacement.

**Requirements**  
1. **Tally**: Compare `yes_count` and `no_count`.  
2. **Reward**: 
   - For accepted submissions: Burn contributor's `tempAlign` and mint equivalent `Align` tokens.
   - For correct validators: Burn their `tempRep` and mint equivalent `Rep` tokens.
3. **Burn**: 
   - Burn the temporary tokens of validators who voted opposite the outcome, with no replacement.
   - Burn the contributor's `tempAlign` tokens if submission is rejected, with no replacement.
4. **Status Update**: Mark submission as `Accepted` or `Rejected`.

---

## 5. Data & Account Structures

1. **GlobalState** (PDA)  
   - `authority: Pubkey`  
   - `temp_align_mint: Pubkey`  
   - `align_mint: Pubkey`  
   - `temp_rep_mint: Pubkey`  
   - `rep_mint: Pubkey`  
   - `bump: u8`  
   - Additional config fields (burn rate, etc.)

2. **Submission**  
   - `contributor: Pubkey`  
   - `timestamp: u64` 
   - `data: String` - Contains the actual data stored on-chain  
   - `yes_count: u64`  
   - `no_count: u64`  
   - `status: enum { Pending, Accepted, Rejected }`  
   - Timestamps for commit/reveal phases

3. **UserProfile**
   - `user: Pubkey`  
   - `temp_rep: u64` (tracking for UI purposes)
   - `permanent_rep: u64` (tracking for UI purposes)
   - Possibly store user's vote records, etc.

4. **Token Mints**  
   - `tempAlignMint` - Temporary alignment tokens (limited transferability)
   - `AlignMint` - Permanent alignment tokens (fully transferable)
   - `tempRepMint` - Temporary reputation tokens (non-transferable)
   - `RepMint` - Permanent reputation tokens (non-transferable)

5. **Vote Commit** (for commit-reveal voting)
   - `validator: Pubkey`
   - `submission: Pubkey`
   - `vote_hash: [u8; 32]`
   - `revealed: bool`

---

## 6. Non-Functional Requirements

1. **Performance**  
   - Each transaction must complete within a few seconds on Solana.
2. **Security**  
   - PDAs used for mint authorities.  
   - Commit-reveal ensures vote privacy until reveal phase.  
   - Token burning discourages bad behavior or collusion.
3. **Scalability**  
   - Must handle many submissions and votes without bogging down.
4. **Maintainability**  
   - Clear code with Anchor's macros, well-documented tests.

---

## 7. User Workflows & UI/UX

**1. Contributor Workflow**  
   - Open dApp or CLI.  
   - Upload data to IPFS.  
   - Call "submit_data" with IPFS hash → receive `tempAlign` tokens.

**2. Validator Workflow**  
   - Stake `tempAlign` tokens to receive `tempRep` tokens.  
   - Browse open submissions.  
   - **Commit** vote hash.  
   - After reveal phase opens, **Reveal** actual vote.  
   - Wait for finalization → Earn permanent tokens if correct.

**3. Admin / DAO**  
   - (Initially) Single authority sets parameters, can upgrade the program.  
   - (Eventually) DAO proposals to change the logic, reward rates, or burn parameters.

---

## 8. Testing & Validation Strategy

1. **Unit Tests**  
   - For each instruction: `initialize`, `submit_data`, `stake`, `commit_vote`, `reveal_vote`, `finalize`.
2. **Integration Tests**  
   - Full flow: Initialize → Submit → Vote → Finalize → Check token balances.
3. **Edge Cases**  
   - Reveal mismatch or missed reveal phase → burn tokens.  
   - Overlapping commit/reveal windows.
4. **Security Audits**  
   - Especially around PDAs, mint authorities, token conversion logic.

---

## 9. Milestones & Roadmap

1. **MVP Implementation**  
   - Step 1: `initialize` & token mints.  
   - Step 2: `submit_data` & basic data structure.  
   - Step 3: `stake_alignment_tokens`.  
   - Step 4: `commit_vote` & `reveal_vote`.  
   - Step 5: `finalize_submission` → reward or burn.

2. **Extended Features**  
   - Challenge/Dispute windows.  
   - Weighted or random subset voting.  
   - Bicameral DAO governance.  
   - Off-chain indexing or aggregator for data analytics.

3. **Production Rollout**  
   - Move from devnet to testnet/mainnet.  
   - Integrate a front-end for broader user adoption.  
   - Conduct user feedback sessions and refine.

---

## 10. Open Questions & Future Enhancements

1. **Reward Formulas**  
   - How many `tempAlign` tokens minted per submission? Is it static or dynamic?  
   - Any upper limit on permanent tokens minted per accepted submission?

2. **Burning Config**  
   - Fractional burn vs. full burn for incorrect votes.  
   - Dynamic burn rates based on token holdings?

3. **DAO Integration**  
   - When do we shift from a single authority to a decentralized governance model?  
   - Will the `Align` token also carry voting weight in governance?

4. **Spam & Sybil Resistance**  
   - Do we require a deposit or minimal stake to create new submissions?  
   - Rate-limiting or reputation gating for repeated submissions?

---

# End of Document
