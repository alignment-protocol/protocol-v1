# Alignment Protocol Implementation Checklist

## Legend

- ✅ Completed
- 🔄 Partially Implemented / In Progress
- ❌ Not Implemented
- 🔴 High Priority
- 🟠 Medium Priority
- 🟢 Low Priority

## 1. Initialize Protocol & Token Mints

| Status | Priority | Task                                                                                                                                             |
| :----: | :------: | ------------------------------------------------------------------------------------------------------------------------------------------------ |
|   ✅   |    -     | Create `State` account (PDA) - (`initialize_state`)                                                                                              |
|   ✅   |    -     | Store `authority`, `oracle_pubkey`, counts, defaults in `State`                                                                                  |
|   ✅   |    -     | Create four token mints (`tempAlignMint`, `AlignMint`, `tempRepMint`, `RepMint`) with program (`State` PDA) as authority - (`initialize_*_mint`) |
|   ✅   |    -     | Store mint pubkeys in `State`                                                                                                                    |
|   ✅   |    -     | Implement authority control for admin functions                                                                                                  |
|   ✅   |    -     | Add ability to update `tokens_to_mint` - (`update_tokens_to_mint`)                                                                               |
|   ❌   |    🟢    | Add support for eventual DAO governance for authority roles                                                                                      |

## 2. Topic Management

| Status | Priority | Task                                                                                                       |
| :----: | :------: | :--------------------------------------------------------------------------------------------------------- |
|   ✅   |    -     | Create `Topic` account (PDA) - (`create_topic`)                                                            |
|   ✅   |    -     | Store `name`, `description`, `authority`, counts, phase durations, `is_active` flag in `Topic`             |
|   ✅   |    -     | Increment `State.topic_count` on creation                                                                  |
|   ✅   |    -     | Authority-only topic creation _(initial implementation – now superseded by user-topic creation)_           |
|   ✅   |    🟢    | Update existing topics (phase durations, activity flag) - (`update_topic`)                                 |
|   ❌   |    🔴    | **Enable adding subtopics (parent-child relationship)**                                                    |
|   ❌   |    🔴    | &nbsp;&nbsp;└─ Define data structure for parent/child topic link                                           |
|   ❌   |    🔴    | &nbsp;&nbsp;└─ Implement instruction for creating subtopics                                                |
|   ❌   |    🔴    | &nbsp;&nbsp;└─ Update logic for Browse/linking submissions within subtopic hierarchy                       |
|   ✅   |    -     | **Allow users (non-authority) to create topics**                                                           |
|   ✅   |    -     | &nbsp;&nbsp;└─ Define rules/constraints for user topic creation _(any signer may now call `create_topic`)_ |
|   ✅   |    -     | &nbsp;&nbsp;└─ Implement instruction for user topic creation (`create_topic`)                              |

## 3. User Setup

| Status | Priority | Task                                                                                  |
| :----: | :------: | ------------------------------------------------------------------------------------- |
|   ✅   |    -     | Create `UserProfile` account (PDA) - (`create_user_profile`)                          |
|   ✅   |    -     | Store user key, submission counter, token account placeholders                        |
|   ✅   |    -     | Create protocol-owned `tempAlign` token account - (`create_user_temp_align_account`)  |
|   ✅   |    -     | Create protocol-owned `tempRep` token account - (`create_user_temp_rep_account`)      |
|   ✅   |    -     | Link temporary token accounts in `UserProfile`                                        |
|   ✅   |    -     | Create `UserTopicBalance` account (PDA) - (`initialize_user_topic_balance`)           |
|   ✅   |    -     | Store user, topic, and zeroed balances (`temp_align`, `temp_rep`, `locked_temp_rep`)  |
|   ✅   |    -     | Create user-owned permanent token ATAs (`Align`, `Rep`) via CPI - (`create_user_ata`) |
|   ✅   |    -     | Link permanent ATAs in `UserProfile`                                                  |

## 4. Submit Data & Link to Topics

