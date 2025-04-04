import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { web3 } from "@coral-xyz/anchor";
import {
  TOKEN_PROGRAM_ID,
  getAccount,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import { TestContext } from "../utils/test-setup";
import * as crypto from "crypto";

export function runTokenLockingTests(ctx: TestContext): void {
  describe("Token Locking and Unlocking Flow", () => {
    let secondVoteCommitPda: web3.PublicKey;
    let secondVoteHash: number[];
    let testSubmissionPda: web3.PublicKey;
    let testSubmissionTopicLinkPda: web3.PublicKey;

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

    // Helper to check locked and available tokens
    async function checkUserTokenBalances(
      userProfilePda: web3.PublicKey,
      topicId: number,
    ) {
      const userProfile =
        await ctx.program.account.userProfile.fetch(userProfilePda);
      const [userTempAlignPda] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("user_temp_align"), userProfile.user.toBuffer()],
        ctx.program.programId,
      );
      const [userTempRepPda] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("user_temp_rep"), userProfile.user.toBuffer()],
        ctx.program.programId,
      );
      const userAlignAta = await getAssociatedTokenAddress(
        ctx.alignMintPda,
        userProfile.user,
      );
      const userRepAta = await getAssociatedTokenAddress(
        ctx.repMintPda,
        userProfile.user,
      );
      const userTempAlignAta = await getAccount(
        ctx.provider.connection,
        userTempAlignPda,
      );
      const userTempRepAta = await getAccount(
        ctx.provider.connection,
        userTempRepPda,
      );
      const userAlignData = await getAccount(
        ctx.provider.connection,
        userAlignAta,
      );
      const userRepData = await getAccount(ctx.provider.connection, userRepAta);
      const topicTokenEntry = userProfile.topicTokens.find(
        (pair) => pair.topicId.toNumber() === topicId,
      );

      if (!topicTokenEntry) {
        return {
          permanentAlign: Number(userAlignData.amount),
          permanentRep: Number(userRepData.amount),
          tempAlign: Number(userTempAlignAta.amount),
          tempRep: Number(userTempRepAta.amount),
        };
      } else {
        return {
          tempAlign: Number(userTempAlignAta.amount),
          tempRep: Number(userTempRepAta.amount),
          permanentAlign: Number(userAlignData.amount),
          permanentRep: Number(userRepData.amount),
          topicSpecificTokens: {
            tempAlign: topicTokenEntry.token.tempAlignAmount.toNumber(),
            tempRep: topicTokenEntry.token.tempRepAmount.toNumber(),
            lockedTempRep: topicTokenEntry.token.lockedTempRepAmount.toNumber(),
          },
        };
      }
    }

    before(async () => {
      // Check starting balances
      console.log("=== Rolling Over Balances from Section 08 ===");
      // Contributor: 50 align, 50 tempRep, 0 tempAlign
      // 100 tempAlign tokens minted, 50 staked for tempRep, 50 tempAlign tokens converted to align
      const contributorCurrentBalances = await checkUserTokenBalances(
        ctx.contributorProfilePda,
        0,
      );
      console.log("Contributor current balances:", contributorCurrentBalances);
      expect(contributorCurrentBalances.tempAlign).to.equal(0);
      expect(contributorCurrentBalances.tempRep).to.equal(50);
      expect(contributorCurrentBalances.permanentAlign).to.equal(50);
      expect(contributorCurrentBalances.permanentRep).to.equal(0);
      expect(contributorCurrentBalances.topicSpecificTokens.tempAlign).to.equal(
        0,
      );
      expect(contributorCurrentBalances.topicSpecificTokens.tempRep).to.equal(
        50,
      );
      expect(
        contributorCurrentBalances.topicSpecificTokens.lockedTempRep,
      ).to.equal(0);

      // Validator: 50 tempAlign, 25 tempRep, 25 permanent Rep
      // 100 tempAlign tokens minted, 50 staked for tempRep,
      // 25 tempRep tokens used for voting then converted to permanent Rep
      const validatorCurrentBalances = await checkUserTokenBalances(
        ctx.validatorProfilePda,
        0,
      );
      console.log("Validator current balances:", validatorCurrentBalances);
      expect(validatorCurrentBalances.tempAlign).to.equal(50);
      expect(validatorCurrentBalances.tempRep).to.equal(25);
      expect(validatorCurrentBalances.permanentAlign).to.equal(0);
      expect(validatorCurrentBalances.permanentRep).to.equal(25);
      expect(validatorCurrentBalances.topicSpecificTokens.tempAlign).to.equal(
        50,
      );
      expect(validatorCurrentBalances.topicSpecificTokens.tempRep).to.equal(25);
      expect(
        validatorCurrentBalances.topicSpecificTokens.lockedTempRep,
      ).to.equal(0);

      // User3 should have all 0 balances
      const user3CurrentBalances = await checkUserTokenBalances(
        ctx.user3ProfilePda,
        0,
      );
      console.log("User3 current balances:", user3CurrentBalances);
      expect(user3CurrentBalances.tempAlign).to.equal(0);
      expect(user3CurrentBalances.tempRep).to.equal(0);
      expect(user3CurrentBalances.permanentAlign).to.equal(0);
      expect(user3CurrentBalances.permanentRep).to.equal(0);

      // Create a new submission specifically for the token locking tests
      const submissionData = "Test submission for token locking tests";
      let currentSubmissionCount = (
        await ctx.program.account.state.fetch(ctx.statePda)
      ).submissionCount.toNumber();

      // Get the current submission count before creating our test submission
      console.log(
        "Current submission count before token locking test submission:",
        currentSubmissionCount,
      );

      // Derive the PDAs for the new submission using the submission count
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

      // Create the test submission using submitDataToTopic (the correct method)
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
        "Created test submission for token locking tests:",
        createSubmissionTx,
      );

      // Check contributor's balance after submission
      const contributorAfterSubmission = await checkUserTokenBalances(
        ctx.contributorProfilePda,
        0,
      );
      console.log("Contributor after submission:", contributorAfterSubmission);
      // Contributor should have 100 newly minted tempAlign tokens
      expect(contributorAfterSubmission.tempAlign).to.equal(100);

      // User 3 also makes a submission to get tempAlign tokens
      currentSubmissionCount = (
        await ctx.program.account.state.fetch(ctx.statePda)
      ).submissionCount.toNumber();

      const [user3SubmissionPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("submission"),
          new anchor.BN(currentSubmissionCount).toBuffer("le", 8),
        ],
        ctx.program.programId,
      );

      const [user3SubmissionTopicLinkPda] =
        web3.PublicKey.findProgramAddressSync(
          [
            Buffer.from("submission_topic_link"),
            user3SubmissionPda.toBuffer(),
            ctx.topic1Pda.toBuffer(),
          ],
          ctx.program.programId,
        );

      // Submit data for user3 to get tempAlign tokens
      const user3SubmitTx = await ctx.program.methods
        .submitDataToTopic("User3 submission to earn tempAlign tokens")
        .accounts({
          state: ctx.statePda,
          topic: ctx.topic1Pda,
          tempAlignMint: ctx.tempAlignMintPda,
          contributorTempAlignAccount: ctx.user3TempAlignAccount,
          submission: user3SubmissionPda,
          submissionTopicLink: user3SubmissionTopicLinkPda,
          contributorProfile: ctx.user3ProfilePda,
          contributor: ctx.user3Keypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.user3Keypair])
        .rpc();

      console.log("User3 submission transaction:", user3SubmitTx);

      // Check user3's balance after submission
      const user3AfterSubmission = await checkUserTokenBalances(
        ctx.user3ProfilePda,
        0,
      );
      console.log("User3 after submission:", user3AfterSubmission);
      expect(user3AfterSubmission.tempAlign).to.equal(100);

      // Now user 3 stakes 50 tempAlign for 50 tempRep tokens
      const user3StakeTx = await ctx.program.methods
        .stakeTopicSpecificTokens(new anchor.BN(50))
        .accounts({
          state: ctx.statePda,
          topic: ctx.topic1Pda,
          userProfile: ctx.user3ProfilePda,
          tempAlignMint: ctx.tempAlignMintPda,
          tempRepMint: ctx.tempRepMintPda,
          userTempAlignAccount: ctx.user3TempAlignAccount,
          userTempRepAccount: ctx.user3TempRepAccount,
          user: ctx.user3Keypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.user3Keypair])
        .rpc();

      console.log("User3 staking transaction:", user3StakeTx);

      // Check user3's balance after staking
      const user3AfterStaking = await checkUserTokenBalances(
        ctx.user3ProfilePda,
        0,
      );
      console.log("User3 balance after staking:", user3AfterStaking);
      expect(user3AfterStaking.tempAlign).to.equal(50); // 100 minted - 50 staked = 50
      expect(user3AfterStaking.tempRep).to.equal(50); // 50 staked = 50 tempRep
      expect(user3AfterStaking.topicSpecificTokens.tempAlign).to.equal(50);
      expect(user3AfterStaking.topicSpecificTokens.tempRep).to.equal(50);
      expect(user3AfterStaking.topicSpecificTokens.lockedTempRep).to.equal(0);

      // Make sure we're in the commit phase before starting tests
      await setupVotingPhase("commit", testSubmissionTopicLinkPda);
    });

    it("Locks tokens when committing a vote", async () => {
      // Check token balances before voting
      const beforeBalances = await checkUserTokenBalances(
        ctx.validatorProfilePda,
        0,
      );
      console.log("Validator before committing vote:", beforeBalances);
      // Validator should have 50 tempAlign and 25 tempRep from section 08
      expect(beforeBalances.tempAlign).to.equal(50);
      expect(beforeBalances.tempRep).to.equal(25);
      expect(beforeBalances.topicSpecificTokens.lockedTempRep).to.equal(0); // No locked tokens at this point

      // Calculate vote hash
      const voteHash = createVoteHash(
        ctx.validatorKeypair,
        testSubmissionTopicLinkPda,
        0, // Yes vote
        "test-nonce-1",
      );

      // Derive the vote commit PDA
      const [testVoteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          testSubmissionTopicLinkPda.toBuffer(),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      // Commit the vote with 10 tokens
      const voteAmount = 10;
      const isPermanentRep = false;

      // Commit the vote
      const tx = await ctx.program.methods
        .commitVote(voteHash, new anchor.BN(voteAmount), isPermanentRep)
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: testSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: testSubmissionPda,
          voteCommit: testVoteCommitPda,
          userProfile: ctx.validatorProfilePda,
          validator: ctx.validatorKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.validatorKeypair])
        .rpc();

      console.log("Token locking vote commit transaction:", tx);

      // Check token balances after voting
      const afterBalances = await checkUserTokenBalances(
        ctx.validatorProfilePda,
        0,
      );
      console.log("Validator after committing vote:", afterBalances);

      // Verify tokens were locked properly - precise numbers
      expect(afterBalances.topicSpecificTokens.lockedTempRep).to.equal(10); // 10 tokens locked for this vote
      expect(afterBalances.topicSpecificTokens.tempRep).to.equal(15); // 25 - 10 = 15 remaining available
      expect(afterBalances.tempAlign).to.equal(50); // tempAlign remains unchanged

      // Store for later tests
      ctx.voteHash = voteHash;
      ctx.voteCommitPda = testVoteCommitPda;
    });

    it("Properly handles multiple votes with different token amounts (quadratic voting calculation)", async () => {
      // Calculate vote hash for user3
      const user3VoteHash = createVoteHash(
        ctx.user3Keypair,
        testSubmissionTopicLinkPda,
        0, // Yes vote
        "user3-nonce",
      );

      // Derive the vote commit PDA for user3
      [secondVoteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          testSubmissionTopicLinkPda.toBuffer(),
          ctx.user3Keypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      // Commit the vote from user3 with a different amount (36 tokens = 6 voting power)
      const voteAmount = 36; // sqrt(36) = 6 voting power

      // Check user3's balance before voting
      const beforeVote = await checkUserTokenBalances(ctx.user3ProfilePda, 0);
      console.log("User3 before committing vote:", beforeVote);
      expect(beforeVote.tempRep).to.equal(50); // 50 from staking
      expect(beforeVote.topicSpecificTokens.lockedTempRep).to.equal(0);

      // Commit user3's vote
      const tx = await ctx.program.methods
        .commitVote(user3VoteHash, new anchor.BN(voteAmount), false)
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: testSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: testSubmissionPda,
          voteCommit: secondVoteCommitPda,
          userProfile: ctx.user3ProfilePda,
          validator: ctx.user3Keypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.user3Keypair])
        .rpc();

      console.log("User3 vote commit transaction:", tx);

      // Save user3's vote hash for later
      secondVoteHash = user3VoteHash;

      // Verify user3's tokens were locked properly
      const user3AfterVote = await checkUserTokenBalances(
        ctx.user3ProfilePda,
        0,
      );
      console.log("User3 after committing vote:", user3AfterVote);
      expect(user3AfterVote.topicSpecificTokens.lockedTempRep).to.equal(
        voteAmount,
      ); // 36 tokens locked
      expect(user3AfterVote.topicSpecificTokens.tempRep).to.equal(14); // 50 - 36 = 14 remaining available

      // Move to reveal phase
      await setupVotingPhase("reveal", testSubmissionTopicLinkPda);

      // Reveal validator's vote first
      const revealTx1 = await ctx.program.methods
        .revealVote(
          ctx.VOTE_CHOICE_YES, // Yes vote
          "test-nonce-1", // Nonce used in commit
        )
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: testSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: testSubmissionPda,
          voteCommit: ctx.voteCommitPda,
          userProfile: ctx.validatorProfilePda,
          validator: ctx.validatorKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.validatorKeypair])
        .rpc();

      console.log("Validator vote reveal transaction:", revealTx1);

      // Reveal user3's vote
      const revealTx2 = await ctx.program.methods
        .revealVote(
          ctx.VOTE_CHOICE_YES, // Yes vote
          "user3-nonce", // Nonce used in commit
        )
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: testSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: testSubmissionPda,
          voteCommit: secondVoteCommitPda,
          userProfile: ctx.user3ProfilePda,
          validator: ctx.user3Keypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.user3Keypair])
        .rpc();

      console.log("User3 vote reveal transaction:", revealTx2);

      // Check that the quadratic voting calculation was done correctly
      const linkAcc = await ctx.program.account.submissionTopicLink.fetch(
        testSubmissionTopicLinkPda,
      );

      // Validator used 10 tokens = sqrt(10) ≈ 3.16, rounded to 3
      // User3 used 36 tokens = sqrt(36) = 6
      // Total yes voting power should be approximately 9
      expect(linkAcc.yesVotingPower.toNumber()).to.equal(9);
      expect(linkAcc.noVotingPower.toNumber()).to.equal(0);

      // Check token balances after reveals - tokens should still be locked
      const validatorAfterReveal = await checkUserTokenBalances(
        ctx.validatorProfilePda,
        0,
      );
      const user3AfterReveal = await checkUserTokenBalances(
        ctx.user3ProfilePda,
        0,
      );

      console.log("Validator tokens after reveals:", validatorAfterReveal);
      console.log("User3 tokens after reveals:", user3AfterReveal);

      // Validator should still have 10 tokens locked after reveal
      expect(validatorAfterReveal.topicSpecificTokens.tempRep).to.equal(15); // 25 - 10 = 15
      expect(validatorAfterReveal.topicSpecificTokens.lockedTempRep).to.equal(
        10,
      );

      // User3 should still have 36 tokens locked after reveal
      expect(user3AfterReveal.topicSpecificTokens.tempRep).to.equal(14); // 50 - 36 = 14
      expect(user3AfterReveal.topicSpecificTokens.lockedTempRep).to.equal(36);
    });

    it("Unlocks tokens after vote finalization", async () => {
      // Move to finalized phase
      await setupVotingPhase("finalized", testSubmissionTopicLinkPda);

      // Check token balances before finalization
      const validatorBeforeFinalization = await checkUserTokenBalances(
        ctx.validatorProfilePda,
        0,
      );
      console.log(
        "Validator before finalization:",
        validatorBeforeFinalization,
      );

      // Validator should still have:
      // - 15 available tempRep (25 initial - 10 locked)
      // - 10 locked tempRep
      expect(validatorBeforeFinalization.topicSpecificTokens.tempRep).to.equal(
        15,
      );
      expect(
        validatorBeforeFinalization.topicSpecificTokens.lockedTempRep,
      ).to.equal(10);

      // Finalize the submission first
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

      console.log("Finalize submission transaction:", finalizeTx);

      // Check contributor balance after submission finalization
      const contributorAfterFinalization = await checkUserTokenBalances(
        ctx.contributorProfilePda,
        0,
      );
      console.log(
        "Contributor after finalization:",
        contributorAfterFinalization,
      );
      // tempAlign should be burned, align tokens should be increased
      expect(contributorAfterFinalization.tempAlign).to.equal(0); // All tempAlign burned

      // Now finalize the vote
      const finalizeVoteTx = await ctx.program.methods
        .finalizeVote()
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: testSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: testSubmissionPda,
          voteCommit: ctx.voteCommitPda,
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

      console.log("Finalize vote transaction:", finalizeVoteTx);

      // Check validator token balances after finalization
      const validatorAfterFinalization = await checkUserTokenBalances(
        ctx.validatorProfilePda,
        0,
      );
      console.log("Validator after finalization:", validatorAfterFinalization);

      // Verify locked tokens were released
      expect(
        validatorAfterFinalization.topicSpecificTokens.lockedTempRep,
      ).to.equal(0); // All tokens should be unlocked
      expect(validatorAfterFinalization.topicSpecificTokens.tempRep).to.equal(
        15,
      ); // 15 available tokens remain unchanged

      // If vote agreed with consensus (yes vote), tokens should be converted to permanent Rep
      const voteCommit = await ctx.program.account.voteCommit.fetch(
        ctx.voteCommitPda,
      );
      expect(voteCommit.finalized).to.be.true;

      // Since we voted yes and the submission was accepted (vote with consensus),
      // The 10 tempRep tokens we used should be converted to permanent Rep tokens
      // Validator should have exactly 25 from previous test + 10 from this test = 35 permanent Rep
      expect(validatorAfterFinalization.permanentRep).to.equal(35);

      // Also verify user3's vote was finalized with proper token handling
      // Finalize user3's vote
      const finalizeUser3VoteTx = await ctx.program.methods
        .finalizeVote()
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: testSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: testSubmissionPda,
          voteCommit: secondVoteCommitPda,
          validatorProfile: ctx.user3ProfilePda,
          validatorTempRepAccount: ctx.user3TempRepAccount,
          validatorRepAta: ctx.user3RepAta,
          tempRepMint: ctx.tempRepMintPda,
          repMint: ctx.repMintPda,
          authority: ctx.authorityKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.authorityKeypair])
        .rpc();

      console.log("Finalize user3 vote transaction:", finalizeUser3VoteTx);

      // Check user3's token balances after finalization
      const user3AfterFinalization = await checkUserTokenBalances(
        ctx.user3ProfilePda,
        0,
      );
      console.log("User3 after finalization:", user3AfterFinalization);
      expect(user3AfterFinalization.topicSpecificTokens.lockedTempRep).to.equal(
        0,
      ); // All tokens should be unlocked
      expect(user3AfterFinalization.tempRep).to.equal(14); // 14 available tokens remain unchanged
      // User3 voted with 36 tokens, so should have exactly 36 permanent Rep
      expect(user3AfterFinalization.permanentRep).to.equal(36);
    });

    it("Correctly tracks token amounts across multiple votes", async () => {
      // We've already tested multiple votes with validator and user3
      // This test just verifies the final balances after all operations

      const validatorAfterFinalization = await checkUserTokenBalances(
        ctx.validatorProfilePda,
        0,
      );
      // Validator should have no locked tokens
      expect(
        validatorAfterFinalization.topicSpecificTokens.lockedTempRep,
      ).to.equal(0);

      // Check final balances for user3
      const user3AfterFinalization = await checkUserTokenBalances(
        ctx.user3ProfilePda,
        0,
      );
      // User3 should have no locked tokens
      expect(user3AfterFinalization.topicSpecificTokens.lockedTempRep).to.equal(
        0,
      );

      // Verify total tempRep and permanent Rep balances
      // Validator should have 15 tempRep remaining and 35 permanent Rep
      expect(validatorAfterFinalization.topicSpecificTokens.tempRep).to.equal(
        15,
      );
      expect(validatorAfterFinalization.permanentRep).to.equal(35);

      // User3 should have 14 tempRep remaining and 36 permanent Rep
      expect(user3AfterFinalization.tempRep).to.equal(14);
      expect(user3AfterFinalization.permanentRep).to.equal(36);

      // Check contributor's align balance after all operations
      const contributorAfterFinalization = await checkUserTokenBalances(
        ctx.contributorProfilePda,
        0,
      );
      expect(contributorAfterFinalization.permanentAlign).to.equal(150);
      expect(contributorAfterFinalization.tempAlign).to.equal(0);
    });
  });
}
