import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { web3 } from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { TestContext } from "../utils/test-setup";

export function runFinalizationTests(ctx: TestContext): void {
  describe("Finalization", () => {
    it("Finalizes the submission", async () => {
      // In a real scenario, we would need to wait for the reveal phase to end
      // For testing, we'll forcibly set the timestamps in the program to simulate past reveal phase

      // Need to adjust voting phases to make sure we're past the reveal phase
      const now = Math.floor(Date.now() / 1000); // Current time in seconds
      const commitPhaseStart = now - 2400; // 40 minutes ago
      const commitPhaseEnd = now - 1800; // 30 minutes ago
      const revealPhaseStart = commitPhaseEnd;
      const revealPhaseEnd = now - 600; // 10 minutes ago (reveal phase is over)

      // Set the voting phases to simulate being past the reveal phase
      const setPhasesTx = await ctx.program.methods
        .setVotingPhases(
          new anchor.BN(commitPhaseStart),
          new anchor.BN(commitPhaseEnd),
          new anchor.BN(revealPhaseStart),
          new anchor.BN(revealPhaseEnd)
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
        setPhasesTx
      );
      console.log(
        "Note: In a production environment, we would need to wait for the reveal phase to end"
      );

      // Finalize the submission
      const tx = await ctx.program.methods
        .finalizeSubmission()
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: ctx.submissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: ctx.submissionPda,
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

      console.log("Finalize submission transaction signature:", tx);

      // Verify the submission-topic link status changed from Pending to either Accepted or Rejected
      const linkAcc = await ctx.program.account.submissionTopicLink.fetch(
        ctx.submissionTopicLinkPda
      );
      expect(linkAcc.status.pending).to.be.undefined; // Should no longer be pending
      // It could be either accepted or rejected depending on voting
      expect(
        linkAcc.status.accepted !== undefined ||
          linkAcc.status.rejected !== undefined
      ).to.be.true;

      // Verify that tempAlign tokens were converted to Align tokens
      const tempAlignData = await getAccount(
        ctx.provider.connection,
        ctx.contributorTempAlignAccount
      );
      const alignData = await getAccount(
        ctx.provider.connection,
        ctx.contributorAlignAta
      );
      const tempRepData = await getAccount(
        ctx.provider.connection,
        ctx.contributorTempRepAccount
      );
      const repData = await getAccount(
        ctx.provider.connection,
        ctx.contributorRepAta
      );

      console.log(
        "Contributor tempAlign amount:",
        Number(tempAlignData.amount)
      );
      console.log("Contributor align amount:", Number(alignData.amount));
      console.log("Contributor tempRep amount:", Number(tempRepData.amount));
      console.log("Contributor Rep amount:", Number(repData.amount));

      // The tokens_to_mint is 100, we've already burned 50 for staking, so there's 50 left
      // All 50 remaining should have been burned and converted to Align
      expect(Number(tempAlignData.amount)).to.equal(0);
      expect(Number(alignData.amount)).to.equal(50);

      // Verify the contributor's topic-specific token balances were updated
      const contributorProfile = await ctx.program.account.userProfile.fetch(
        ctx.contributorProfilePda
      );
      const topicTokenEntry = contributorProfile.topicTokens.find(
        (pair) => pair.topicId.toNumber() === 0 // Topic ID 0
      );
      expect(topicTokenEntry).to.not.be.undefined;

      expect(topicTokenEntry.topicId.toNumber()).to.equal(0);
      expect(topicTokenEntry.token.tempAlignAmount.toNumber()).to.equal(0); // All converted
      expect(topicTokenEntry.token.tempRepAmount.toNumber()).to.equal(50); // 50 earned from staking
    });

    it("Finalizes the vote", async () => {
      // Create ATA for validator's permanent Rep tokens if it doesn't already exist
      const validatorRep = await getAccount(
        ctx.provider.connection,
        ctx.validatorRepAta
      ).catch(() => null);

      if (!validatorRep) {
        const tx = await ctx.program.methods
          .createUserAta()
          .accounts({
            state: ctx.statePda,
            payer: ctx.authorityKeypair.publicKey,
            user: ctx.validatorKeypair.publicKey,
            mint: ctx.repMintPda,
            userAta: ctx.validatorRepAta,
            systemProgram: web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: anchor.web3.ASSOCIATED_TOKEN_PROGRAM_ID,
            rent: web3.SYSVAR_RENT_PUBKEY,
          })
          .signers([ctx.authorityKeypair, ctx.validatorKeypair])
          .rpc();

        console.log("Create validatorRepAta transaction signature:", tx);
      }

      // By now, the submission should be finalized (Accepted or Rejected status)
      // Verify the status of the submission-topic link
      const linkAcc = await ctx.program.account.submissionTopicLink.fetch(
        ctx.submissionTopicLinkPda
      );
      console.log("Submission-topic link status:", linkAcc.status);

      // Finalize the vote
      const tx = await ctx.program.methods
        .finalizeVote()
        .accounts({
          state: ctx.statePda,
          submissionTopicLink: ctx.submissionTopicLinkPda,
          topic: ctx.topic1Pda,
          submission: ctx.submissionPda,
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

      console.log("Finalize vote transaction signature:", tx);

      // Verify the vote was finalized
      const voteCommitAcc = await ctx.program.account.voteCommit.fetch(
        ctx.voteCommitPda
      );
      expect(voteCommitAcc.finalized).to.be.true;

      // Verify the validator's tempRep tokens were converted to permanent Rep
      const tempAlignData = await getAccount(
        ctx.provider.connection,
        ctx.validatorTempAlignAccount
      );
      const alignData = await getAccount(
        ctx.provider.connection,
        ctx.validatorAlignAta
      );
      const tempRepData = await getAccount(
        ctx.provider.connection,
        ctx.validatorTempRepAccount
      );
      const repData = await getAccount(
        ctx.provider.connection,
        ctx.validatorRepAta
      );

      // Since we voted yes and the submission was accepted (vote with consensus),
      // 25 tempRep tokens should be converted to 25 permanent Rep tokens
      // With our token locking implementation, the tokens used for voting
      // are moved to lockedTempRepAmount and then fully converted
      console.log("Validator tempAlign amount:", Number(tempAlignData.amount));
      console.log("Validator align amount:", Number(alignData.amount));
      console.log("Validator tempRep amount:", Number(tempRepData.amount));
      console.log("Validator permanent Rep amount:", Number(repData.amount));

      // Verify the permanent Rep tokens were minted - exactly 25 tokens from the locked tokens
      expect(Number(repData.amount)).to.equal(25);

      // With our token locking implementation, the 25 locked tokens are burned
      // during finalization, but the initial available amount remains at 25
      expect(Number(tempRepData.amount)).to.equal(25);

      // Verify that the validator's profile was updated
      const validatorProfile = await ctx.program.account.userProfile.fetch(
        ctx.validatorProfilePda
      );
      expect(validatorProfile.permanentRepAmount.toNumber()).to.equal(25);

      // Verify the validator's topic-specific token balances were updated
      const topicTokenEntry = validatorProfile.topicTokens.find(
        (pair) => pair.topicId.toNumber() === 0 // Topic ID 0
      );

      if (topicTokenEntry) {
        expect(topicTokenEntry.topicId.toNumber()).to.equal(0);
        // After our fix, temporary reputation amount should remain at 25
        // (since we moved tokens to lockedTempRepAmount during commit_vote, not deducted directly)
        expect(topicTokenEntry.token.tempRepAmount.toNumber()).to.equal(25);

        // With token locking implementation, ensure locked tokens are released after finalization
        expect(
          topicTokenEntry.token.lockedTempRepAmount?.toNumber() || 0
        ).to.equal(0);
      }
    });
  });
}
