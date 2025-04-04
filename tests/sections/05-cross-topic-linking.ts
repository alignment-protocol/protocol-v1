import { expect } from "chai";
import { web3 } from "@coral-xyz/anchor";
import { TestContext } from "../utils/test-setup";

export function runCrossTopicLinkingTests(ctx: TestContext): void {
  describe("Cross-Topic Linking", () => {
    it("Links the submission to the second topic", async () => {
      // Derive the cross-topic link PDA
      [ctx.crossTopicLinkPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("submission_topic_link"),
          ctx.submissionPda.toBuffer(),
          ctx.topic2Pda.toBuffer(),
        ],
        ctx.program.programId,
      );

      // Link the submission to the second topic
      const tx = await ctx.program.methods
        .linkSubmissionToTopic()
        .accounts({
          state: ctx.statePda,
          topic: ctx.topic2Pda,
          submission: ctx.submissionPda,
          submissionTopicLink: ctx.crossTopicLinkPda,
          authority: ctx.authorityKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair])
        .rpc();

      console.log("Cross-topic linking transaction signature:", tx);

      // Verify the cross-topic link was created correctly
      const linkAcc = await ctx.program.account.submissionTopicLink.fetch(
        ctx.crossTopicLinkPda,
      );
      expect(linkAcc.submission.toString()).to.equal(
        ctx.submissionPda.toString(),
      );
      expect(linkAcc.topic.toString()).to.equal(ctx.topic2Pda.toString());
      expect(linkAcc.status.pending).to.not.be.undefined; // Check that status is Pending
      expect(linkAcc.yesVotingPower.toNumber()).to.equal(0);
      expect(linkAcc.noVotingPower.toNumber()).to.equal(0);
      expect(linkAcc.totalCommittedVotes.toNumber()).to.equal(0);
      expect(linkAcc.totalRevealedVotes.toNumber()).to.equal(0);

      // Verify that the topic's submission count was incremented
      const topicAcc = await ctx.program.account.topic.fetch(ctx.topic2Pda);
      expect(topicAcc.submissionCount.toNumber()).to.equal(1);

      // Verify the state's submission count did NOT change when linking to another topic
      const stateAcc = await ctx.program.account.state.fetch(ctx.statePda);
      // Get the current submission count from the first test
      const submissionCount = stateAcc.submissionCount.toNumber();
      // Should still be 1 since we only created one submission and just linked it to another topic
      expect(submissionCount).to.equal(1);
    });
  });
}
