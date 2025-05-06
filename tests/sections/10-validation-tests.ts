import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { web3, BN } from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { TestContext } from "../utils/test-setup";
import * as crypto from "crypto";

// Helper to create a vote hash
function createVoteHash(
  voter: web3.Keypair,
  submissionTopicLink: web3.PublicKey,
  choice: number, // 0 = Yes, 1 = No
  nonce: string,
): number[] {
  const message = Buffer.concat([
    voter.publicKey.toBuffer(),
    submissionTopicLink.toBuffer(),
    Buffer.from([choice]),
    Buffer.from(nonce),
  ]);
  return Array.from(crypto.createHash("sha256").update(message).digest());
}

// Helper function to set voting phases
async function setupVotingPhase(
  ctx: TestContext,
  phase: "commit" | "reveal" | "finalized" | "premature", // Added premature for testing finalization edge case
  submissionTopicLinkPda: web3.PublicKey,
  submissionPdaToUse: web3.PublicKey,
) {
  const now = Math.floor(Date.now() / 1000);
  let commitPhaseStart, commitPhaseEnd, revealPhaseStart, revealPhaseEnd;

  if (phase === "commit") {
    commitPhaseStart = now - 60;
    commitPhaseEnd = now + 600;
    revealPhaseStart = commitPhaseEnd;
    revealPhaseEnd = revealPhaseStart + 600;
  } else if (phase === "reveal") {
    commitPhaseStart = now - 1200;
    commitPhaseEnd = now - 60;
    revealPhaseStart = commitPhaseEnd;
    revealPhaseEnd = now + 600;
  } else if (phase === "finalized") {
    commitPhaseStart = now - 2400;
    commitPhaseEnd = now - 1800;
    revealPhaseStart = commitPhaseEnd;
    revealPhaseEnd = now - 60; // Ended in the past
  } else {
    // premature (reveal phase hasn't ended yet)
    commitPhaseStart = now - 2400;
    commitPhaseEnd = now - 1800;
    revealPhaseStart = commitPhaseEnd;
    revealPhaseEnd = now + 600; // Ends in the future
  }

  console.log(
    `Setting phases for ${phase}: Commit ${commitPhaseStart}-${commitPhaseEnd}, Reveal ${revealPhaseStart}-${revealPhaseEnd} on Link ${submissionTopicLinkPda.toBase58()}`,
  );
  const setPhasesTx = await ctx.program.methods
    .setVotingPhases(
      new anchor.BN(commitPhaseStart),
      new anchor.BN(commitPhaseEnd),
      new anchor.BN(revealPhaseStart),
      new anchor.BN(revealPhaseEnd),
    )
    .accounts({
      state: ctx.statePda,
      submissionTopicLink: submissionTopicLinkPda,
      topic: ctx.topic1Pda,
      submission: submissionPdaToUse,
      authority: ctx.authorityKeypair.publicKey,
      systemProgram: web3.SystemProgram.programId,
    })
    .signers([ctx.authorityKeypair])
    .rpc();
  console.log(` -> Set phases TX: ${setPhasesTx}`);
  // Short delay to allow clock changes to propagate if needed, although usually not necessary in local tests
  await new Promise((resolve) => setTimeout(resolve, 500));
  return { commitPhaseStart, commitPhaseEnd, revealPhaseStart, revealPhaseEnd };
}

