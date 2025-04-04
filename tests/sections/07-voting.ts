import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { web3 } from "@coral-xyz/anchor";
import { TestContext } from "../utils/test-setup";
import * as crypto from "crypto";

export function runVotingTests(ctx: TestContext): void {
  describe("Voting", () => {
    it("Commits a vote on the submission", async () => {
      // Need to adjust voting phases for testing purposes, since we're not waiting for real time in tests
      const now = Math.floor(Date.now() / 1000); // Current time in seconds
      const commitPhaseEnd = now + 600; // 10 minutes from now
      const revealPhaseEnd = commitPhaseEnd + 600; // 10 minutes after commit phase

      // Set the voting phases to make sure we're within the commit phase
      const setPhasesTx = await ctx.program.methods
        .setVotingPhases(
          new anchor.BN(now - 60), // Start 1 minute ago
          new anchor.BN(commitPhaseEnd),
          new anchor.BN(commitPhaseEnd),
          new anchor.BN(revealPhaseEnd),
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

      console.log("Set voting phases transaction signature:", setPhasesTx);

      // Calculate vote hash from vote choice, nonce, validator and submission-topic link
      const message = Buffer.concat([
        ctx.validatorKeypair.publicKey.toBuffer(),
        ctx.submissionTopicLinkPda.toBuffer(),
        Buffer.from([0]), // Yes vote is 0
        Buffer.from(ctx.VOTE_NONCE),
      ]);

      // Using node's crypto module for hashing
      const voteHash = Array.from(
        crypto.createHash("sha256").update(message).digest(),
      );
      ctx.voteHash = voteHash;

      // Derive the vote commit PDA
      [ctx.voteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          ctx.submissionTopicLinkPda.toBuffer(),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );

      // Define vote amount
      const voteAmount = 25; // Half of the staked tempRep from earlier
      const isPermanentRep = false; // Use temporary reputation

      // Commit the vote
      const tx = await ctx.program.methods
        .commitVote(voteHash, new anchor.BN(voteAmount), isPermanentRep)
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: ctx.submissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: ctx.submissionPda,
          voteCommit: ctx.voteCommitPda,
          userProfile: ctx.validatorProfilePda,
          validator: ctx.validatorKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.validatorKeypair])
        .rpc();

      console.log("Vote commit transaction signature:", tx);

      // Verify the vote commit was created correctly
      const voteCommitAcc = await ctx.program.account.voteCommit.fetch(
        ctx.voteCommitPda,
      );
      expect(voteCommitAcc.submissionTopicLink.toString()).to.equal(
        ctx.submissionTopicLinkPda.toString(),
      );
      expect(voteCommitAcc.validator.toString()).to.equal(
        ctx.validatorKeypair.publicKey.toString(),
      );

      // Compare vote hash
      const fetchedHashArray = Array.from(voteCommitAcc.voteHash);
      expect(fetchedHashArray).to.deep.equal(voteHash);

      expect(voteCommitAcc.revealed).to.be.false;
      expect(voteCommitAcc.finalized).to.be.false;
      expect(voteCommitAcc.voteChoice).to.be.null;
      expect(voteCommitAcc.voteAmount.toNumber()).to.equal(voteAmount);
      expect(voteCommitAcc.isPermanentRep).to.equal(isPermanentRep);

      // Verify the submission-topic link vote count was incremented
      const linkAcc = await ctx.program.account.submissionTopicLink.fetch(
        ctx.submissionTopicLinkPda,
      );
      expect(linkAcc.totalCommittedVotes.toNumber()).to.equal(1);
      expect(linkAcc.totalRevealedVotes.toNumber()).to.equal(0);
    });

    it("Reveals the committed vote", async () => {
      // Need to adjust voting phases to make sure we're in the reveal phase
      const now = Math.floor(Date.now() / 1000); // Current time in seconds
      const commitPhaseStart = now - 1200; // 20 minutes ago
      const commitPhaseEnd = now - 600; // 10 minutes ago
      const revealPhaseStart = commitPhaseEnd; // Reveal phase starts when commit phase ends
      const revealPhaseEnd = now + 600; // 10 minutes from now

      // Set the voting phases to make sure we're within the reveal phase
      const setPhasesTx = await ctx.program.methods
        .setVotingPhases(
          new anchor.BN(commitPhaseStart),
          new anchor.BN(commitPhaseEnd),
          new anchor.BN(revealPhaseStart),
          new anchor.BN(revealPhaseEnd),
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
        "Set voting phases for reveal transaction signature:",
        setPhasesTx,
      );

      // Reveal the vote with the same choice and nonce used in the commit
      const tx = await ctx.program.methods
        .revealVote(
          ctx.VOTE_CHOICE_YES, // The Yes vote choice
          ctx.VOTE_NONCE, // The nonce used in the commit
        )
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: ctx.submissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: ctx.submissionPda,
          voteCommit: ctx.voteCommitPda,
          userProfile: ctx.validatorProfilePda,
          validator: ctx.validatorKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.validatorKeypair])
        .rpc();

      console.log("Vote reveal transaction signature:", tx);

      // Verify the vote commit was updated correctly
      const voteCommitAcc = await ctx.program.account.voteCommit.fetch(
        ctx.voteCommitPda,
      );
      expect(voteCommitAcc.revealed).to.be.true;
      expect(voteCommitAcc.voteChoice).to.not.be.null;
      expect(voteCommitAcc.voteChoice.yes).to.not.be.undefined;

      // Verify the submission-topic link vote counts were updated
      const linkAcc = await ctx.program.account.submissionTopicLink.fetch(
        ctx.submissionTopicLinkPda,
      );
      expect(linkAcc.totalRevealedVotes.toNumber()).to.equal(1);

      // The vote amount was 25, and the quadratic voting power is sqrt(25) = 5
      const expectedVotingPower = 5;
      expect(linkAcc.yesVotingPower.toNumber()).to.equal(expectedVotingPower);
      expect(linkAcc.noVotingPower.toNumber()).to.equal(0);
    });
  });
}