| Status | Priority | Task                                                                                                                     |
| :----: | :------: | :----------------------------------------------------------------------------------------------------------------------- |
|   ✅   |    -     | Create `Submission` account (PDA) - (`submit_data_to_topic`)                                                             |
|   ✅   |    -     | Store `contributor`, `timestamp`, `data_reference` (String for off-chain data)                                           |
|   ✅   |    -     | Increment `UserProfile.user_submission_count`                                                                            |
|   ✅   |    -     | Create `SubmissionTopicLink` account (PDA) - (`submit_data_to_topic`, `link_submission_to_topic`)                        |
|   ✅   |    -     | Store `submission`, `topic`, initial `status` (Pending), phase timestamps, zeroed counters/powers                        |
|   ✅   |    -     | Increment `Topic.submission_count`                                                                                       |
|   ✅   |    -     | Mint topic-specific `tempAlign` tokens to `UserTopicBalance` - (`submit_data_to_topic`)                                  |
|   ✅   |    -     | Link existing `Submission` to another `Topic` without minting - (`link_submission_to_topic`)                             |
|   ❌   |    🟠    | **Enable adding sub-submissions (parent-child relationship for multi-turn data)**                                        |
|   ❌   |    🟠    | &nbsp;&nbsp;└─ Define data structure for parent/child submission link (e.g., `parent_submission` field in `Submission`?) |
|   ❌   |    🟠    | &nbsp;&nbsp;└─ Implement instruction for creating sub-submissions                                                        |
|   ❌   |    🟠    | &nbsp;&nbsp;└─ Update voting/finalization logic to potentially consider submission hierarchy                             |
|   ✅   |    -     | Enforce data reference validation (length & non-empty checks in `submit_data_to_topic`)                                  |
|   ❌   |    🟢    | Add optional spam prevention mechanism (e.g., stake requirement for submission)                                          |

## 5. Stake tempAlign Tokens for tempRep (Topic-Specific)

| Status | Priority | Task                                                                                   |
| :----: | :------: | -------------------------------------------------------------------------------------- |
|   ✅   |    -     | Implement `stake_topic_specific_tokens` instruction                                    |
|   ✅   |    -     | Burn topic-specific `tempAlign` from `UserTopicBalance` via CPI                        |
|   ✅   |    -     | Mint topic-specific `tempRep` to `UserTopicBalance` via CPI                            |
|   ✅   |    -     | Update `temp_align_amount` and `temp_rep_amount` in `UserTopicBalance`                 |
|   ✅   |    -     | Implement reputation accounting logic (direct 1:1 conversion)                          |
|   ❌   |    🟠    | Add staking period/lockup functionality (beyond vote locking)                          |
|   ❌   |    🟠    | Implement diminishing returns or alternative reputation calculation (currently linear) |

## 6. Commit & Reveal Votes (Topic-Specific)

| Status | Priority | Task                                                                                                |
| :----: | :------: | --------------------------------------------------------------------------------------------------- |
|   ✅   |    -     | Implement two-phase voting process on `SubmissionTopicLink`                                         |
|   ✅   |    -     | Create `VoteCommit` account (PDA) - (`commit_vote`)                                                 |
|   ✅   |    -     | Store link, validator, `vote_hash`, timestamps, `vote_amount`, `is_permanent_rep` flag              |
|   ✅   |    -     | Lock `tempRep` tokens (`locked_temp_rep_amount` in `UserTopicBalance`) during commit                |
|   ✅   |    -     | Handle voting with permanent `Rep` tokens (check ATA balance) - (`commit_vote`)                     |
|   ✅   |    -     | Increment `total_committed_votes` in `SubmissionTopicLink`                                          |
|   ✅   |    -     | Enforce commit phase time window                                                                    |
|   ✅   |    -     | Prevent self-voting (validator cannot vote on own submissions)                                      |
|   ✅   |    -     | Implement `reveal_vote` instruction                                                                 |
|   ✅   |    -     | Verify hash against stored `vote_hash`                                                              |
|   ✅   |    -     | Update `VoteCommit` status (`revealed`, `vote_choice`)                                              |
|   ✅   |    -     | Calculate voting power (quadratic assumed) and add to `SubmissionTopicLink` (`yes/no_voting_power`) |
|   ✅   |    -     | Increment `total_revealed_votes` in `SubmissionTopicLink`                                           |
|   ✅   |    -     | Enforce reveal phase time window                                                                    |
|   ✅   |    -     | Handle edge cases (missed reveals) during finalization                                              |
|   ✅   |    -     | Allow authority to manually set voting phases (`set_voting_phases`)                                 |

## 7. Finalize Submission & Votes (Topic-Specific)

