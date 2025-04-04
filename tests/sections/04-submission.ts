import { expect } from "chai";
import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { TestContext } from "../utils/test-setup";

export function runSubmissionTests(ctx: TestContext): void {
  describe("Submission", () => {
    it("Submits data to the first topic", async () => {
      // Get the current submission count from state before submission
      const stateAccBefore = await ctx.program.account.state.fetch(
        ctx.statePda,
      );
      const currentSubmissionCount = stateAccBefore.submissionCount.toNumber();
      console.log(
        "Current submission count BEFORE first submission:",
        currentSubmissionCount,
      );

      // Derive the submission PDA using the current submission count
      [ctx.submissionPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("submission"),
          new anchor.BN(currentSubmissionCount).toBuffer("le", 8),
        ],
        ctx.program.programId,
      );

      // Derive the submission-topic link PDA
      [ctx.submissionTopicLinkPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("submission_topic_link"),
          ctx.submissionPda.toBuffer(),
          ctx.topic1Pda.toBuffer(),
        ],
        ctx.program.programId,
      );

      // Submit data to the first topic
      const tx = await ctx.program.methods
        .submitDataToTopic(ctx.SUBMISSION_DATA)
        .accounts({
          state: ctx.statePda,
          topic: ctx.topic1Pda,
          tempAlignMint: ctx.tempAlignMintPda,
          contributorTempAlignAccount: ctx.contributorTempAlignAccount,
          submission: ctx.submissionPda,
          submissionTopicLink: ctx.submissionTopicLinkPda,
          contributorProfile: ctx.contributorProfilePda,
          contributor: ctx.contributorKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.contributorKeypair])
        .rpc();

      // Get the submission count after the first submission
      const afterSubmitState = await ctx.program.account.state.fetch(
        ctx.statePda,
      );
      console.log(
        "Submission count after first submission:",
        afterSubmitState.submissionCount.toNumber(),
      );

      console.log("Submit data transaction signature:", tx);

      // Verify the submission was created correctly
      const submissionAcc = await ctx.program.account.submission.fetch(
        ctx.submissionPda,
      );
      expect(submissionAcc.contributor.toString()).to.equal(
        ctx.contributorKeypair.publicKey.toString(),
      );
      expect(submissionAcc.dataReference).to.equal(ctx.SUBMISSION_DATA);

      // Verify the submission-topic link was created correctly
      const linkAcc = await ctx.program.account.submissionTopicLink.fetch(
        ctx.submissionTopicLinkPda,
      );
      expect(linkAcc.submission.toString()).to.equal(
        ctx.submissionPda.toString(),
      );
      expect(linkAcc.topic.toString()).to.equal(ctx.topic1Pda.toString());
      expect(linkAcc.status.pending).to.not.be.undefined; // Check that status is Pending
      expect(linkAcc.yesVotingPower.toNumber()).to.equal(0);
      expect(linkAcc.noVotingPower.toNumber()).to.equal(0);
      expect(linkAcc.totalCommittedVotes.toNumber()).to.equal(0);
      expect(linkAcc.totalRevealedVotes.toNumber()).to.equal(0);

      // Verify that tempAlign tokens were minted to the contributor's protocol-owned account
      const contributorTempAlignData = await getAccount(
        ctx.provider.connection,
        ctx.contributorTempAlignAccount,
      );
      expect(Number(contributorTempAlignData.amount)).to.equal(100); // Should match tokensToMint = 100

      // Verify that the submission count was incremented in state and topic
      // State account should have submission count of 1 (started at 0)
      const stateAcc = await ctx.program.account.state.fetch(ctx.statePda);
      expect(stateAcc.submissionCount.toNumber()).to.equal(1);

      const topicAcc = await ctx.program.account.topic.fetch(ctx.topic1Pda);
      expect(topicAcc.submissionCount.toNumber()).to.equal(1);

      // Verify the contributor's topic-specific token balance was updated
      const contributorProfile = await ctx.program.account.userProfile.fetch(
        ctx.contributorProfilePda,
      );
      const topicTokenEntry = contributorProfile.topicTokens.find(
        (pair) => pair.topicId.toNumber() === 0, // Topic ID 0
      );
      expect(topicTokenEntry).to.not.be.undefined;

      // Now that we've already checked that topicTokenEntry exists
      expect(topicTokenEntry.topicId.toNumber()).to.equal(0);
      expect(topicTokenEntry.token.tempAlignAmount.toNumber()).to.equal(100);
      expect(topicTokenEntry.token.tempRepAmount.toNumber()).to.equal(0);
    });
  });
}
