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

      // Fetch topic 2 state BEFORE linking
      const topic2AccBefore = await ctx.program.account.topic.fetch(
        ctx.topic2Pda,
      );
      console.log(
        "Topic 2 submission count BEFORE linking:",
        topic2AccBefore.submissionCount.toNumber(),
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
      expect(linkAcc.status.pending).to.not.be.undefined;
      expect(linkAcc.yesVotingPower.toNumber()).to.equal(0);
      expect(linkAcc.noVotingPower.toNumber()).to.equal(0);
      expect(linkAcc.totalCommittedVotes.toNumber()).to.equal(0);
      expect(linkAcc.totalRevealedVotes.toNumber()).to.equal(0);
      // Verify phase timestamps were set based on Topic 2 defaults
      const submissionAcc = await ctx.program.account.submission.fetch(
        ctx.submissionPda,
      ); // Fetch submission to get timestamp if needed, or use Clock sysvar in instruction
      const now = Math.floor(Date.now() / 1000); // Approximate link creation time
      const expectedCommitEnd =
        now + topic2AccBefore.commitPhaseDuration.toNumber();
      const expectedRevealStart = expectedCommitEnd;
      const expectedRevealEnd =
        expectedRevealStart + topic2AccBefore.revealPhaseDuration.toNumber();
      // Note: The instruction currently doesn't set timestamps for linking, only for initial submission.
      // We might need to adjust the instruction or this test depending on desired behavior.
      // For now, we'll check they are non-zero or update if the instruction changes.
      // Let's assume for now they SHOULD be set upon linking. We'll refine if needed.
      expect(linkAcc.commitPhaseStart.toNumber()).to.be.closeTo(now, 60); // Should be set around tx time
      expect(linkAcc.commitPhaseEnd.toNumber()).to.be.closeTo(
        expectedCommitEnd,
        60,
      );
      expect(linkAcc.revealPhaseStart.toNumber()).to.be.closeTo(
        expectedRevealStart,
        60,
      );
      expect(linkAcc.revealPhaseEnd.toNumber()).to.be.closeTo(
        expectedRevealEnd,
        60,
      );

      // Verify that the topic's submission count was incremented
      const topic2AccAfter = await ctx.program.account.topic.fetch(
        ctx.topic2Pda,
      );
      console.log(
        "Topic 2 submission count AFTER linking:",
        topic2AccAfter.submissionCount.toNumber(),
      );
      expect(topic2AccAfter.submissionCount.toNumber()).to.equal(
        topic2AccBefore.submissionCount.toNumber() + 1,
      );
    });
  });
}