| Status | Priority | Task                                                                                                                                                                                                                |
| :----: | :------: | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
|   🔄   |    -     | **Implement `finalize_submission` instruction (callable by anyone after reveal phase) - _(Requires Rework)_**                                                                                                       |
|   ✅   |    -     | &nbsp;&nbsp;└─ Determine outcome (`Accepted`/`Rejected`) based _initially_ on `yes/no_voting_power` (human votes only)                                                                                              |
|   ✅   |    -     | &nbsp;&nbsp;└─ Update `SubmissionTopicLink.status` based on human vote outcome _if no AI validation requested_                                                                                                      |
|   ✅   |    -     | &nbsp;&nbsp;└─ If Accepted: Burn contributor's topic `tempAlign` and mint permanent `Align` to ATA (_if no AI validation_)                                                                                          |
|   ✅   |    -     | &nbsp;&nbsp;└─ If Rejected: Burn contributor's topic `tempAlign` with no replacement (_if no AI validation_)                                                                                                        |
|   ❌   |    🔴    | &nbsp;&nbsp;└─ **New Logic for AI Integration:**                                                                                                                                                                    |
|   ❌   |    🔴    | &nbsp;&nbsp;&nbsp;&nbsp;└─ Fetch relevant `AiValidationRequest` account(s).                                                                                                                                         |
|   ❌   |    🔴    | &nbsp;&nbsp;&nbsp;&nbsp;└─ If AI validation exists:                                                                                                                                                                 |
|   ❌   |    🔴    | &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;└─ Calculate `ai_power` from `AiValidationRequest.temp_rep_staked` (using `calculate_ai_voting_power` logic).                                                                   |
|   ❌   |    🔴    | &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;└─ Calculate combined `total_yes = human_yes + ai_power (if ai=Yes)` and `total_no = human_no + ai_power (if ai=No)`.                                                           |
|   ❌   |    🔴    | &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;└─ Determine `final_status` (Accepted/Rejected) based on comparing `total_yes` vs `total_no`.                                                                                   |
|   ❌   |    🔴    | &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;└─ Update `SubmissionTopicLink.status` with this `final_status`.                                                                                                                |
|   ❌   |    🔴    | &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;└─ Process contributor's `tempAlign` (burn/mint `Align`) based on this `final_status`.                                                                                          |
|   ❌   |    🔴    | &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;└─ Update `AiValidationRequest` status (e.g., `Finalized`).                                                                                                                     |
|   ❌   |    🔴    | &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;└─ Mark staked `tempRep` in `AiValidationRequest` as `Returned` (if `final_status == ai_decision`) or `Claimable` (if `final_status != ai_decision`).                           |
|   ✅   |    -     | **Implement `finalize_vote` instruction (callable by anyone after submission finalization)**                                                                                                                        |
|   ✅   |    -     | &nbsp;&nbsp;└─ Process validator rewards/penalties based on `VoteCommit.vote_choice` vs `SubmissionTopicLink.final_status` _(Ensure this uses the final, potentially AI-influenced, status)_                        |
|   ✅   |    -     | &nbsp;&nbsp;└─ If correct (`tempRep` vote): Burn locked `tempRep`, mint permanent `Rep` to ATA                                                                                                                      |
|   ✅   |    -     | &nbsp;&nbsp;└─ If incorrect (`tempRep` vote): Burn locked `tempRep` with no replacement                                                                                                                             |
|   ✅   |    🟠    | &nbsp;&nbsp;└─ If correct (`Rep` vote): Return/handle escrowed `Rep` (Needs clarification/refinement)                                                                                                               |
|   ✅   |    🟠    | &nbsp;&nbsp;└─ If incorrect (`Rep` vote): Burn/handle escrowed `Rep` (Needs clarification/refinement)                                                                                                               |
|   ✅   |    -     | &nbsp;&nbsp;└─ Update `VoteCommit.finalized` status                                                                                                                                                                 |
|   ❌   |    🔴    | **Implement `claim_ai_stake` instruction (callable by eligible human validators)**                                                                                                                                  |
|   ❌   |    🔴    | &nbsp;&nbsp;└─ Allow validator whose `VoteCommit.vote_choice` matches the `final_status` to claim `tempRep` from the `AiValidationRequest` _only if_ `final_status != ai_decision` and stake is marked `Claimable`. |
|   ❌   |    🟠    | &nbsp;&nbsp;└─ Define and implement distribution logic for `claim_ai_stake` (proportional, equal split, first-come?).                                                                                               |

