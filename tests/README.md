# Alignment Protocol Tests

This directory contains tests for the Alignment Protocol.

## Test Structure

The tests are organized into sections to make them more maintainable:

- `utils/` - Utility functions and test setup
  - `test-setup.ts` - Common setup and context for all tests
  - `constants.ts` - Shared constants

- `sections/` - Individual test sections
  - `01-initialization.ts` - Protocol initialization tests
  - `02-topic-management.ts` - Topic creation and management
  - `03-user-setup.ts` - User profile and token account creation
  - `04-submission.ts` - Submission creation
  - `05-cross-topic-linking.ts` - Cross-topic linking
  - `06-staking.ts` - Token staking
  - `07-voting.ts` - Vote commitment and reveal
  - `08-finalization.ts` - Submission and vote finalization

- `alignment-protocol.ts` - Main test runner that imports all test sections
- `alignment-protocol.orig.ts` - Original monolithic test file (kept for reference)

## Running Tests

To run the tests:

```
anchor test
# or
npm test
```

The test runner will automatically manage the Solana validator for you.

Note: The tests are sequential and depend on each other, so they must be run in order. Individual test files cannot be run on their own since each test section builds on the state created by previous sections.

## Test Flow

The tests simulate the complete protocol workflow, focusing on two user types (contributors and validators) and their interactions:

1. **Protocol Initialization** (01-initialization.ts)
   - **Protocol Authority** initializes the base protocol:
     - Creates the state account (PDA) with default 24-hour commit/reveal phases
     - Initializes four token mints as PDAs:
       - Temporary ALIGN token mint (for contributors pending finalization)
       - Permanent ALIGN token mint (for accepted submissions)
       - Temporary REP token mint (for validators during voting)
       - Permanent REP token mint (for validated votes)
     - Configures tokens_to_mint parameter (100 tokens per submission)
     - Sets state PDA as mint and freeze authority for all tokens

2. **Topic Management** (02-topic-management.ts)
   - **Protocol Authority** creates topic infrastructure:
     - Creates topic accounts using the create_topic instruction
     - Configures topic-specific commit and reveal phase durations
     - Topics are uniquely identified by PDAs (seeds: "topic" + topic_count)
     - Each topic serves as a category for submissions and voting

3. **User Setup** (03-user-setup.ts)
   - Both **Contributors** and **Validators** create their profiles:
     - User profile PDAs store participation history and token balances
   - For each user type, protocol establishes:
     - Protocol-owned temporary token accounts (state PDA is owner)
       - For **Contributors**: To receive temporary ALIGN rewards for submissions
       - For **Validators**: To stake temporary ALIGN and receive temporary REP
     - User-owned permanent token accounts (Associated Token Accounts)
       - For **Contributors**: To receive permanent ALIGN for accepted submissions
       - For **Validators**: To receive permanent REP for correct votes

4. **Submission Creation** (04-submission.ts)
   - **Contributors** submit data to topics:
     - Create submissions through the create_submission instruction 
     - Submission data is stored in submission PDAs
     - Submission-topic links are created to categorize submissions
     - **Contributors** receive temporary ALIGN tokens in protocol-owned accounts
     - Token amounts (100 per submission) are tracked in user profiles

5. **Cross-Topic Linking** (05-cross-topic-linking.ts)
   - **Contributors** link their existing submissions to additional topics:
     - Create additional submission-topic link PDAs
     - This allows submissions to appear in multiple topic categories
     - The original submission account is updated with link information
     - No additional tokens are minted for cross-topic links

6. **Token Staking** (06-staking.ts)
   - **Validators** stake temporary ALIGN tokens to participate in voting:
     - Acquire temporary ALIGN tokens (simulation gives them some)
     - Stake temp ALIGN through the stake_temp_align instruction
     - Temporary ALIGN is burned from validator's protocol-owned account
     - Temporary REP is minted to validator's protocol-owned account
     - 1:1 staking ratio ensures proportional voting power
     - Staking prepares validators to participate in the voting process

7. **Voting** (07-voting.ts)
   - **Validators** participate in two-phase voting:
     - Commit phase:
       - Submit encrypted vote commitments (hash of choice + secret nonce)
       - Vote power is locked based on temp REP tokens at time of commitment
     - Reveal phase:
       - Reveal actual vote choice (YES/NO) with nonce to verify commitment
       - Vote power is applied quadratically (square root of token amount)
       - Each validator's vote updates the submission's vote tallies

8. **Finalization** (08-finalization.ts)
   - Protocol processes final outcomes:
     - For **Contributors**:
       - If submission is accepted (more YES than NO votes):
         - Temporary ALIGN tokens are converted to permanent ALIGN
         - Permanent tokens are sent to contributor's personal ATA
       - If submission is rejected:
         - Temporary tokens are burned without replacement
     - For **Validators**:
       - If they voted correctly (with majority):
         - Temporary REP tokens are converted to permanent REP
         - Permanent REP is recorded in user profile and sent to ATA
       - If they voted incorrectly:
         - Temporary REP tokens are burned without replacement

Each step builds on the previous ones, creating a complete end-to-end test that simulates the full user journey for both contributors and validators. The tests verify both the technical functionality and the token economic incentives of the protocol, showing how users are rewarded for positive contributions and accurate validation.