export function runValidationTests(ctx: TestContext): void {
  describe("Vote Validation and Error Handling", () => {
    // PDAs specific to this test suite will be assigned in `before`
    // let validationSubmissionPda: web3.PublicKey; // Now in ctx
    // let validationSubmissionTopicLinkPda: web3.PublicKey; // Now in ctx
    // let validationVoteCommitPda: web3.PublicKey; // Now in ctx

    before("Create a new submission for validation tests", async () => {
      console.log("--- Setting up for Validation Tests ---");
      const submissionData = "Submission for validation test suite";

      // Use contributor to create the submission
      const user = ctx.contributorKeypair;
      const userProfilePda = ctx.contributorProfilePda;
      const userTopicBalancePda = ctx.contributorTopic1BalancePda;
      const userTempAlignAccount = ctx.contributorTempAlignAccount;
      const userRepAta = ctx.contributorRepAta; // Needed for self-vote test accounts

      // Get user's current submission index
      const userProfile =
        await ctx.program.account.userProfile.fetch(userProfilePda);
      const submissionIndex = userProfile.userSubmissionCount;
      console.log(
        `Creating validation submission for ${user.publicKey.toBase58()} (index ${submissionIndex.toNumber()})`,
      );

      // Derive PDAs for this new submission
      [ctx.validationSubmissionPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("submission"),
          user.publicKey.toBuffer(),
          submissionIndex.toBuffer("le", 8),
        ],
        ctx.program.programId,
      );
      [ctx.validationSubmissionTopicLinkPda] =
        web3.PublicKey.findProgramAddressSync(
          [
            Buffer.from("submission_topic_link"),
            ctx.validationSubmissionPda.toBuffer(),
            ctx.topic1Pda.toBuffer(),
          ],
          ctx.program.programId,
        );

      console.log(
        ` -> Validation Submission PDA: ${ctx.validationSubmissionPda.toBase58()}`,
      );
      console.log(
        ` -> Validation Link PDA: ${ctx.validationSubmissionTopicLinkPda.toBase58()}`,
      );

      // Create the submission
      const tx = await ctx.program.methods
        .submitDataToTopic(submissionData, submissionIndex)
        .accounts({
          topic: ctx.topic1Pda,
          tempAlignMint: ctx.tempAlignMintPda,
          contributor: user.publicKey,
          payer: ctx.authorityKeypair.publicKey,
        })
        .signers([ctx.authorityKeypair])
        .rpc();
      console.log(" -> Created validation submission TX:", tx);

      // Verify it's pending
      const link = await ctx.program.account.submissionTopicLink.fetch(
        ctx.validationSubmissionTopicLinkPda,
      );
      expect(link.status.pending).to.not.be.undefined;

      // Ensure validator has rep tokens
      const validatorBalance = await ctx.program.account.userTopicBalance.fetch(
        ctx.validatorTopic1BalancePda,
      );
      expect(validatorBalance.tempRepAmount.toNumber()).to.be.greaterThan(0);

      // Set initial phase to commit
      await setupVotingPhase(
        ctx,
        "commit",
        ctx.validationSubmissionTopicLinkPda,
        ctx.validationSubmissionPda,
      );
    });

    it("Prevents self-voting on own submissions", async () => {
      // validationSubmission was created by contributor
      const user = ctx.contributorKeypair;
      const userProfilePda = ctx.contributorProfilePda;
      const userTopicBalancePda = ctx.contributorTopic1BalancePda;
      const userRepAta = ctx.contributorRepAta;
      const nonce = ctx.VOTE_NONCE_VALIDATION + "self";
      const selfVoteHash = createVoteHash(
        user,
        ctx.validationSubmissionTopicLinkPda,
        0,
        nonce,
      );

      const [selfVoteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          ctx.validationSubmissionTopicLinkPda.toBuffer(),
          user.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      console.log("Attempting self-vote (should fail)...");
      try {
        await ctx.program.methods
          .commitVote(selfVoteHash, new BN(1), false)
          .accounts({
            topic: ctx.topic1Pda,
            submission: ctx.validationSubmissionPda, // Submission created by contributor
            validatorRepAta: userRepAta, // Contributor's rep ATA
            validator: user.publicKey, // Contributor is the signer
            payer: ctx.authorityKeypair.publicKey,
          })
          .signers([ctx.authorityKeypair])
          .rpc();
        expect.fail("Self-voting should have been rejected");
      } catch (error) {
        console.log(" -> Received expected error:", error.message);
        // The constraint validator.key() != submission.contributor is checked directly in commitVote
        expect(error.error.errorCode.code).to.equal("SelfVotingNotAllowed");
        expect(error.error.errorMessage).to.include(
          "Self-voting is not allowed",
        );
      }
    });

    it("Prevents voting with 0 tokens", async () => {
      const nonce = ctx.VOTE_NONCE_VALIDATION + "zero";
      const zeroVoteHash = createVoteHash(
        ctx.validatorKeypair,
        ctx.validationSubmissionTopicLinkPda,
        0,
        nonce,
      );
      const [zeroVoteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          ctx.validationSubmissionTopicLinkPda.toBuffer(),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      console.log("Attempting vote with 0 tokens (should fail)...");
      try {
        await ctx.program.methods
          .commitVote(zeroVoteHash, new BN(0), false)
          .accounts({
            topic: ctx.topic1Pda,
            submission: ctx.validationSubmissionPda,
            validatorRepAta: ctx.validatorRepAta,
            validator: ctx.validatorKeypair.publicKey,
            payer: ctx.authorityKeypair.publicKey,
          })
          .signers([ctx.authorityKeypair])
          .rpc();
        expect.fail("Voting with 0 tokens should have been rejected");
      } catch (error) {
        console.log(" -> Received expected error:", error.message);
        expect(error.error.errorCode.code).to.equal("ZeroVoteAmount");
        expect(error.error.errorMessage).to.include(
          "Vote amount must be greater than zero",
        );
      }
    });

    it("Prevents voting with insufficient tokens", async () => {
      const nonce = ctx.VOTE_NONCE_VALIDATION + "insufficient";
      const insufficientVoteHash = createVoteHash(
        ctx.validatorKeypair,
        ctx.validationSubmissionTopicLinkPda,
        0,
        nonce,
      );
      const [insufficientVoteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          ctx.validationSubmissionTopicLinkPda.toBuffer(),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      const balance = await ctx.program.account.userTopicBalance.fetch(
        ctx.validatorTopic1BalancePda,
      );
      const availableTokens = balance.tempRepAmount.toNumber();
      const excessAmount = availableTokens + 1;
      console.log(
        `Attempting vote with ${excessAmount} tokens (available: ${availableTokens}) (should fail)...`,
      );

      try {
        await ctx.program.methods
          .commitVote(insufficientVoteHash, new BN(excessAmount), false)
          .accounts({
            topic: ctx.topic1Pda,
            submission: ctx.validationSubmissionPda,
            validatorRepAta: ctx.validatorRepAta,
            validator: ctx.validatorKeypair.publicKey,
            payer: ctx.authorityKeypair.publicKey,
          })
          .signers([ctx.authorityKeypair])
          .rpc();
        expect.fail("Voting with insufficient tokens should be rejected");
      } catch (error) {
        console.log(" -> Received expected error:", error.message);
        // commitVote uses NoReputationForTopic when checking UserTopicBalance specifically
        expect(error.error.errorCode.code).to.equal("NoReputationForTopic");
        expect(error.error.errorMessage).to.include(
          "Validator has no reputation tokens for this topic",
        ); // Or similar based on actual error
      }
    });

    it("Prevents committing votes during reveal phase", async () => {
      await setupVotingPhase(
        ctx,
        "reveal",
        ctx.validationSubmissionTopicLinkPda,
        ctx.validationSubmissionPda,
      );

      const nonce = ctx.VOTE_NONCE_VALIDATION + "commit-in-reveal";
      const wrongPhaseVoteHash = createVoteHash(
        ctx.validatorKeypair,
        ctx.validationSubmissionTopicLinkPda,
        0,
        nonce,
      );
      const [wrongPhaseVoteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          ctx.validationSubmissionTopicLinkPda.toBuffer(),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      console.log(
        "Attempting to commit vote during reveal phase (should fail)...",
      );
      try {
        await ctx.program.methods
          .commitVote(wrongPhaseVoteHash, new BN(1), false)
          .accounts({
            topic: ctx.topic1Pda,
            submission: ctx.validationSubmissionPda,
            validatorRepAta: ctx.validatorRepAta,
            validator: ctx.validatorKeypair.publicKey,
            payer: ctx.authorityKeypair.publicKey,
          })
          .signers([ctx.authorityKeypair])
          .rpc();
        expect.fail("Committing vote during reveal phase should be rejected");
      } catch (error) {
        console.log(" -> Received expected error:", error.message);
        expect(error.error.errorCode.code).to.equal("CommitPhaseEnded");
        expect(error.error.errorMessage).to.include("Commit phase has ended");
      }

      // Set back to commit phase for subsequent tests
      await setupVotingPhase(
        ctx,
        "commit",
        ctx.validationSubmissionTopicLinkPda,
        ctx.validationSubmissionPda,
      );
    });

    it("Prevents revealing votes during commit phase", async () => {
      await setupVotingPhase(
        ctx,
        "commit",
        ctx.validationSubmissionTopicLinkPda,
        ctx.validationSubmissionPda,
      );

      // Commit a valid vote first
      const nonce = ctx.VOTE_NONCE_VALIDATION + "reveal-in-commit";
      ctx.VOTE_NONCE_VALIDATION = nonce; // Store the actual nonce used
      ctx.validationVoteHash = createVoteHash(
        ctx.validatorKeypair,
        ctx.validationSubmissionTopicLinkPda,
        0,
        nonce,
      );
      [ctx.validationVoteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          ctx.validationSubmissionTopicLinkPda.toBuffer(),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      // Ensure PDA is clean before init
      try {
        await ctx.program.account.voteCommit.fetch(ctx.validationVoteCommitPda);
        console.warn(
          "Validation Vote Commit PDA already exists. Test might fail.",
        );
      } catch (e) {
        /* Expected */
      }

      console.log("Committing a valid vote first...");
      await ctx.program.methods
        .commitVote(ctx.validationVoteHash, new BN(5), false)
        .accounts({
          topic: ctx.topic1Pda,
          submission: ctx.validationSubmissionPda,
          validatorRepAta: ctx.validatorRepAta,
          validator: ctx.validatorKeypair.publicKey,
          payer: ctx.authorityKeypair.publicKey,
        })
        .signers([ctx.authorityKeypair])
        .rpc();
      console.log(" -> Committed vote successfully.");

      // Attempt reveal while still in commit phase
      console.log(
        "Attempting to reveal vote during commit phase (should fail)...",
      );
      try {
        await ctx.program.methods
          .revealVote(ctx.VOTE_CHOICE_YES, nonce)
          .accounts({
            state: ctx.statePda,
            submissionTopicLink: ctx.validationSubmissionTopicLinkPda,
            topic: ctx.topic1Pda,
            submission: ctx.validationSubmissionPda,
            voteCommit: ctx.validationVoteCommitPda,
            userProfile: ctx.validatorProfilePda,
            validator: ctx.validatorKeypair.publicKey,
            systemProgram: web3.SystemProgram.programId,
          })
          .signers([ctx.validatorKeypair])
          .rpc();
        expect.fail("Revealing vote during commit phase should be rejected");
      } catch (error) {
        console.log(" -> Received expected error:", error.message);
        expect(error.error.errorCode.code).to.equal("RevealPhaseNotStarted");
        expect(error.error.errorMessage).to.include(
          "Reveal phase has not started",
        );
      }
    });

    it("Prevents revealing with incorrect nonce", async () => {
      await setupVotingPhase(
        ctx,
        "reveal",
        ctx.validationSubmissionTopicLinkPda,
        ctx.validationSubmissionPda,
      ); // Move to reveal phase

      console.log(
        "Attempting to reveal vote with incorrect nonce (should fail)...",
      );
      try {
        await ctx.program.methods
          .revealVote(ctx.VOTE_CHOICE_YES, "incorrect-nonce") // Wrong nonce
          .accounts({
            state: ctx.statePda,
            submissionTopicLink: ctx.validationSubmissionTopicLinkPda,
            topic: ctx.topic1Pda,
            submission: ctx.validationSubmissionPda,
            voteCommit: ctx.validationVoteCommitPda, // The vote committed earlier
            userProfile: ctx.validatorProfilePda,
            validator: ctx.validatorKeypair.publicKey,
            systemProgram: web3.SystemProgram.programId,
          })
          .signers([ctx.validatorKeypair])
          .rpc();
        expect.fail("Revealing with incorrect nonce should be rejected");
      } catch (error) {
        console.log(" -> Received expected error:", error.message);
        expect(error.error.errorCode.code).to.equal("InvalidVoteHash");
        expect(error.error.errorMessage).to.include("Invalid vote hash");
      }

      // Reveal correctly to allow subsequent finalization tests
      console.log("Revealing vote with correct nonce for cleanup...");
      await ctx.program.methods
        .revealVote(ctx.VOTE_CHOICE_YES, ctx.VOTE_NONCE_VALIDATION) // Use correct nonce stored earlier
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: ctx.validationSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: ctx.validationSubmissionPda,
          voteCommit: ctx.validationVoteCommitPda,
          userProfile: ctx.validatorProfilePda,
          validator: ctx.validatorKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.validatorKeypair])
        .rpc();
      console.log(" -> Revealed vote successfully.");
      const voteCommit = await ctx.program.account.voteCommit.fetch(
        ctx.validationVoteCommitPda,
      );
      expect(voteCommit.revealed).to.be.true;
    });

    it("Prevents revealing a vote twice", async () => {
      await setupVotingPhase(
        ctx,
        "reveal",
        ctx.validationSubmissionTopicLinkPda,
        ctx.validationSubmissionPda,
      );
      const voteCommit = await ctx.program.account.voteCommit.fetch(
        ctx.validationVoteCommitPda,
      );
      expect(voteCommit.revealed).to.be.true; // Precondition

      console.log(
        "Attempting to reveal an already revealed vote (should fail)...",
      );
      try {
        await ctx.program.methods
          .revealVote(ctx.VOTE_CHOICE_YES, ctx.VOTE_NONCE_VALIDATION) // Correct nonce/choice
          .accounts({
            state: ctx.statePda,
            submissionTopicLink: ctx.validationSubmissionTopicLinkPda,
            topic: ctx.topic1Pda,
            submission: ctx.validationSubmissionPda,
            voteCommit: ctx.validationVoteCommitPda, // The already revealed vote
            userProfile: ctx.validatorProfilePda,
            validator: ctx.validatorKeypair.publicKey,
            systemProgram: web3.SystemProgram.programId,
          })
          .signers([ctx.validatorKeypair])
          .rpc();
        expect.fail("Revealing an already revealed vote should be rejected");
      } catch (error) {
        console.log(" -> Received expected error:", error.message);
        // RevealVote context has `constraint = vote_commit.revealed == false`
        expect(error.error.errorCode.code).to.equal("ConstraintRaw");
        expect(error.error.errorMessage).to.include(
          "A raw constraint was violated",
        );
      }
    });

    it("Prevents revealing votes after reveal phase ends", async () => {
      // === Setup for this specific test ===
      console.log("--- Setting up for 'reveal too late' test ---");
      // Use User3 to ensure a distinct vote commit PDA from the validator's previous vote
      const voter = ctx.user3Keypair;
      const voterProfilePda = ctx.user3ProfilePda;
      const voterTopicBalancePda = ctx.user3Topic1BalancePda;
      const voterRepAta = ctx.user3RepAta; // User3's Rep ATA

      // 1. Ensure the validation submission link is in the commit phase
      await setupVotingPhase(
        ctx,
        "commit",
        ctx.validationSubmissionTopicLinkPda,
        ctx.validationSubmissionPda,
      );

      // 2. Commit a vote as User3 on the validation submission
      const nonce = ctx.VOTE_NONCE_VALIDATION + "user3-reveal-late";
      const voteHash = createVoteHash(
        voter,
        ctx.validationSubmissionTopicLinkPda,
        0,
        nonce,
      );
      const voteAmount = new BN(1); // Commit a small amount

      // Derive PDA for User3's vote using the STANDARD seeds expected by the program context
      const [user3VoteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          ctx.validationSubmissionTopicLinkPda.toBuffer(),
          voter.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );
      // Ensure PDA is clean before init
      try {
        await ctx.program.account.voteCommit.fetch(user3VoteCommitPda);
        console.warn(
          "User3 Vote Commit PDA already exists for validation link. Test might fail.",
        );
      } catch (e) {
        /* Expected */
      }

      console.log(
        `Committing vote as User3 (${voter.publicKey.toBase58()}) for 'reveal too late' test...`,
      );
      await ctx.program.methods
        .commitVote(voteHash, voteAmount, false)
        .accounts({
          topic: ctx.topic1Pda,
          submission: ctx.validationSubmissionPda,
          validatorRepAta: voterRepAta, // Pass User3's Rep ATA
          validator: voter.publicKey, // User3 is the signer/validator here
          payer: ctx.authorityKeypair.publicKey,
        })
        .signers([ctx.authorityKeypair])
        .rpc();
      console.log(
        " -> Committed User3 vote successfully:",
        user3VoteCommitPda.toBase58(),
      );
      const committedVote =
        await ctx.program.account.voteCommit.fetch(user3VoteCommitPda);
      expect(committedVote.revealed).to.be.false; // Ensure it's not revealed yet

      // === Actual Test Logic ===
      // 3. Set phase to *after* reveal phase ended ("finalized")
      await setupVotingPhase(
        ctx,
        "finalized",
        ctx.validationSubmissionTopicLinkPda,
        ctx.validationSubmissionPda,
      );

      // 4. Attempt to reveal User3's vote (which is unrevealed) after the phase ends
      console.log(
        "Attempting to reveal User3's vote after reveal phase ended (should fail)...",
      );
      try {
        await ctx.program.methods
          .revealVote(ctx.VOTE_CHOICE_YES, nonce) // Use the correct nonce
          .accounts({
            state: ctx.statePda,
            submissionTopicLink: ctx.validationSubmissionTopicLinkPda,
            topic: ctx.topic1Pda,
            submission: ctx.validationSubmissionPda,
            voteCommit: user3VoteCommitPda, // Target User3's vote commit
            userProfile: voterProfilePda, // User3's profile
            validator: voter.publicKey, // User3 is signer
            systemProgram: web3.SystemProgram.programId,
          })
          .signers([voter])
          .rpc();
        expect.fail(
          "Revealing vote after reveal phase ended should be rejected",
        );
      } catch (error) {
        console.log(" -> Received expected error:", error.message);
        // The !revealed constraint passes, but the time check inside reveal_vote fails
        expect(error.error.errorCode.code).to.equal("RevealPhaseEnded");
        expect(error.error.errorMessage).to.include("Reveal phase has ended");
      }
    });

    it("Prevents finalizing votes before submission is finalized", async () => {
      await setupVotingPhase(
        ctx,
        "finalized",
        ctx.validationSubmissionTopicLinkPda,
        ctx.validationSubmissionPda,
      );
      const link = await ctx.program.account.submissionTopicLink.fetch(
        ctx.validationSubmissionTopicLinkPda,
      );
      expect(link.status.pending).to.not.be.undefined; // Precondition

      console.log(
        "Attempting to finalize vote before submission is finalized (should fail)...",
      );
      try {
        await ctx.program.methods
          .finalizeVote()
          .accounts({
            state: ctx.statePda,
            submissionTopicLink: ctx.validationSubmissionTopicLinkPda, // The pending link
            topic: ctx.topic1Pda,
            submission: ctx.validationSubmissionPda,
            voteCommit: ctx.validationVoteCommitPda, // The revealed vote
            validatorProfile: ctx.validatorProfilePda,
            userTopicBalance: ctx.validatorTopic1BalancePda,
            validatorTempRepAccount: ctx.validatorTempRepAccount,
            validatorRepAta: ctx.validatorRepAta,
            tempRepMint: ctx.tempRepMintPda,
            repMint: ctx.repMintPda,
            authority: ctx.authorityKeypair.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: web3.SystemProgram.programId,
          })
          .signers([ctx.authorityKeypair])
          .rpc();
        expect.fail(
          "Finalizing vote before submission is finalized should be rejected",
        );
      } catch (error) {
        console.log(" -> Received expected error:", error.message);
        // FinalizeVote context has `constraint = submission_topic_link.status != SubmissionStatus::Pending`
        expect(error.error.errorCode.code).to.equal("ConstraintRaw");
        expect(error.error.errorMessage).to.include(
          "A raw constraint was violated",
        );
      }
    });

    it("Prevents finalizing submission before reveal phase ends", async () => {
      await setupVotingPhase(
        ctx,
        "premature",
        ctx.validationSubmissionTopicLinkPda,
        ctx.validationSubmissionPda,
      );

      console.log(
        "Attempting to finalize submission before reveal phase ends (should fail)...",
      );
      try {
        await ctx.program.methods
          .finalizeSubmission()
          .accounts({
            state: ctx.statePda,
            submissionTopicLink: ctx.validationSubmissionTopicLinkPda,
            topic: ctx.topic1Pda,
            submission: ctx.validationSubmissionPda,
            contributorProfile: ctx.contributorProfilePda,
            userTopicBalance: ctx.contributorTopic1BalancePda,
            contributorTempAlignAccount: ctx.contributorTempAlignAccount,
            contributorAlignAta: ctx.contributorAlignAta,
            tempAlignMint: ctx.tempAlignMintPda,
            alignMint: ctx.alignMintPda,
            authority: ctx.authorityKeypair.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: web3.SystemProgram.programId,
          })
          .signers([ctx.authorityKeypair])
          .rpc();
        expect.fail(
          "Finalizing submission before reveal phase ends should be rejected",
        );
      } catch (error) {
        console.log(" -> Received expected error:", error.message);
        expect(error.error.errorCode.code).to.equal("RevealPhaseNotEnded");
        expect(error.error.errorMessage).to.include(
          "Reveal phase has not ended yet",
        );
      }
    });

    it("Prevents finalizing submission twice", async () => {
      await setupVotingPhase(
        ctx,
        "finalized",
        ctx.validationSubmissionTopicLinkPda,
        ctx.validationSubmissionPda,
      );
      console.log("Finalizing submission first time...");
      await ctx.program.methods
        .finalizeSubmission()
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: ctx.validationSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: ctx.validationSubmissionPda,
          contributorProfile: ctx.contributorProfilePda,
          userTopicBalance: ctx.contributorTopic1BalancePda,
          contributorTempAlignAccount: ctx.contributorTempAlignAccount,
          contributorAlignAta: ctx.contributorAlignAta,
          tempAlignMint: ctx.tempAlignMintPda,
          alignMint: ctx.alignMintPda,
          authority: ctx.authorityKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.authorityKeypair])
        .rpc();
      console.log(" -> Finalized submission successfully.");
      const link = await ctx.program.account.submissionTopicLink.fetch(
        ctx.validationSubmissionTopicLinkPda,
      );
      expect(link.status.pending).to.be.undefined;

      console.log("Attempting to finalize submission again (should fail)...");
      try {
        await ctx.program.methods
          .finalizeSubmission()
          .accounts({
            state: ctx.statePda,
            submissionTopicLink: ctx.validationSubmissionTopicLinkPda,
            topic: ctx.topic1Pda,
            submission: ctx.validationSubmissionPda,
            contributorProfile: ctx.contributorProfilePda,
            userTopicBalance: ctx.contributorTopic1BalancePda,
            contributorTempAlignAccount: ctx.contributorTempAlignAccount,
            contributorAlignAta: ctx.contributorAlignAta,
            tempAlignMint: ctx.tempAlignMintPda,
            alignMint: ctx.alignMintPda,
            authority: ctx.authorityKeypair.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: web3.SystemProgram.programId,
          })
          .signers([ctx.authorityKeypair])
          .rpc();
        expect.fail("Finalizing submission twice should be rejected");
      } catch (error) {
        console.log(" -> Received expected error:", error.message);
        expect(error.error.errorCode.code).to.equal("SubmissionNotPending");
        expect(error.error.errorMessage).to.include(
          "Submission is not in the pending state",
        );
      }
    });

    it("Prevents finalizing vote twice", async () => {
      console.log("Finalizing vote first time...");
      await ctx.program.methods
        .finalizeVote()
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: ctx.validationSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: ctx.validationSubmissionPda,
          voteCommit: ctx.validationVoteCommitPda,
          validatorProfile: ctx.validatorProfilePda,
          userTopicBalance: ctx.validatorTopic1BalancePda,
          validatorTempRepAccount: ctx.validatorTempRepAccount,
          validatorRepAta: ctx.validatorRepAta,
          tempRepMint: ctx.tempRepMintPda,
          repMint: ctx.repMintPda,
          authority: ctx.authorityKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.authorityKeypair])
        .rpc();
      console.log(" -> Finalized vote successfully.");
      const voteCommit = await ctx.program.account.voteCommit.fetch(
        ctx.validationVoteCommitPda,
      );
      expect(voteCommit.finalized).to.be.true;

      console.log("Attempting to finalize vote again (should fail)...");
      try {
        await ctx.program.methods
          .finalizeVote()
          .accounts({
            state: ctx.statePda,
            submissionTopicLink: ctx.validationSubmissionTopicLinkPda,
            topic: ctx.topic1Pda,
            submission: ctx.validationSubmissionPda,
            voteCommit: ctx.validationVoteCommitPda,
            validatorProfile: ctx.validatorProfilePda,
            userTopicBalance: ctx.validatorTopic1BalancePda,
            validatorTempRepAccount: ctx.validatorTempRepAccount,
            validatorRepAta: ctx.validatorRepAta,
            tempRepMint: ctx.tempRepMintPda,
            repMint: ctx.repMintPda,
            authority: ctx.authorityKeypair.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: web3.SystemProgram.programId,
          })
          .signers([ctx.authorityKeypair])
          .rpc();
        expect.fail("Finalizing vote twice should be rejected");
      } catch (error) {
        console.log(" -> Received expected error:", error.message);
        expect(error.error.errorCode.code).to.equal("VoteAlreadyFinalized");
        expect(error.error.errorMessage).to.include(
          "Vote has already been finalized",
        );
      }
    });
  }); // End describe block
} // End runValidationTests function