## 8. AI Validation (Optional)

| Status | Priority | Task                                                                                                                                                                                                                                                                    |
| :----: | :------: | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|   ✅   |    -     | **Implement `request_ai_validation` instruction (callable by contributor)**                                                                                                                                                                                             |
|   ✅   |    -     | &nbsp;&nbsp;└─ Lock contributor's `tempRep` from `UserTopicBalance`                                                                                                                                                                                                     |
|   ✅   |    -     | &nbsp;&nbsp;└─ Create `AiValidationRequest` account (PDA) - _(See multiple request handling below)_                                                                                                                                                                     |
|   ✅   |    -     | &nbsp;&nbsp;└─ Store link, requester, `temp_rep_staked`, timestamp, initial status (Pending)                                                                                                                                                                            |
|   🔄   |    -     | **Implement `submit_ai_vote` instruction (callable by `oracle_pubkey`) - _(Logic Change)_**                                                                                                                                                                             |
|   ✅   |    -     | &nbsp;&nbsp;└─ Verify caller signature                                                                                                                                                                                                                                  |
|   ✅   |    -     | &nbsp;&nbsp;└─ Update `AiValidationRequest` status, `ai_decision`.                                                                                                                                                                                                      |
|   ❌   |    -     | &nbsp;&nbsp;└─ **Does NOT directly modify `SubmissionTopicLink.yes/no_voting_power` or `ai_voting_power` anymore.** (Weight applied during `finalize_submission`)                                                                                                       |
|   ❌   |    🟠    | **Allow multiple AI validation requests per SubmissionTopicLink (using User-Specific Counter)**                                                                                                                                                                         |
|   ❌   |    🟠    | &nbsp;&nbsp;└─ Add `user_ai_request_count: u64` to `UserTopicBalance` struct                                                                                                                                                                                            |
|   ❌   |    🟠    | &nbsp;&nbsp;└─ Update `InitializeUserTopicBalance` context space allocation for the new counter (+8 bytes)                                                                                                                                                              |
|   ❌   |    🟠    | &nbsp;&nbsp;└─ Update `RequestAiValidation` context seeds to `[b"ai_request", link.key(), requester.key(), expected_index.to_le_bytes()]`                                                                                                                               |
|   ❌   |    🟠    | &nbsp;&nbsp;└─ Update `RequestAiValidation` instruction to take `expected_ai_request_index` (from client reading `user_topic_balance.user_ai_request_count`), store it in `AiValidationRequest`, and increment `user_topic_balance.user_ai_request_count` upon success. |
|   ❌   |    🟠    | &nbsp;&nbsp;└─ Update client to fetch `UserTopicBalance`, read `user_ai_request_count`, pass it as `expected_ai_request_index` argument, and derive the correct PDA.                                                                                                    |
|   🔄   |    🟠    | Clarify handling of contributor's staked `tempRep` in `AiValidationRequest` - _(Now handled via `finalize_submission` marking as Returned/Claimable, and new `claim_ai_stake` instruction)_                                                                             |
|   ❌   |    🟠    | Define `calculate_ai_voting_power` function logic (e.g., linear, quadratic based on `temp_rep_staked`).                                                                                                                                                                 |

## 9. Testing & Validation

