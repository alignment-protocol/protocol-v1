import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { web3, BN } from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { TestContext } from "../utils/test-setup";

export function runFinalizationTests(ctx: TestContext): void {
  describe("Finalization", () => {
    it("Finalizes the submission", async () => {
      // Fetch state before setting phases
      let linkAccBefore = await ctx.program.account.submissionTopicLink.fetch(
        ctx.submissionTopicLinkPda,
      );
      console.log(
        "Link account status before setting phases for finalization:",
        linkAccBefore.status,
      );
      console.log(
        " -> Reveal phase end before:",
        linkAccBefore.revealPhaseEnd.toNumber(),
      );

      // Adjust voting phases to simulate being past the reveal phase
      const now = Math.floor(Date.now() / 1000);
      const commitStart = now - 2400; // 40 mins ago
      const commitEnd = now - 1800; // 30 mins ago
      const revealStart = commitEnd;
      const revealEnd = now - 600; // 10 mins ago (ended)

      console.log(
        `Setting phases for finalization: Commit ${commitStart}-${commitEnd}, Reveal ${revealStart}-${revealEnd}`,
      );
      const setPhasesTx = await ctx.program.methods
        .setVotingPhases(
          new anchor.BN(commitStart),
          new anchor.BN(commitEnd),
          new anchor.BN(revealStart),
          new anchor.BN(revealEnd),
        )
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: ctx.submissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: ctx.submissionPda,
          authority: ctx.authorityKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.authorityKeypair])
        .rpc();
      console.log(
        "Set voting phases for finalization transaction signature:",
        setPhasesTx,
      );

      // Verify phases are set and reveal phase is ended
      linkAccBefore = await ctx.program.account.submissionTopicLink.fetch(
        ctx.submissionTopicLinkPda,
      );
      expect(linkAccBefore.revealPhaseEnd.toNumber()).to.equal(revealEnd);
      expect(linkAccBefore.revealPhaseEnd.toNumber()).to.be.lessThan(now); // Check it's actually in the past
      expect(linkAccBefore.status.pending).to.not.be.undefined; // Should still be pending before finalize call

      // Fetch balances before finalization
      const balanceBefore = await ctx.program.account.userTopicBalance.fetch(
        ctx.contributorTopic1BalancePda,
      );
      const globalTempAlignBefore = await getAccount(
        ctx.provider.connection,
        ctx.contributorTempAlignAccount,
      );
      const globalAlignBefore = await getAccount(
        ctx.provider.connection,
        ctx.contributorAlignAta,
      );

      console.log("--- Before Finalize Submission ---");
      console.log(
        `Contributor UserTopicBalance: Align=${balanceBefore.tempAlignAmount.toNumber()}, Rep=${balanceBefore.tempRepAmount.toNumber()}`,
      );
      console.log(
        `Contributor Global TempAlign ATA: ${Number(globalTempAlignBefore.amount)}`,
      );
      console.log(
        `Contributor Global Align ATA: ${Number(globalAlignBefore.amount)}`,
      );

      // Finalize the submission - *** ADDED userTopicBalance ***
      const tx = await ctx.program.methods
        .finalizeSubmission()
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: ctx.submissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: ctx.submissionPda,
          contributorProfile: ctx.contributorProfilePda, // Still needed for constraints
          userTopicBalance: ctx.contributorTopic1BalancePda, // ADDED
          contributorTempAlignAccount: ctx.contributorTempAlignAccount,
          contributorAlignAta: ctx.contributorAlignAta,
          tempAlignMint: ctx.tempAlignMintPda,
          alignMint: ctx.alignMintPda,
          authority: ctx.authorityKeypair.publicKey, // Payer/caller
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.authorityKeypair])
        .rpc();

      console.log("Finalize submission transaction signature:", tx);

      // --- Verification ---
      // Verify the submission-topic link status changed
      const linkAccAfter = await ctx.program.account.submissionTopicLink.fetch(
        ctx.submissionTopicLinkPda,
      );
      console.log(
        "Link account status after finalization:",
        linkAccAfter.status,
      );
      expect(linkAccAfter.status.pending).to.be.undefined;
      expect(linkAccAfter.status.accepted).to.not.be.undefined; // Should be accepted based on previous 'Yes' vote

      // Verify token conversion by checking global accounts
      const globalTempAlignAfter = await getAccount(
        ctx.provider.connection,
        ctx.contributorTempAlignAccount,
      );
      const globalAlignAfter = await getAccount(
        ctx.provider.connection,
        ctx.contributorAlignAta,
      );

      console.log("--- After Finalize Submission ---");
      console.log(
        `Contributor Global TempAlign ATA: ${Number(globalTempAlignAfter.amount)}`,
      );
      console.log(
        `Contributor Global Align ATA: ${Number(globalAlignAfter.amount)}`,
      );

      // Contributor started with 100 tempAlign (tokensToMint), staked 50, leaving 50.
      // Finalization should burn the remaining 50 tempAlign and mint 50 permanent Align.
      const expectedConversionAmount = Number(globalTempAlignBefore.amount); // Amount before finalize was 50
      expect(Number(globalTempAlignAfter.amount)).to.equal(0); // All tempAlign should be gone
      expect(Number(globalAlignAfter.amount)).to.equal(
        Number(globalAlignBefore.amount) + expectedConversionAmount,
      ); // Align increases

      // Verify the contributor's UserTopicBalance was updated
      const balanceAfter = await ctx.program.account.userTopicBalance.fetch(
        ctx.contributorTopic1BalancePda,
      );
      console.log(
        `Contributor UserTopicBalance: Align=${balanceAfter.tempAlignAmount.toNumber()}, Rep=${balanceAfter.tempRepAmount.toNumber()}`,
      );
      expect(balanceAfter.tempAlignAmount.toNumber()).to.equal(0); // tempAlign entitlement consumed
      expect(balanceAfter.tempRepAmount.toNumber()).to.equal(
        balanceBefore.tempRepAmount.toNumber(),
      ); // tempRep unaffected

      // REMOVED Check for contributorProfile.topicTokens
    });

    it("Finalizes the vote", async () => {
      // Fetch state before finalizing vote
      const voteCommitBefore = await ctx.program.account.voteCommit.fetch(
        ctx.voteCommitPda,
      );
      const balanceBefore = await ctx.program.account.userTopicBalance.fetch(
        ctx.validatorTopic1BalancePda,
      );
      const globalTempRepBefore = await getAccount(
        ctx.provider.connection,
        ctx.validatorTempRepAccount,
      );
      const globalRepBefore = await getAccount(
        ctx.provider.connection,
        ctx.validatorRepAta,
      );

      console.log("--- Before Finalize Vote ---");
      console.log("VoteCommit finalized status:", voteCommitBefore.finalized);
      console.log(
        `Validator UserTopicBalance: Align=${balanceBefore.tempAlignAmount.toNumber()}, Rep=${balanceBefore.tempRepAmount.toNumber()}, Locked=${balanceBefore.lockedTempRepAmount.toNumber()}`,
      );
      console.log(
        `Validator Global TempRep ATA: ${Number(globalTempRepBefore.amount)}`,
      );
      console.log(
        `Validator Global Rep ATA: ${Number(globalRepBefore.amount)}`,
      );

      // Verify the submission link is no longer pending
      const linkAcc = await ctx.program.account.submissionTopicLink.fetch(
        ctx.submissionTopicLinkPda,
      );
      expect(linkAcc.status.pending).to.be.undefined;
      console.log("Submission-topic link status:", linkAcc.status);

      // Finalize the vote - *** ADDED userTopicBalance ***
      const tx = await ctx.program.methods
        .finalizeVote()
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: ctx.submissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: ctx.submissionPda,
          voteCommit: ctx.voteCommitPda,
          validatorProfile: ctx.validatorProfilePda, // Still needed for constraints
          userTopicBalance: ctx.validatorTopic1BalancePda, // ADDED
          validatorTempRepAccount: ctx.validatorTempRepAccount,
          validatorRepAta: ctx.validatorRepAta,
          tempRepMint: ctx.tempRepMintPda,
          repMint: ctx.repMintPda,
          authority: ctx.authorityKeypair.publicKey, // Payer/caller
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.authorityKeypair])
        .rpc();

      console.log("Finalize vote transaction signature:", tx);

      // --- Verification ---
      // Verify the vote commit was finalized
      const voteCommitAfter = await ctx.program.account.voteCommit.fetch(
        ctx.voteCommitPda,
      );
      console.log("--- After Finalize Vote ---");
      console.log("VoteCommit finalized status:", voteCommitAfter.finalized);
      expect(voteCommitAfter.finalized).to.be.true;

      // Verify token conversion (tempRep burned, Rep minted)
      const globalTempRepAfter = await getAccount(
        ctx.provider.connection,
        ctx.validatorTempRepAccount,
      );
      const globalRepAfter = await getAccount(
        ctx.provider.connection,
        ctx.validatorRepAta,
      );

      console.log(
        `Validator Global TempRep ATA: ${Number(globalTempRepAfter.amount)}`,
      );
      console.log(`Validator Global Rep ATA: ${Number(globalRepAfter.amount)}`);

      // Validator committed 25 tempRep. Since vote was 'Yes' and submission 'Accepted',
      // the 25 tempRep should be burned and 25 permanent Rep should be minted.
      const expectedConversionAmount =
        voteCommitBefore.tempRepAmount.toNumber(); // 25 from commit
      // Check tempRep decreased (burned)
      expect(Number(globalTempRepAfter.amount)).to.equal(
        Number(globalTempRepBefore.amount) - expectedConversionAmount,
      );
      // Check Rep increased (minted)
      expect(Number(globalRepAfter.amount)).to.equal(
        Number(globalRepBefore.amount) + expectedConversionAmount,
      );

      // Verify the validator's UserTopicBalance was updated
      const balanceAfter = await ctx.program.account.userTopicBalance.fetch(
        ctx.validatorTopic1BalancePda,
      );
      console.log(
        `Validator UserTopicBalance: Align=${balanceAfter.tempAlignAmount.toNumber()}, Rep=${balanceAfter.tempRepAmount.toNumber()}, Locked=${balanceAfter.lockedTempRepAmount.toNumber()}`,
      );
      // Locked amount should become 0
      expect(balanceAfter.lockedTempRepAmount.toNumber()).to.equal(0);
      // Available tempRep amount should remain unchanged, as the locked amount was processed
      expect(balanceAfter.tempRepAmount.toNumber()).to.equal(
        balanceBefore.tempRepAmount.toNumber(),
      );

      // REMOVED Check for validatorProfile.permanentRepAmount
      // REMOVED Check for validatorProfile.topicTokens
    });
  });
}
