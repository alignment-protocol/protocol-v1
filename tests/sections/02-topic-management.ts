import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { web3 } from "@coral-xyz/anchor";
import { TestContext } from "../utils/test-setup";

export function runTopicManagementTests(ctx: TestContext): void {
  describe("Topic Management", () => {
    it("Creates the first topic", async () => {
      // Derive the topic PDA for the first topic (ID = 0)
      [ctx.topic1Pda] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("topic"), Buffer.from([0, 0, 0, 0, 0, 0, 0, 0])],
        ctx.program.programId
      );

      // Create the first topic
      const tx = await ctx.program.methods
        .createTopic(
          ctx.TOPIC1_NAME,
          ctx.TOPIC1_DESCRIPTION,
          null, // Use default commit phase duration
          null // Use default reveal phase duration
        )
        .accounts({
          state: ctx.statePda,
          topic: ctx.topic1Pda,
          authority: ctx.authorityKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair])
        .rpc();

      console.log("Create first topic transaction signature:", tx);

      // Fetch and verify the topic data
      const topicAcc = await ctx.program.account.topic.fetch(ctx.topic1Pda);
      expect(topicAcc.id.toNumber()).to.equal(0);
      expect(topicAcc.name).to.equal(ctx.TOPIC1_NAME);
      expect(topicAcc.description).to.equal(ctx.TOPIC1_DESCRIPTION);
      expect(topicAcc.authority.toString()).to.equal(
        ctx.authorityKeypair.publicKey.toString()
      );
      expect(topicAcc.submissionCount.toNumber()).to.equal(0);
      expect(topicAcc.isActive).to.be.true;

      // Verify that the topic count in state was incremented
      const stateAcc = await ctx.program.account.state.fetch(ctx.statePda);
      expect(stateAcc.topicCount.toNumber()).to.equal(1);

      // Verify the default durations were set correctly
      expect(topicAcc.commitPhaseDuration.toNumber()).to.equal(
        stateAcc.defaultCommitPhaseDuration.toNumber()
      );
      expect(topicAcc.revealPhaseDuration.toNumber()).to.equal(
        stateAcc.defaultRevealPhaseDuration.toNumber()
      );
    });

    it("Creates a second topic", async () => {
      // Derive the topic PDA for the second topic (ID = 1)
      [ctx.topic2Pda] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("topic"), Buffer.from([1, 0, 0, 0, 0, 0, 0, 0])],
        ctx.program.programId
      );

      // Create the second topic with custom phase durations
      const customCommitDuration = 12 * 60 * 60; // 12 hours in seconds
      const customRevealDuration = 12 * 60 * 60; // 12 hours in seconds

      const tx = await ctx.program.methods
        .createTopic(
          ctx.TOPIC2_NAME,
          ctx.TOPIC2_DESCRIPTION,
          new anchor.BN(customCommitDuration),
          new anchor.BN(customRevealDuration)
        )
        .accounts({
          state: ctx.statePda,
          topic: ctx.topic2Pda,
          authority: ctx.authorityKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair])
        .rpc();

      console.log("Create second topic transaction signature:", tx);

      // Fetch and verify the topic data
      const topicAcc = await ctx.program.account.topic.fetch(ctx.topic2Pda);
      expect(topicAcc.id.toNumber()).to.equal(1);
      expect(topicAcc.name).to.equal(ctx.TOPIC2_NAME);
      expect(topicAcc.description).to.equal(ctx.TOPIC2_DESCRIPTION);
      expect(topicAcc.authority.toString()).to.equal(
        ctx.authorityKeypair.publicKey.toString()
      );
      expect(topicAcc.submissionCount.toNumber()).to.equal(0);
      expect(topicAcc.isActive).to.be.true;

      // Verify that the topic count in state was incremented
      const stateAcc = await ctx.program.account.state.fetch(ctx.statePda);
      expect(stateAcc.topicCount.toNumber()).to.equal(2);

      // Verify the custom durations were set correctly
      expect(topicAcc.commitPhaseDuration.toNumber()).to.equal(
        customCommitDuration
      );
      expect(topicAcc.revealPhaseDuration.toNumber()).to.equal(
        customRevealDuration
      );
    });
  });
}