| Status | Priority | Task                                                                                                                                   |
| :----: | :------: | -------------------------------------------------------------------------------------------------------------------------------------- |
|   ✅   |    -     | Unit tests for initialize instructions (`initialize_state`, `initialize_*_mint`)                                                       |
|   ✅   |    -     | Unit tests for topic management (`create_topic`)                                                                                       |
|   ✅   |    -     | Unit tests for user setup (`create_user_profile`, `create_user_*_account`, `initialize_user_topic_balance`, `create_user_ata`)         |
|   ✅   |    -     | Unit tests for submission/linking (`submit_data_to_topic`, `link_submission_to_topic`)                                                 |
|   ✅   |    -     | Unit tests for staking (`stake_topic_specific_tokens`)                                                                                 |
|   ✅   |    -     | Unit tests for voting (`commit_vote`, `reveal_vote`)                                                                                   |
|   ✅   |    -     | Unit tests for finalization (`finalize_submission`, `finalize_vote`)                                                                   |
|   ✅   |    -     | Unit tests for cross-topic submission linking (`link_submission_to_topic`)                                                             |
|   ✅   |    -     | Unit tests for AI validation (`request_ai_validation`, `submit_ai_vote`) - _(May need updates for new logic)_                          |
|   ✅   |    -     | End-to-end tests with basic single-topic workflow (`01` to `08` in `tests/sections/`)                                                  |
|   🔄   |    -     | End-to-end tests covering advanced features (`09-token-locking-tests.ts`, `10-validation-tests.ts`) - _(Needs updates for AI changes)_ |
|   ❌   |    🔴    | Tests with multiple concurrent contributors and validators interacting within/across topics                                            |
|   ❌   |    🔴    | Tests for new features (subtopics, user topic creation, sub-submissions)                                                               |
|   ❌   |    🟠    | Tests for AI validation update (multiple requests)                                                                                     |
|   ❌   |    🔴    | Tests for updated `finalize_submission` logic (combined human/AI power scenarios).                                                     |
|   ❌   |    🔴    | Tests for `claim_ai_stake` instruction (eligibility, distribution).                                                                    |
|   ❌   |    🟢    | Formal Security audits (especially token handling, PDA authorities, oracle interaction, permanent Rep voting, AI stake handling)       |
|   ❌   |    🟢    | Performance / Load testing                                                                                                             |

## 10. Client/UI Development

| Status | Priority | Task                                                                                                                                     |
| :----: | :------: | :--------------------------------------------------------------------------------------------------------------------------------------- |
|   🔄   |    -     | CLI tool implementation (basic framework exists) - (`cli/`)                                                                              |
|   ✅   |    -     | **CLI commands for all current protocol functions:**                                                                                     |
|   ✅   |    -     | &nbsp;&nbsp;└─ `initialize_state`, `initialize_*_mint`, `update_tokens_to_mint`                                                          |
|   ✅   |    -     | &nbsp;&nbsp;└─ `create_topic`                                                                                                            |
|   ✅   |    -     | &nbsp;&nbsp;└─ `create_user_profile`, `create_user_*_account`, `initialize_user_topic_balance`, `create_user_ata`                        |
|   ✅   |    -     | &nbsp;&nbsp;└─ `submit_data_to_topic`                                                                                                    |
|   ✅   |    -     | &nbsp;&nbsp;└─ `link_submission_to_topic`                                                                                                |
|   ✅   |    -     | &nbsp;&nbsp;└─ `stake_topic_specific_tokens`                                                                                             |
|   ✅   |    -     | &nbsp;&nbsp;└─ `commit_vote`                                                                                                             |
|   ✅   |    -     | &nbsp;&nbsp;└─ `reveal_vote`                                                                                                             |
|   ✅   |    -     | &nbsp;&nbsp;└─ `finalize_submission`                                                                                                     |
|   ✅   |    -     | &nbsp;&nbsp;└─ `finalize_vote`                                                                                                           |
|   ✅   |    -     | &nbsp;&nbsp;└─ `request_ai_validation`                                                                                                   |
|   ✅   |    -     | &nbsp;&nbsp;└─ `submit_ai_vote` (for testing/oracle simulation)                                                                          |
|   ❌   |    🔴    | **CLI commands for new protocol functions:**                                                                                             |
|   ❌   |    🔴    | &nbsp;&nbsp;└─ Subtopic creation / management                                                                                            |
|   ❌   |    🔴    | &nbsp;&nbsp;└─ User topic creation                                                                                                       |
|   ❌   |    🔴    | &nbsp;&nbsp;└─ Sub-submission creation / management                                                                                      |
|   ❌   |    🔴    | &nbsp;&nbsp;└─ `claim_ai_stake`                                                                                                          |
|   🔄   |    🟠    | **CLI "explorer" functionality:**                                                                                                        |
|   🔄   |    🟠    | &nbsp;&nbsp;└─ Browse topics (needs hierarchy support)                                                                                   |
|   🔄   |    🟠    | &nbsp;&nbsp;└─ Browse submissions (needs hierarchy support)                                                                              |
|   ✅   |    🟠    | &nbsp;&nbsp;└─ View user profiles and token balances (`UserProfile`, `UserTopicBalance`, ATAs)                                           |
|   ✅   |    🟠    | &nbsp;&nbsp;└─ View `AiValidationRequest` details.                                                                                       |
|   ❌   |    🟠    | &nbsp;&nbsp;└─ View network stats (`State`, aggregate topic/submission counts)                                                           |
|   ❌   |    🟠    | **Ensure protocol state structures and events are designed for efficient off-chain querying and indexing by applications (like Vedas).** |
|   ✅   |    -     | Deploy protocol to devnet for testing (Address exists)                                                                                   |
|   ❌   |    🟠    | Web UI/dApp for user-friendly interaction                                                                                                |
|   ❌   |    🟠    | Wallet integration for UI/dApp                                                                                                           |

