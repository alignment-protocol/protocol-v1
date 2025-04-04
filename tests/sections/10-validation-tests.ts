import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { web3 } from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { TestContext } from "../utils/test-setup";
import * as crypto from "crypto";

export function runValidationTests(ctx: TestContext): void {
  describe("Vote Validation and Error Handling", () => {
    let testSubmissionPda: web3.PublicKey;
    let testSubmissionTopicLinkPda: web3.PublicKey;
    let userWithNoTokensPda: web3.PublicKey;

    // Helper to create a vote hash
    function createVoteHash(
      voter: web3.Keypair,
      submissionTopicLink: web3.PublicKey,
      choice: number,
      nonce: string,
    ) {
      const message = Buffer.concat([
        voter.publicKey.toBuffer(),
        submissionTopicLink.toBuffer(),
        Buffer.from([choice]), // 0 for Yes, 1 for No
        Buffer.from(nonce),
      ]);

      return Array.from(crypto.createHash("sha256").update(message).digest());
    }

    // Setup helper function to create voting phases for testing
    async function setupVotingPhase(
      phase: "commit" | "reveal" | "finalized",
      submissionTopicLinkPda: web3.PublicKey,
    ) {
      const now = Math.floor(Date.now() / 1000);
      let commitPhaseStart, commitPhaseEnd, revealPhaseStart, revealPhaseEnd;

      if (phase === "commit") {
        // Set up for commit phase (active now)
        commitPhaseStart = now - 60; // 1 minute ago
        commitPhaseEnd = now + 600; // 10 minutes from now
        revealPhaseStart = commitPhaseEnd;
        revealPhaseEnd = commitPhaseEnd + 600; // 10 minutes after commit phase
      } else if (phase === "reveal") {
        // Set up for reveal phase (active now)
        commitPhaseStart = now - 1200; // 20 minutes ago
        commitPhaseEnd = now - 60; // 1 minute ago
        revealPhaseStart = commitPhaseEnd;
        revealPhaseEnd = now + 600; // 10 minutes from now
      } else {
        // Set up for finalized (past reveal phase)
        commitPhaseStart = now - 2400; // 40 minutes ago
        commitPhaseEnd = now - 1800; // 30 minutes ago
        revealPhaseStart = commitPhaseEnd;
        revealPhaseEnd = now - 60; // 1 minute ago
      }

      // Set the voting phases
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
          submission: testSubmissionPda,
          authority: ctx.authorityKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.authorityKeypair])
        .rpc();

      console.log(`Set voting phases for ${phase} phase, tx: ${setPhasesTx}`);
      return {
        commitPhaseStart,
        commitPhaseEnd,
        revealPhaseStart,
        revealPhaseEnd,
      };
    }

    before(async () => {
      // Setup test data - create a submission to use for validation tests
      const submissionData = "Test submission for validation tests";

      // Get the current submission count
      const stateAcc = await ctx.program.account.state.fetch(ctx.statePda);
      const currentSubmissionCount = stateAcc.submissionCount.toNumber();
      console.log(
        "Current submission count for validation tests:",
        currentSubmissionCount,
      );

      // Derive the PDAs for the new submission
      [testSubmissionPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("submission"),
          new anchor.BN(currentSubmissionCount).toBuffer("le", 8),
        ],
        ctx.program.programId,
      );

      [testSubmissionTopicLinkPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("submission_topic_link"),
          testSubmissionPda.toBuffer(),
          ctx.topic1Pda.toBuffer(),
        ],
        ctx.program.programId,
      );

      // Create a test submission from the contributor
      const createSubmissionTx = await ctx.program.methods
        .submitDataToTopic(submissionData)
        .accounts({
          state: ctx.statePda,
          topic: ctx.topic1Pda,
          tempAlignMint: ctx.tempAlignMintPda,
          contributorTempAlignAccount: ctx.contributorTempAlignAccount,
          submission: testSubmissionPda,
          submissionTopicLink: testSubmissionTopicLinkPda,
          contributorProfile: ctx.contributorProfilePda,
          contributor: ctx.contributorKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.contributorKeypair])
        .rpc();

      console.log(
        "Created submission for validation tests:",
        createSubmissionTx,
      );

      // Set up for commit phase
      await setupVotingPhase("commit", testSubmissionTopicLinkPda);

      // Create a user with no tokens for testing insufficient tokens case
      const noTokensKeypair = web3.Keypair.generate();

      // Fund this account with SOL
      const fundTx = new web3.Transaction().add(
        web3.SystemProgram.transfer({
          fromPubkey: ctx.authorityKeypair.publicKey,
          toPubkey: noTokensKeypair.publicKey,
          lamports: 0.1 * web3.LAMPORTS_PER_SOL,
        }),
      );
      await ctx.provider.sendAndConfirm(fundTx, [ctx.authorityKeypair]);

      // Create a user profile for this account
      [userWithNoTokensPda] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("user_profile"), noTokensKeypair.publicKey.toBuffer()],
        ctx.program.programId,
      );

      try {
        const createProfileTx = await ctx.program.methods
          .createUserProfile()
          .accounts({
            state: ctx.statePda,
            userProfile: userWithNoTokensPda,
            user: noTokensKeypair.publicKey,
            systemProgram: web3.SystemProgram.programId,
            rent: web3.SYSVAR_RENT_PUBKEY,
          })
          .signers([noTokensKeypair])
          .rpc();

        console.log(
          "Created profile for user with no tokens:",
          createProfileTx,
        );
      } catch (error) {
        console.log("Error creating profile for no-tokens user:", error);
      }
    });

    it("Prevents self-voting on own submissions", async () => {
      // Try to self-vote (contributor voting on own submission)
      const selfVoteHash = createVoteHash(
        ctx.contributorKeypair,
        testSubmissionTopicLinkPda,
        0, // Yes vote
        "self-vote-nonce",
      );

      // Derive the vote commit PDA for self-vote
      const [selfVoteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          testSubmissionTopicLinkPda.toBuffer(),
          ctx.contributorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      // Try to commit the self-vote
      try {
        const tx = await ctx.program.methods
          .commitVote(selfVoteHash, new anchor.BN(5), false)
          .accounts({
            state: ctx.statePda,
            submissionTopicLink: testSubmissionTopicLinkPda,
            topic: ctx.topic1Pda,
            submission: testSubmissionPda,
            voteCommit: selfVoteCommitPda,
            userProfile: ctx.contributorProfilePda,
            validator: ctx.contributorKeypair.publicKey,
            systemProgram: web3.SystemProgram.programId,
            rent: web3.SYSVAR_RENT_PUBKEY,
          })
          .signers([ctx.contributorKeypair])
          .rpc();

        // If we reach here, the test failed because self-voting should be rejected
        expect.fail("Self-voting should have been rejected");
      } catch (error) {
        // Expect an error about self-voting not allowed
        expect(error.toString()).to.include("SelfVotingNotAllowed");
      }
    });

    it("Prevents voting with insufficient tokens", async () => {
      // Try to vote with more tokens than available
      const voteHash = createVoteHash(
        ctx.validatorKeypair,
        testSubmissionTopicLinkPda,
        0, // Yes vote
        "insufficient-tokens-nonce",
      );

      // Derive the vote commit PDA
      const [voteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          testSubmissionTopicLinkPda.toBuffer(),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      // Find out how many tokens the validator has
      const validatorProfile = await ctx.program.account.userProfile.fetch(
        ctx.validatorProfilePda,
      );

      const topicTokenEntry = validatorProfile.topicTokens.find(
        (pair) => pair.topicId.toNumber() === 0, // Topic ID 0
      );

      const availableTokens = topicTokenEntry.token.tempRepAmount.toNumber();
      console.log(`Validator has ${availableTokens} available tempRep tokens`);

      // Try to commit a vote with more tokens than available
      const excessAmount = availableTokens + 1;

      try {
        const tx = await ctx.program.methods
          .commitVote(voteHash, new anchor.BN(excessAmount), false)
          .accounts({
            state: ctx.statePda,
            submissionTopicLink: testSubmissionTopicLinkPda,
            topic: ctx.topic1Pda,
            submission: testSubmissionPda,
            voteCommit: voteCommitPda,
            userProfile: ctx.validatorProfilePda,
            validator: ctx.validatorKeypair.publicKey,
            systemProgram: web3.SystemProgram.programId,
            rent: web3.SYSVAR_RENT_PUBKEY,
          })
          .signers([ctx.validatorKeypair])
          .rpc();

        // If we reach here, the test failed
        expect.fail("Voting with insufficient tokens should be rejected");
      } catch (error) {
        // Expect an error about insufficient tokens
        expect(error.toString()).to.include("InsufficientTokenBalance");
      }
    });

    it("Prevents committing votes during reveal phase", async () => {
      // Set up for reveal phase
      await setupVotingPhase("reveal", testSubmissionTopicLinkPda);

      // Try to commit a vote during reveal phase
      const voteHash = createVoteHash(
        ctx.validatorKeypair,
        testSubmissionTopicLinkPda,
        0, // Yes vote
        "wrong-phase-nonce",
      );

      // Derive the vote commit PDA
      const [voteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          testSubmissionTopicLinkPda.toBuffer(),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      try {
        const tx = await ctx.program.methods
          .commitVote(voteHash, new anchor.BN(5), false)
          .accounts({
            state: ctx.statePda,
            submissionTopicLink: testSubmissionTopicLinkPda,
            topic: ctx.topic1Pda,
            submission: testSubmissionPda,
            voteCommit: voteCommitPda,
            userProfile: ctx.validatorProfilePda,
            validator: ctx.validatorKeypair.publicKey,
            systemProgram: web3.SystemProgram.programId,
            rent: web3.SYSVAR_RENT_PUBKEY,
          })
          .signers([ctx.validatorKeypair])
          .rpc();

        // If we reach here, the test failed
        expect.fail("Committing vote during reveal phase should be rejected");
      } catch (error) {
        // Expect an error about wrong voting phase
        expect(error.toString()).to.include("VotingPhaseError");
      }

      // Set back to commit phase for other tests
      await setupVotingPhase("commit", testSubmissionTopicLinkPda);
    });

    it("Prevents revealing votes during commit phase", async () => {
      // First commit a vote
      const voteHash = createVoteHash(
        ctx.validatorKeypair,
        testSubmissionTopicLinkPda,
        0, // Yes vote
        "reveal-test-nonce",
      );

      // Derive the vote commit PDA
      const [voteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          testSubmissionTopicLinkPda.toBuffer(),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      // Commit the vote
      const commitTx = await ctx.program.methods
        .commitVote(voteHash, new anchor.BN(5), false)
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: testSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: testSubmissionPda,
          voteCommit: voteCommitPda,
          userProfile: ctx.validatorProfilePda,
          validator: ctx.validatorKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.validatorKeypair])
        .rpc();

      console.log("Committed vote for reveal test:", commitTx);

      // Now try to reveal it during commit phase
      try {
        const revealTx = await ctx.program.methods
          .revealVote(
            ctx.VOTE_CHOICE_YES, // Yes vote
            "reveal-test-nonce", // Nonce used in commit
          )
          .accounts({
            state: ctx.statePda,
            submissionTopicLink: testSubmissionTopicLinkPda,
            topic: ctx.topic1Pda,
            submission: testSubmissionPda,
            voteCommit: voteCommitPda,
            userProfile: ctx.validatorProfilePda,
            validator: ctx.validatorKeypair.publicKey,
            systemProgram: web3.SystemProgram.programId,
          })
          .signers([ctx.validatorKeypair])
          .rpc();

        // If we reach here, the test failed
        expect.fail("Revealing vote during commit phase should be rejected");
      } catch (error) {
        // Expect an error about wrong voting phase
        expect(error.toString()).to.include("VotingPhaseError");
      }
    });

    it("Prevents duplicate vote commitment", async () => {
      // Since we already committed a vote in previous test, try to commit again
      const duplicateVoteHash = createVoteHash(
        ctx.validatorKeypair,
        testSubmissionTopicLinkPda,
        1, // No vote this time
        "different-nonce",
      );

      // Use the same vote commit PDA
      const [voteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          testSubmissionTopicLinkPda.toBuffer(),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      // Try to commit the duplicate vote
      try {
        const tx = await ctx.program.methods
          .commitVote(duplicateVoteHash, new anchor.BN(5), false)
          .accounts({
            state: ctx.statePda,
            submissionTopicLink: testSubmissionTopicLinkPda,
            topic: ctx.topic1Pda,
            submission: testSubmissionPda,
            voteCommit: voteCommitPda,
            userProfile: ctx.validatorProfilePda,
            validator: ctx.validatorKeypair.publicKey,
            systemProgram: web3.SystemProgram.programId,
            rent: web3.SYSVAR_RENT_PUBKEY,
          })
          .signers([ctx.validatorKeypair])
          .rpc();

        // If we reach here, the test failed because the duplicate vote should be rejected
        expect.fail("Duplicate vote commitment should have been rejected");
      } catch (error) {
        // Expect an error about duplicate vote commitment
        expect(error.toString()).to.include("DuplicateVoteCommitment");
      }
    });

    it("Prevents revealing with incorrect nonce", async () => {
      // Move to reveal phase
      await setupVotingPhase("reveal", testSubmissionTopicLinkPda);

      // Get the vote commit PDA from previous test
      const [voteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          testSubmissionTopicLinkPda.toBuffer(),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      // Try to reveal with incorrect nonce
      try {
        const revealTx = await ctx.program.methods
          .revealVote(
            ctx.VOTE_CHOICE_YES, // Yes vote
            "incorrect-nonce", // Wrong nonce!
          )
          .accounts({
            state: ctx.statePda,
            submissionTopicLink: testSubmissionTopicLinkPda,
            topic: ctx.topic1Pda,
            submission: testSubmissionPda,
            voteCommit: voteCommitPda,
            userProfile: ctx.validatorProfilePda,
            validator: ctx.validatorKeypair.publicKey,
            systemProgram: web3.SystemProgram.programId,
          })
          .signers([ctx.validatorKeypair])
          .rpc();

        // If we reach here, the test failed
        expect.fail("Revealing with incorrect nonce should be rejected");
      } catch (error) {
        // Expect an error about hash mismatch
        expect(error.toString()).to.include("VoteHashMismatch");
      }

      // Now reveal with correct nonce to clean up
      const correctRevealTx = await ctx.program.methods
        .revealVote(
          ctx.VOTE_CHOICE_YES, // Yes vote
          "reveal-test-nonce", // Correct nonce
        )
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: testSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: testSubmissionPda,
          voteCommit: voteCommitPda,
          userProfile: ctx.validatorProfilePda,
          validator: ctx.validatorKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.validatorKeypair])
        .rpc();

      console.log(
        "Successfully revealed vote with correct nonce:",
        correctRevealTx,
      );
    });

    it("Prevents finalizing votes before submission is finalized", async () => {
      // Move to finalized phase
      await setupVotingPhase("finalized", testSubmissionTopicLinkPda);

      // Get the vote commit PDA
      const [voteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          testSubmissionTopicLinkPda.toBuffer(),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      // Try to finalize vote before finalizing submission
      try {
        const finalizeVoteTx = await ctx.program.methods
          .finalizeVote()
          .accounts({
            state: ctx.statePda,
            submissionTopicLink: testSubmissionTopicLinkPda,
            topic: ctx.topic1Pda,
            submission: testSubmissionPda,
            voteCommit: voteCommitPda,
            validatorProfile: ctx.validatorProfilePda,
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

        // If we reach here, the test failed
        expect.fail(
          "Finalizing vote before submission is finalized should be rejected",
        );
      } catch (error) {
        // Expect an error about submission not being finalized
        expect(error.toString()).to.include("PendingSubmission");
      }

      // Now finalize the submission
      const finalizeTx = await ctx.program.methods
        .finalizeSubmission()
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: testSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: testSubmissionPda,
          contributorProfile: ctx.contributorProfilePda,
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

      console.log("Finalized submission:", finalizeTx);

      // Now finalize the vote should work
      const finalizeVoteTx = await ctx.program.methods
        .finalizeVote()
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: testSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: testSubmissionPda,
          voteCommit: voteCommitPda,
          validatorProfile: ctx.validatorProfilePda,
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

      console.log("Successfully finalized vote:", finalizeVoteTx);
    });
  });
}
