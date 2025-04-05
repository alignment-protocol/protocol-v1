import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { web3, BN } from "@coral-xyz/anchor";
import { TestContext } from "../utils/test-setup";
import * as crypto from "crypto";

export function runVotingTests(ctx: TestContext): void {
  describe("Voting", () => {
    it("Commits a vote on the submission", async () => {
      // Fetch the link account to check its state before setting phases
      let linkAccBefore = await ctx.program.account.submissionTopicLink.fetch(
        ctx.submissionTopicLinkPda,
      );
      console.log(
        "Link account status before setting phases:",
        linkAccBefore.status,
      );
      console.log(
        " -> Commit phase:",
        linkAccBefore.commitPhaseStart.toNumber(),
        "to",
        linkAccBefore.commitPhaseEnd.toNumber(),
      );
      console.log(
        " -> Reveal phase:",
        linkAccBefore.revealPhaseStart.toNumber(),
        "to",
        linkAccBefore.revealPhaseEnd.toNumber(),
      );

      // Adjust voting phases for testing - ensure we are within the commit window
      const now = Math.floor(Date.now() / 1000);
      const commitStart = now - 60; // Start 1 minute ago to be safe
      const commitEnd = now + 600; // 10 minutes from now
      const revealStart = commitEnd;
      const revealEnd = revealStart + 600; // 10 minutes after commit phase

      console.log(
        `Setting phases: Commit ${commitStart}-${commitEnd}, Reveal ${revealStart}-${revealEnd}`,
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
          submission: ctx.submissionPda, // Submission associated with the link
          authority: ctx.authorityKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.authorityKeypair])
        .rpc();
      console.log("Set voting phases transaction signature:", setPhasesTx);

      // Verify phases were set
      linkAccBefore = await ctx.program.account.submissionTopicLink.fetch(
        ctx.submissionTopicLinkPda,
      );
      expect(linkAccBefore.commitPhaseStart.toNumber()).to.equal(commitStart);
      expect(linkAccBefore.commitPhaseEnd.toNumber()).to.equal(commitEnd);
      expect(linkAccBefore.revealPhaseStart.toNumber()).to.equal(revealStart);
      expect(linkAccBefore.revealPhaseEnd.toNumber()).to.equal(revealEnd);

      // Calculate vote hash
      const voteChoice = ctx.VOTE_CHOICE_YES; // Using { yes: {} }
      const voteNonce = ctx.VOTE_NONCE; // Using "test-nonce"

      // Ensure voteChoice is represented correctly for hashing (e.g., 0 for Yes, 1 for No)
      // This depends on how VOTE_CHOICE_YES is defined in test-setup. For now, assume 0.
      const voteChoiceByte = Buffer.from([0]); // Assuming 0 represents Yes

      const message = Buffer.concat([
        ctx.validatorKeypair.publicKey.toBuffer(),
        ctx.submissionTopicLinkPda.toBuffer(),
        voteChoiceByte,
        Buffer.from(voteNonce),
      ]);
      const voteHash = Array.from(
        crypto.createHash("sha256").update(message).digest(),
      );
      ctx.voteHash = voteHash; // Store for reveal test
      console.log(
        "Calculated Vote Hash:",
        Buffer.from(voteHash).toString("hex"),
      );

      // Derive the vote commit PDA
      [ctx.voteCommitPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote_commit"),
          ctx.submissionTopicLinkPda.toBuffer(),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId,
      );
      console.log("Derived VoteCommit PDA:", ctx.voteCommitPda.toBase58());

      // Define vote amount and type
      const validatorBalance = await ctx.program.account.userTopicBalance.fetch(
        ctx.validatorTopic1BalancePda,
      );
      const availableTempRep = validatorBalance.tempRepAmount.toNumber();
      const voteAmount = new BN(availableTempRep / 2); // Vote with half the available tempRep (25)
      const isPermanentRep = false; // Using temporary reputation
      console.log(
        `Validator ${ctx.validatorKeypair.publicKey.toBase58()} committing vote:`,
      );
      console.log(` -> Amount: ${voteAmount.toNumber()} (Temp Rep)`);
      console.log(` -> Hash: ${Buffer.from(voteHash).toString("hex")}`);
      console.log(` -> On Link: ${ctx.submissionTopicLinkPda.toBase58()}`);

      // Commit the vote - *** ADDED userTopicBalance and validatorRepAta ***
      const tx = await ctx.program.methods
        .commitVote(voteHash, voteAmount, isPermanentRep)
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: ctx.submissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: ctx.submissionPda,
          voteCommit: ctx.voteCommitPda,
          userProfile: ctx.validatorProfilePda,
          userTopicBalance: ctx.validatorTopic1BalancePda, // ADDED
          validatorRepAta: ctx.validatorRepAta, // ADDED
          validator: ctx.validatorKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
          // rent: web3.SYSVAR_RENT_PUBKEY, // Implicit
        })
        .signers([ctx.validatorKeypair])
        .rpc();

      console.log("Vote commit transaction signature:", tx);

      // --- Verification ---
      // Verify the vote commit account was created
      const voteCommitAcc = await ctx.program.account.voteCommit.fetch(
        ctx.voteCommitPda,
      );
      console.log("Fetched VoteCommit account:", voteCommitAcc);
      expect(voteCommitAcc.submissionTopicLink.toString()).to.equal(
        ctx.submissionTopicLinkPda.toString(),
      );
      expect(voteCommitAcc.validator.toString()).to.equal(
        ctx.validatorKeypair.publicKey.toString(),
      );
      expect(Array.from(voteCommitAcc.voteHash)).to.deep.equal(voteHash); // Compare arrays
      expect(voteCommitAcc.revealed).to.be.false;
      expect(voteCommitAcc.finalized).to.be.false;
      expect(voteCommitAcc.voteChoice).to.be.null;
      expect(voteCommitAcc.voteAmount.toNumber()).to.equal(
        voteAmount.toNumber(),
      );
      expect(voteCommitAcc.isPermanentRep).to.equal(isPermanentRep);
      expect(voteCommitAcc.commitTimestamp.toNumber()).to.be.closeTo(now, 60); // Check commit time

      // Verify the submission-topic link vote count was updated
      const linkAccAfter = await ctx.program.account.submissionTopicLink.fetch(
        ctx.submissionTopicLinkPda,
      );
      console.log("Link account after commit:", linkAccAfter);
      expect(linkAccAfter.totalCommittedVotes.toNumber()).to.equal(
        linkAccBefore.totalCommittedVotes.toNumber() + 1,
      );
      expect(linkAccAfter.totalRevealedVotes.toNumber()).to.equal(
        linkAccBefore.totalRevealedVotes.toNumber(),
      ); // Should not change yet

      // Verify the validator's UserTopicBalance locked amount was updated
      const balanceAfter = await ctx.program.account.userTopicBalance.fetch(
        ctx.validatorTopic1BalancePda,
      );
      console.log("Validator UserTopicBalance after commit:", balanceAfter);
      // Lock amount should increase if using temp rep
      const expectedLockedAmount = isPermanentRep
        ? validatorBalance.lockedTempRepAmount.toNumber()
        : validatorBalance.lockedTempRepAmount.toNumber() +
          voteAmount.toNumber();
      expect(balanceAfter.lockedTempRepAmount.toNumber()).to.equal(
        expectedLockedAmount,
      );
      // Available tempRep should decrease if using temp rep
      const expectedTempRepAmount = isPermanentRep
        ? validatorBalance.tempRepAmount.toNumber()
        : validatorBalance.tempRepAmount.toNumber() - voteAmount.toNumber();
      expect(balanceAfter.tempRepAmount.toNumber()).to.equal(
        expectedTempRepAmount,
      );
    });

    it("Reveals the committed vote", async () => {
      // Fetch state before setting phases
      let linkAccBeforeReveal =
        await ctx.program.account.submissionTopicLink.fetch(
          ctx.submissionTopicLinkPda,
        );
      let voteCommitBeforeReveal = await ctx.program.account.voteCommit.fetch(
        ctx.voteCommitPda,
      );
      console.log(
        "Link account status before setting phases for reveal:",
        linkAccBeforeReveal.status,
      );
      console.log(
        "VoteCommit status before setting phases for reveal:",
        voteCommitBeforeReveal.revealed,
      );

      // Adjust voting phases - ensure we are within the reveal window
      const now = Math.floor(Date.now() / 1000);
      const commitStart = now - 1200; // 20 minutes ago
      const commitEnd = now - 600; // 10 minutes ago
      const revealStart = commitEnd; // Starts immediately after commit ends
      const revealEnd = now + 600; // 10 minutes from now

      console.log(
        `Setting phases for reveal: Commit ${commitStart}-${commitEnd}, Reveal ${revealStart}-${revealEnd}`,
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
        "Set voting phases for reveal transaction signature:",
        setPhasesTx,
      );

      // Verify phases were set correctly
      linkAccBeforeReveal = await ctx.program.account.submissionTopicLink.fetch(
        ctx.submissionTopicLinkPda,
      );
      expect(linkAccBeforeReveal.revealPhaseStart.toNumber()).to.equal(
        revealStart,
      );
      expect(linkAccBeforeReveal.revealPhaseEnd.toNumber()).to.equal(revealEnd);

      // Use the same choice and nonce from the commit test
      const voteChoice = ctx.VOTE_CHOICE_YES; // { yes: {} }
      const voteNonce = ctx.VOTE_NONCE; // "test-nonce"
      console.log(
        `Validator ${ctx.validatorKeypair.publicKey.toBase58()} revealing vote:`,
      );
      console.log(` -> Choice: ${voteChoice}`);
      console.log(` -> Nonce: ${voteNonce}`);
      console.log(` -> For VoteCommit: ${ctx.voteCommitPda.toBase58()}`);

      // Reveal the vote
      const tx = await ctx.program.methods
        .revealVote(voteChoice, voteNonce)
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: ctx.submissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: ctx.submissionPda,
          voteCommit: ctx.voteCommitPda,
          userProfile: ctx.validatorProfilePda, // Needed for constraint check
          validator: ctx.validatorKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.validatorKeypair])
        .rpc();

      console.log("Vote reveal transaction signature:", tx);

      // --- Verification ---
      // Verify the vote commit account was updated
      const voteCommitAccAfter = await ctx.program.account.voteCommit.fetch(
        ctx.voteCommitPda,
      );
      console.log("VoteCommit account after reveal:", voteCommitAccAfter);
      expect(voteCommitAccAfter.revealed).to.be.true;
      expect(voteCommitAccAfter.voteChoice).to.not.be.null;
      expect(voteCommitAccAfter.voteChoice.yes).to.not.be.undefined; // Check specifically for 'yes'

      // Verify the submission-topic link vote counts were updated
      const linkAccAfter = await ctx.program.account.submissionTopicLink.fetch(
        ctx.submissionTopicLinkPda,
      );
      console.log("Link account after reveal:", linkAccAfter);
      expect(linkAccAfter.totalRevealedVotes.toNumber()).to.equal(
        linkAccBeforeReveal.totalRevealedVotes.toNumber() + 1,
      );

      // Calculate expected voting power (sqrt of vote amount)
      // Using the amount from the fetched voteCommit account before reveal
      const voteAmount = voteCommitBeforeReveal.voteAmount.toNumber();
      const expectedVotingPower = Math.floor(Math.sqrt(voteAmount)); // Use floor for integer sqrt
      console.log(
        ` -> Vote Amount: ${voteAmount}, Expected Voting Power (sqrt): ${expectedVotingPower}`,
      );

      expect(linkAccAfter.yesVotingPower.toNumber()).to.equal(
        linkAccBeforeReveal.yesVotingPower.toNumber() + expectedVotingPower,
      );
      expect(linkAccAfter.noVotingPower.toNumber()).to.equal(
        linkAccBeforeReveal.noVotingPower.toNumber(),
      ); // No vote hasn't changed
    });
  });
}