## 11. Documentation & Non-Functional Requirements

| Status | Priority | Task                                                                   |
| :----: | :------: | ---------------------------------------------------------------------- |
|   ✅   |    -     | Core Program Logic documented via code comments                        |
|   ✅   |    -     | PRD Document (`prd.md`) - Updated                                      |
|   🔄   |    -     | Checklist Document (`checklist.md`) - Updated                          |
|   ✅   |    -     | Basic Test flow documentation (`tests/README.md`)                      |
|   ✅   |    -     | Diagrams & Visual Documentation (Token flow, Basic Workflow)           |
|   🔄   |    -     | Program efficiency and optimization                                    |
|   ❌   |    🟠    | API documentation / SDK documentation for integrators                  |
|   ❌   |    🟠    | User guides (for contributors, validators via CLI/UI)                  |
|   ❌   |    🟠    | Developer documentation (setup, architecture, contribution guidelines) |
|   ❌   |    🟢    | Comprehensive error handling and logging                               |
|   ❌   |    🟢    | Scalability analysis and potential improvements                        |
|   ❌   |    🟢    | Security hardening beyond basic audit fixes                            |
|   ❌   |    🟢    | Monitoring and on-chain analytics integration                          |

## 12. Milestones & Roadmap (Post-Hackathon)

### Milestone 1: Core Topic-Based Protocol (✅ Achieved)

- ✅ Foundational instructions: Init, Topics, Users, Submit, Stake, Vote, Finalize
- ✅ Basic cross-topic linking
- ✅ Basic AI Validation integration _(Initial version)_
- ✅ Core data structures implemented
- ✅ Basic CLI commands and unit/integration tests

### Milestone 2: Advanced Features & Refinements (🔄 In Progress / Next)

- 🔴 Implement Subtopics & User Topic Creation
- 🟠 Implement Sub-submissions
- 🟠 Refine Permanent Rep Voting Mechanics (Escrow/Rewards/Slashing)
- 🟠 Implement Multiple AI Requests per Submission
- 🔴 Refine AI Validation Integration (Separate recording, Combined Finalization, Stake Claiming)
- 🟠 Clarify AI Request TempRep Handling - _(Addressed by new flow)_
- 🟠 Explore Spam/Sybil Resistance Mechanisms
- 🟠 Explore Advanced Staking/Reputation Mechanics (Lockups, Diminishing Returns)

### Milestone 3: Production Readiness (❌ Not Started)

- 🔴 Complete CLI Tool (All functions + Explorer)
- 🟠 Develop User Interface (Web dApp)
- 🟠 Comprehensive Testing (Multi-user, Load, Edge Cases)
- 🟠 Formal Security Audit & Fixes
- 🟠 Finalize Documentation (API, User Guides, Dev Docs)
- 🟢 Plan and Execute Testnet Deployment
- 🟢 Plan and Execute Mainnet Deployment & Community Onboarding

## 13. Open Questions & Future Enhancements (To Investigate / Implement)

- ✅ Confirmed quadratic voting power calculation for human votes (implemented in `calculate_quadratic_voting_power`)
- 🟠 Define AI voting power calculation (`calculate_ai_voting_power` function in Sec 8)
- 🟠 Finalize permanent Rep voting rewards/slashing/escrow mechanism
- 🟠 Define distribution logic for `claim_ai_stake` (See Sec 7)
- 🟠 Define specific burning/slashing rules (Confirm amounts/destinations)
- 🟢 Plan DAO integration strategy & scope
- 🟢 Investigate spam/Sybil resistance options (Staking, Rate Limiting)
- 🟢 Define Topic lifecycle rules (Archiving, Limits)
- 🟢 Refine Tokenomics (Value accrual, Transferability)
- 🟢 Explore Challenge/Dispute windows
- 🟢 Explore Weighted/Random subset voting
- 🟢 Explore Off-chain indexing/aggregator needs

---
