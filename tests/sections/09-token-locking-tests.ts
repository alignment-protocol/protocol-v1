import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { web3, BN } from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { TestContext } from "../utils/test-setup";
import * as crypto from "crypto";

export function runTokenLockingTests(ctx: TestContext): void {
  describe("Token Locking and Unlocking Flow", () => {
    // Setup helper function to set voting phases - RE-ADDED
    async function setupVotingPhase(
      phase: "commit" | "reveal" | "finalized",
      submissionTopicLinkPda: web3.PublicKey,
      submissionPdaToUse: web3.PublicKey,
    ) {
      const now = Math.floor(Date.now() / 1000);
      let commitPhaseStart, commitPhaseEnd, revealPhaseStart, revealPhaseEnd;

      if (phase === "commit") {
        commitPhaseStart = now - 60;
        commitPhaseEnd = now + 600;
        revealPhaseStart = commitPhaseEnd;
        revealPhaseEnd = commitPhaseEnd + 600;
      } else if (phase === "reveal") {
        commitPhaseStart = now - 1200;
        commitPhaseEnd = now - 60;
        revealPhaseStart = commitPhaseEnd;
        revealPhaseEnd = now + 600;
      } else {
        // finalized
        commitPhaseStart = now - 2400;
        commitPhaseEnd = now - 1800;
        revealPhaseStart = commitPhaseEnd;
        revealPhaseEnd = now - 60;
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
      return {
        commitPhaseStart,
        commitPhaseEnd,
        revealPhaseStart,
        revealPhaseEnd,
      };
    }

    // Helper to create a vote hash (remains the same) - RE-ADDED
    function createVoteHash(
      voter: web3.Keypair,
      submissionTopicLink: web3.PublicKey,
      choice: number,
      nonce: string,
    ) {
      const message = Buffer.concat([
        voter.publicKey.toBuffer(),
        submissionTopicLink.toBuffer(),
        Buffer.from([choice]), // 1 for Yes, 0 for No (as per on-chain enum)
        Buffer.from(nonce),
      ]);
      return Array.from(crypto.createHash("sha256").update(message).digest());
    }

    before("Initialize User3 balance and create test submissions", async () => {
      // Initialize UserTopicBalance for User3/Topic1
      [ctx.user3Topic1BalancePda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("user_topic_balance"),
          ctx.user3Keypair.publicKey.toBuffer(),
          ctx.topic1Pda.toBuffer(),
        ],
        ctx.program.programId,
      );
      try {
        await ctx.program.account.userTopicBalance.fetch(
          ctx.user3Topic1BalancePda,
        );
        console.log("User3 Topic 1 Balance account already exists.");
      } catch (error) {
        console.log("Initializing User3 Topic 1 Balance account...");
        const initTx = await ctx.program.methods
          .initializeUserTopicBalance()
          .accounts({
            user: ctx.user3Keypair.publicKey,
            topic: ctx.topic1Pda,
            payer: ctx.authorityKeypair.publicKey,
          })
          .signers([ctx.authorityKeypair])
          .rpc();
        console.log(" -> Initialize User3 Topic 1 Balance TX:", initTx);
      }

      console.log("=== Verifying Starting Balances (Post Section 08) ===");
      // --- Contributor ---
      const contribBalance08 = await ctx.program.account.userTopicBalance.fetch(
        ctx.contributorTopic1BalancePda,
      );
      const contribGlobalTempAlign08 = await getAccount(
        ctx.provider.connection,
        ctx.contributorTempAlignAccount,
      );
      const contribGlobalAlign08 = await getAccount(
        ctx.provider.connection,
        ctx.contributorAlignAta,
      );
      console.log(`Contributor Balances:`);
      console.log(
        ` -> UserTopicBalance: Align=${contribBalance08.tempAlignAmount.toNumber()}, Rep=${contribBalance08.tempRepAmount.toNumber()}, Locked=${contribBalance08.lockedTempRepAmount.toNumber()}`,
      );
      console.log(
        ` -> Global TempAlign: ${Number(contribGlobalTempAlign08.amount)}`,
      );
      console.log(` -> Global Align: ${Number(contribGlobalAlign08.amount)}`);
      // Expect: TopicBalance(Align=0, Rep=50), GlobalTempAlign=0, GlobalAlign=50
      expect(contribBalance08.tempAlignAmount.toNumber()).to.equal(0);
      expect(contribBalance08.tempRepAmount.toNumber()).to.equal(50);
      expect(contribBalance08.lockedTempRepAmount.toNumber()).to.equal(0);
      expect(Number(contribGlobalTempAlign08.amount)).to.equal(0);
      expect(Number(contribGlobalAlign08.amount)).to.equal(50);

      // --- Validator ---
      const validatorBalance08 =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.validatorTopic1BalancePda,
        );
      const validatorGlobalTempRep08 = await getAccount(
        ctx.provider.connection,
        ctx.validatorTempRepAccount,
      );
      const validatorGlobalRep08 = await getAccount(
        ctx.provider.connection,
        ctx.validatorRepAta,
      );
      console.log(`Validator Balances:`);
      console.log(
        ` -> UserTopicBalance: Align=${validatorBalance08.tempAlignAmount.toNumber()}, Rep=${validatorBalance08.tempRepAmount.toNumber()}, Locked=${validatorBalance08.lockedTempRepAmount.toNumber()}`,
      );
      console.log(
        ` -> Global TempRep: ${Number(validatorGlobalTempRep08.amount)}`,
      );
      console.log(` -> Global Rep: ${Number(validatorGlobalRep08.amount)}`);
      // Expect: TopicBalance(Align=50, Rep=25, Locked=0), GlobalTempRep=25, GlobalRep=25
      expect(validatorBalance08.tempAlignAmount.toNumber()).to.equal(50);
      expect(validatorBalance08.tempRepAmount.toNumber()).to.equal(25);
      expect(validatorBalance08.lockedTempRepAmount.toNumber()).to.equal(0);
      expect(Number(validatorGlobalTempRep08.amount)).to.equal(25);
      expect(Number(validatorGlobalRep08.amount)).to.equal(25);

      // --- User3 ---
      const user3Balance08 = await ctx.program.account.userTopicBalance.fetch(
        ctx.user3Topic1BalancePda,
      );
      const user3GlobalTempAlign08 = await getAccount(
        ctx.provider.connection,
        ctx.user3TempAlignAccount,
      );
      const user3GlobalTempRep08 = await getAccount(
        ctx.provider.connection,
        ctx.user3TempRepAccount,
      );
      const user3GlobalAlign08 = await getAccount(
        ctx.provider.connection,
        ctx.user3AlignAta,
      );
      const user3GlobalRep08 = await getAccount(
        ctx.provider.connection,
        ctx.user3RepAta,
      );
      console.log(`User3 Balances:`);
      console.log(
        ` -> UserTopicBalance: Align=${user3Balance08.tempAlignAmount.toNumber()}, Rep=${user3Balance08.tempRepAmount.toNumber()}, Locked=${user3Balance08.lockedTempRepAmount.toNumber()}`,
      );
      console.log(
        ` -> Global TempAlign: ${Number(user3GlobalTempAlign08.amount)}`,
      );
      console.log(` -> Global TempRep: ${Number(user3GlobalTempRep08.amount)}`);
      console.log(` -> Global Align: ${Number(user3GlobalAlign08.amount)}`);
      console.log(` -> Global Rep: ${Number(user3GlobalRep08.amount)}`);
      // Expect: All 0
      expect(user3Balance08.tempAlignAmount.toNumber()).to.equal(0);
      expect(user3Balance08.tempRepAmount.toNumber()).to.equal(0);
      expect(user3Balance08.lockedTempRepAmount.toNumber()).to.equal(0);
      expect(Number(user3GlobalTempAlign08.amount)).to.equal(0);
      expect(Number(user3GlobalTempRep08.amount)).to.equal(0);
      expect(Number(user3GlobalAlign08.amount)).to.equal(0);
      expect(Number(user3GlobalRep08.amount)).to.equal(0);

      // === Create Test Submissions ===
      const tokensToMint = (await ctx.program.account.state.fetch(ctx.statePda))
        .tokensToMint;

      // --- Contributor makes a new submission for these tests ---
      let contributorProfile = await ctx.program.account.userProfile.fetch(
        ctx.contributorProfilePda,
      );
      let testSubmissionIndex = contributorProfile.userSubmissionCount;
      console.log(
        `Contributor making test submission (index ${testSubmissionIndex.toNumber()})`,
      );
      [ctx.testSubmissionPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("submission"),
          ctx.contributorKeypair.publicKey.toBuffer(),
          testSubmissionIndex.toBuffer("le", 8),
        ],
        ctx.program.programId,
      );
      [ctx.testSubmissionTopicLinkPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("submission_topic_link"),
          ctx.testSubmissionPda.toBuffer(),
          ctx.topic1Pda.toBuffer(),
        ],
        ctx.program.programId,
      );

      const createSubmissionTx = await ctx.program.methods
        .submitDataToTopic(
          "Test submission for token locking tests",
          testSubmissionIndex,
        )
        .accounts({
          topic: ctx.topic1Pda,
          tempAlignMint: ctx.tempAlignMintPda,
          contributor: ctx.contributorKeypair.publicKey,
          payer: ctx.authorityKeypair.publicKey,
        })
        .signers([ctx.authorityKeypair])
        .rpc();
      console.log(" -> Contributor test submission TX:", createSubmissionTx);

      // Verify contributor balances updated
      const contribBalanceAfterTestSub =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.contributorTopic1BalancePda,
        );
      const contribGlobalTempAlignAfterTestSub = await getAccount(
        ctx.provider.connection,
        ctx.contributorTempAlignAccount,
      );
      expect(Number(contribGlobalTempAlignAfterTestSub.amount)).to.equal(
        tokensToMint.toNumber(),
      ); // Should receive new tokens
      expect(contribBalanceAfterTestSub.tempAlignAmount.toNumber()).to.equal(
        tokensToMint.toNumber(),
      ); // UserTopicBalance updated

      // --- User3 makes a submission ---
      let user3Profile = await ctx.program.account.userProfile.fetch(
        ctx.user3ProfilePda,
      );
      let user3SubmissionIndex = user3Profile.userSubmissionCount;
      console.log(
        `User3 making test submission (index ${user3SubmissionIndex.toNumber()})`,
      );
      [ctx.user3SubmissionPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("submission"),
          ctx.user3Keypair.publicKey.toBuffer(),
          user3SubmissionIndex.toBuffer("le", 8),
        ],
        ctx.program.programId,
      );
      [ctx.user3SubmissionTopicLinkPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("submission_topic_link"),
          ctx.user3SubmissionPda.toBuffer(),
          ctx.topic1Pda.toBuffer(),
        ],
        ctx.program.programId,
      );

      const user3SubmitTx = await ctx.program.methods
        .submitDataToTopic(
          "User3 submission to earn tempAlign",
          user3SubmissionIndex,
        )
        .accounts({
          topic: ctx.topic1Pda,
          tempAlignMint: ctx.tempAlignMintPda,
          contributor: ctx.user3Keypair.publicKey,
          payer: ctx.authorityKeypair.publicKey,
        })
        .signers([ctx.authorityKeypair])
        .rpc();
      console.log(" -> User3 submission TX:", user3SubmitTx);

      // Verify User3 balances updated
      const user3BalanceAfterTestSub =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.user3Topic1BalancePda,
        );
      const user3GlobalTempAlignAfterTestSub = await getAccount(
        ctx.provider.connection,
        ctx.user3TempAlignAccount,
      );
      expect(Number(user3GlobalTempAlignAfterTestSub.amount)).to.equal(
        tokensToMint.toNumber(),
      );
      expect(user3BalanceAfterTestSub.tempAlignAmount.toNumber()).to.equal(
        tokensToMint.toNumber(),
      );

      // --- User3 Stakes Tokens ---
      const user3StakeAmount = new BN(50);
      console.log(`User3 staking ${user3StakeAmount.toNumber()} tempAlign`);
      const user3StakeTx = await ctx.program.methods
        .stakeTopicSpecificTokens(user3StakeAmount)
        .accounts({
          state: ctx.statePda,
          topic: ctx.topic1Pda,
          userProfile: ctx.user3ProfilePda,
          userTopicBalance: ctx.user3Topic1BalancePda, // Account to update
          tempAlignMint: ctx.tempAlignMintPda,
          tempRepMint: ctx.tempRepMintPda,
          userTempAlignAccount: ctx.user3TempAlignAccount, // Source for burn
          userTempRepAccount: ctx.user3TempRepAccount, // Target for mint
          user: ctx.user3Keypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([ctx.user3Keypair])
        .rpc();
      console.log(" -> User3 staking TX:", user3StakeTx);

      // Verify User3 balances after staking
      const user3BalanceAfterStake =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.user3Topic1BalancePda,
        );
      const user3GlobalTempAlignAfterStake = await getAccount(
        ctx.provider.connection,
        ctx.user3TempAlignAccount,
      );
      const user3GlobalTempRepAfterStake = await getAccount(
        ctx.provider.connection,
        ctx.user3TempRepAccount,
      );
      expect(Number(user3GlobalTempAlignAfterStake.amount)).to.equal(
        tokensToMint.toNumber() - user3StakeAmount.toNumber(),
      ); // 50 left
      expect(Number(user3GlobalTempRepAfterStake.amount)).to.equal(
        user3StakeAmount.toNumber(),
      ); // 50 minted
      expect(user3BalanceAfterStake.tempAlignAmount.toNumber()).to.equal(
        tokensToMint.toNumber() - user3StakeAmount.toNumber(),
      ); // Balance reflects stake
      expect(user3BalanceAfterStake.tempRepAmount.toNumber()).to.equal(
        user3StakeAmount.toNumber(),
      );
      expect(user3BalanceAfterStake.lockedTempRepAmount.toNumber()).to.equal(0);

      // Make sure the *test* submission is in the commit phase
      await setupVotingPhase(
        "commit",
        ctx.testSubmissionTopicLinkPda,
        ctx.testSubmissionPda,
      );
    });

    it("Locks tokens when committing a vote", async () => {
      // Fetch validator balance before commit
      const balanceBefore = await ctx.program.account.userTopicBalance.fetch(
        ctx.validatorTopic1BalancePda,
      );
      console.log("Validator UserTopicBalance before commit:", balanceBefore);
      // Validator has 25 tempRep from previous tests (section 06/08 staking/finalization)
      expect(balanceBefore.tempRepAmount.toNumber()).to.equal(25);
      expect(balanceBefore.lockedTempRepAmount.toNumber()).to.equal(0);

      // Create vote hash for validator on the *test* submission
      const voteNonce = "test-nonce-lock-1";
      ctx.testVoteHash = createVoteHash(
        ctx.validatorKeypair,
        ctx.testSubmissionTopicLinkPda,
        1, // Choice for Yes (1 for Yes, 0 for No)
        voteNonce,
      );

      // Derive vote commit PDA for validator on *test* submission
      [ctx.testVoteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          ctx.testSubmissionTopicLinkPda.toBuffer(),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      const voteAmount = new BN(10);
      const isPermanentRep = false;
      console.log(
        `Validator committing ${voteAmount.toNumber()} tempRep on test submission ${ctx.testSubmissionPda.toBase58()}`,
      );

      // Commit the vote
      const tx = await ctx.program.methods
        .commitVote(ctx.testVoteHash, voteAmount, isPermanentRep)
        .accounts({
          topic: ctx.topic1Pda,
          submission: ctx.testSubmissionPda,
          validatorRepAta: ctx.validatorRepAta, // Needed even if false
          validator: ctx.validatorKeypair.publicKey,
          payer: ctx.authorityKeypair.publicKey,
        })
        .signers([ctx.authorityKeypair])
        .rpc();
      console.log(" -> Validator commit TX:", tx);

      // Verify balances after commit
      const balanceAfter = await ctx.program.account.userTopicBalance.fetch(
        ctx.validatorTopic1BalancePda,
      );
      console.log("Validator UserTopicBalance after commit:", balanceAfter);
      expect(balanceAfter.tempRepAmount.toNumber()).to.equal(
        balanceBefore.tempRepAmount.toNumber() - voteAmount.toNumber(),
      ); // 25 - 10 = 15
      expect(balanceAfter.lockedTempRepAmount.toNumber()).to.equal(
        balanceBefore.lockedTempRepAmount.toNumber() + voteAmount.toNumber(),
      ); // 0 + 10 = 10
    });

    it("Properly handles multiple votes and calculates quadratic voting power", async () => {
      // Fetch user3 balance before commit
      const user3BalanceBefore =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.user3Topic1BalancePda,
        );
      console.log("User3 UserTopicBalance before commit:", user3BalanceBefore);
      // User3 has 50 tempRep from staking in the `before` block
      expect(user3BalanceBefore.tempRepAmount.toNumber()).to.equal(50);
      expect(user3BalanceBefore.lockedTempRepAmount.toNumber()).to.equal(0);

      // Create vote hash for user3 on the *test* submission
      const voteNonce = "user3-nonce-lock";
      ctx.user3VoteHash = createVoteHash(
        ctx.user3Keypair,
        ctx.testSubmissionTopicLinkPda,
        1, // Choice for Yes (1 for Yes, 0 for No)
        voteNonce,
      );

      // Derive vote commit PDA for user3 on *test* submission
      [ctx.user3VoteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          ctx.testSubmissionTopicLinkPda.toBuffer(),
          ctx.user3Keypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      const voteAmount = new BN(36); // sqrt(36) = 6 voting power
      const isPermanentRep = false;
      console.log(
        `User3 committing ${voteAmount.toNumber()} tempRep on test submission ${ctx.testSubmissionPda.toBase58()}`,
      );

      // Commit user3's vote
      const tx = await ctx.program.methods
        .commitVote(ctx.user3VoteHash, voteAmount, isPermanentRep)
        .accounts({
          topic: ctx.topic1Pda,
          submission: ctx.testSubmissionPda,
          validatorRepAta: ctx.user3RepAta, // Needed even if false
          validator: ctx.user3Keypair.publicKey,
          payer: ctx.authorityKeypair.publicKey,
        })
        .signers([ctx.authorityKeypair])
        .rpc();
      console.log(" -> User3 commit TX:", tx);

      // Verify User3 balances after commit
      const user3BalanceAfter =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.user3Topic1BalancePda,
        );
      console.log("User3 UserTopicBalance after commit:", user3BalanceAfter);
      expect(user3BalanceAfter.tempRepAmount.toNumber()).to.equal(
        user3BalanceBefore.tempRepAmount.toNumber() - voteAmount.toNumber(),
      ); // 50 - 36 = 14
      expect(user3BalanceAfter.lockedTempRepAmount.toNumber()).to.equal(
        user3BalanceBefore.lockedTempRepAmount.toNumber() +
          voteAmount.toNumber(),
      ); // 0 + 36 = 36

      // --- Reveal Phase ---
      await setupVotingPhase(
        "reveal",
        ctx.testSubmissionTopicLinkPda,
        ctx.testSubmissionPda,
      );

      // Reveal validator's vote
      console.log("Revealing validator's vote (10 tokens)");
      const revealTx1 = await ctx.program.methods
        .revealVote(ctx.VOTE_CHOICE_YES, "test-nonce-lock-1")
        .accounts({
          topic: ctx.topic1Pda,
          submission: ctx.testSubmissionPda,
          validator: ctx.validatorKeypair.publicKey,
          payer: ctx.authorityKeypair.publicKey,
        })
        .signers([ctx.authorityKeypair])
        .rpc();
      console.log(" -> Validator reveal TX:", revealTx1);

      // Reveal user3's vote
      console.log("Revealing user3's vote (36 tokens)");
      const revealTx2 = await ctx.program.methods
        .revealVote(ctx.VOTE_CHOICE_YES, "user3-nonce-lock")
        .accounts({
          topic: ctx.topic1Pda,
          submission: ctx.testSubmissionPda,
          validator: ctx.user3Keypair.publicKey,
          payer: ctx.authorityKeypair.publicKey,
        })
        .signers([ctx.authorityKeypair])
        .rpc();
      console.log(" -> User3 reveal TX:", revealTx2);

      // Check quadratic voting power on the link account
      const linkAcc = await ctx.program.account.submissionTopicLink.fetch(
        ctx.testSubmissionTopicLinkPda,
      );
      console.log("Link account after reveals:", linkAcc);
      const expectedPowerValidator = Math.floor(Math.sqrt(10)); // sqrt(10) ~= 3
      const expectedPowerUser3 = Math.floor(Math.sqrt(36)); // sqrt(36) = 6
      console.log(
        `Expected voting power: Validator=${expectedPowerValidator}, User3=${expectedPowerUser3}`,
      );
      expect(linkAcc.yesVotingPower.toNumber()).to.equal(
        expectedPowerValidator + expectedPowerUser3,
      ); // 3 + 6 = 9
      expect(linkAcc.noVotingPower.toNumber()).to.equal(0);
      expect(linkAcc.totalCommittedVotes.toNumber()).to.equal(2); // Both committed
      expect(linkAcc.totalRevealedVotes.toNumber()).to.equal(2); // Both revealed

      // Check balances after reveal - locked amounts should remain until finalization
      const validatorBalanceAfterReveal =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.validatorTopic1BalancePda,
        );
      console.log(
        "Validator UserTopicBalance after reveal:",
        validatorBalanceAfterReveal,
      );
      expect(validatorBalanceAfterReveal.tempRepAmount.toNumber()).to.equal(15); // Still 15 available
      expect(
        validatorBalanceAfterReveal.lockedTempRepAmount.toNumber(),
      ).to.equal(10); // Still 10 locked

      const user3BalanceAfterReveal =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.user3Topic1BalancePda,
        );
      console.log(
        "User3 UserTopicBalance after reveal:",
        user3BalanceAfterReveal,
      );
      expect(user3BalanceAfterReveal.tempRepAmount.toNumber()).to.equal(14); // Still 14 available
      expect(user3BalanceAfterReveal.lockedTempRepAmount.toNumber()).to.equal(
        36,
      ); // Still 36 locked
    });

    it("Unlocks tokens after vote finalization", async () => {
      // --- Finalization Phase ---
      await setupVotingPhase(
        "finalized",
        ctx.testSubmissionTopicLinkPda,
        ctx.testSubmissionPda,
      );

      // Fetch balances before finalization
      const validatorBalanceBefore =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.validatorTopic1BalancePda,
        );
      const user3BalanceBefore =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.user3Topic1BalancePda,
        );
      const contribBalanceBefore =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.contributorTopic1BalancePda,
        );
      const contribGlobalAlignBefore = await getAccount(
        ctx.provider.connection,
        ctx.contributorAlignAta,
      );
      const validatorGlobalRepBefore = await getAccount(
        ctx.provider.connection,
        ctx.validatorRepAta,
      );
      const user3GlobalRepBefore = await getAccount(
        ctx.provider.connection,
        ctx.user3RepAta,
      );

      console.log("--- Before Finalization ---");
      console.log("Validator Balance:", validatorBalanceBefore);
      console.log("User3 Balance:", user3BalanceBefore);
      console.log("Contributor Balance:", contribBalanceBefore);

      // Finalize the submission first (contributor gets Align tokens)
      console.log(
        `Finalizing test submission ${ctx.testSubmissionPda.toBase58()}`,
      );
      const finalizeSubTx = await ctx.program.methods
        .finalizeSubmission()
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: ctx.testSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: ctx.testSubmissionPda,
          contributorProfile: ctx.contributorProfilePda,
          userTopicBalance: ctx.contributorTopic1BalancePda, // Contributor's balance updated
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
      console.log(" -> Finalize Submission TX:", finalizeSubTx);

      // Verify contributor got Align tokens
      const contribBalanceAfter =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.contributorTopic1BalancePda,
        );
      const contribGlobalAlignAfter = await getAccount(
        ctx.provider.connection,
        ctx.contributorAlignAta,
      );
      const tokensMinted = (
        await ctx.program.account.state.fetch(ctx.statePda)
      ).tokensToMint.toNumber();
      expect(contribBalanceAfter.tempAlignAmount.toNumber()).to.equal(0); // Entitlement consumed
      expect(Number(contribGlobalAlignAfter.amount)).to.equal(
        Number(contribGlobalAlignBefore.amount) + tokensMinted,
      ); // Received 100 Align

      // Finalize validator's vote
      console.log(
        `Finalizing validator's vote on test submission (VoteCommit: ${ctx.testVoteCommitPda.toBase58()})`,
      );
      const finalizeVoteTxVal = await ctx.program.methods
        .finalizeVote()
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: ctx.testSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: ctx.testSubmissionPda,
          voteCommit: ctx.testVoteCommitPda, // Validator's vote commit for test sub
          validatorProfile: ctx.validatorProfilePda,
          userTopicBalance: ctx.validatorTopic1BalancePda, // Validator's balance updated
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
      console.log(" -> Finalize Validator Vote TX:", finalizeVoteTxVal);

      // Finalize user3's vote
      console.log(
        `Finalizing user3's vote on test submission (VoteCommit: ${ctx.user3VoteCommitPda.toBase58()})`,
      );
      const finalizeVoteTxU3 = await ctx.program.methods
        .finalizeVote()
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: ctx.testSubmissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: ctx.testSubmissionPda,
          voteCommit: ctx.user3VoteCommitPda, // User3's vote commit for test sub
          validatorProfile: ctx.user3ProfilePda, // User3's profile
          userTopicBalance: ctx.user3Topic1BalancePda, // User3's balance updated
          validatorTempRepAccount: ctx.user3TempRepAccount, // User3's temp rep account
          validatorRepAta: ctx.user3RepAta, // User3's rep ATA
          tempRepMint: ctx.tempRepMintPda,
          repMint: ctx.repMintPda,
          authority: ctx.authorityKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.authorityKeypair])
        .rpc();
      console.log(" -> Finalize User3 Vote TX:", finalizeVoteTxU3);

      // --- Verification ---
      // Check validator balances after finalization
      const validatorBalanceAfter =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.validatorTopic1BalancePda,
        );
      const validatorGlobalRepAfter = await getAccount(
        ctx.provider.connection,
        ctx.validatorRepAta,
      );
      console.log(
        "Validator UserTopicBalance after finalization:",
        validatorBalanceAfter,
      );
      console.log(
        "Validator Global Rep ATA after finalization:",
        Number(validatorGlobalRepAfter.amount),
      );
      expect(validatorBalanceAfter.lockedTempRepAmount.toNumber()).to.equal(0); // Unlocked
      expect(validatorBalanceAfter.tempRepAmount.toNumber()).to.equal(
        validatorBalanceBefore.tempRepAmount.toNumber(),
      ); // Available unchanged
      // Voted 10 tempRep, submission accepted -> +10 permanent Rep
      expect(Number(validatorGlobalRepAfter.amount)).to.equal(
        Number(validatorGlobalRepBefore.amount) + 10,
      );

      // Check user3 balances after finalization
      const user3BalanceAfter =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.user3Topic1BalancePda,
        );
      const user3GlobalRepAfter = await getAccount(
        ctx.provider.connection,
        ctx.user3RepAta,
      );
      console.log(
        "User3 UserTopicBalance after finalization:",
        user3BalanceAfter,
      );
      console.log(
        "User3 Global Rep ATA after finalization:",
        Number(user3GlobalRepAfter.amount),
      );
      expect(user3BalanceAfter.lockedTempRepAmount.toNumber()).to.equal(0); // Unlocked
      expect(user3BalanceAfter.tempRepAmount.toNumber()).to.equal(
        user3BalanceBefore.tempRepAmount.toNumber(),
      ); // Available unchanged
      // Voted 36 tempRep, submission accepted -> +36 permanent Rep
      expect(Number(user3GlobalRepAfter.amount)).to.equal(
        Number(user3GlobalRepBefore.amount) + 36,
      );

      // Verify vote commits are finalized
      const voteCommitVal = await ctx.program.account.voteCommit.fetch(
        ctx.testVoteCommitPda,
      );
      const voteCommitU3 = await ctx.program.account.voteCommit.fetch(
        ctx.user3VoteCommitPda,
      );
      expect(voteCommitVal.finalized).to.be.true;
      expect(voteCommitU3.finalized).to.be.true;
    });
  });
}
