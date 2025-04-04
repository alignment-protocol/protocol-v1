import { expect } from "chai";
import * as anchor from "@coral-xyz/anchor";
import { web3, BN } from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { TestContext } from "../utils/test-setup";

export function runSubmissionTests(ctx: TestContext): void {
  describe("Submission", () => {
    before(
      "Initialize UserTopicBalance for Contributor and Topic 1",
      async () => {
        [ctx.contributorTopic1BalancePda] =
          web3.PublicKey.findProgramAddressSync(
            [
              Buffer.from("user_topic_balance"),
              ctx.contributorKeypair.publicKey.toBuffer(),
              ctx.topic1Pda.toBuffer(),
            ],
            ctx.program.programId,
          );

        const tx = await ctx.program.methods
          .initializeUserTopicBalance()
          .accounts({
            user: ctx.contributorKeypair.publicKey,
            userProfile: ctx.contributorProfilePda,
            topic: ctx.topic1Pda,
            userTopicBalance: ctx.contributorTopic1BalancePda,
            systemProgram: web3.SystemProgram.programId,
            rent: web3.SYSVAR_RENT_PUBKEY,
          })
          .signers([ctx.contributorKeypair])
          .rpc();
        console.log("Initialize Contributor Topic 1 Balance TX:", tx);

        const balanceAcc = await ctx.program.account.userTopicBalance.fetch(
          ctx.contributorTopic1BalancePda,
        );
        expect(balanceAcc.user.toString()).to.equal(
          ctx.contributorKeypair.publicKey.toString(),
        );
        expect(balanceAcc.topic.toString()).to.equal(ctx.topic1Pda.toString());
        expect(balanceAcc.tempAlignAmount.toNumber()).to.equal(0);
        expect(balanceAcc.tempRepAmount.toNumber()).to.equal(0);
        expect(balanceAcc.lockedTempRepAmount.toNumber()).to.equal(0);
      },
    );

    it("Submits data to the first topic", async () => {
      let contributorProfileBefore =
        await ctx.program.account.userProfile.fetch(ctx.contributorProfilePda);
      const currentSubmissionIndex =
        contributorProfileBefore.userSubmissionCount;
      console.log(
        "Contributor's current submission index BEFORE submission:",
        currentSubmissionIndex.toNumber(),
      );

      const topicAccBeforeSubmit = await ctx.program.account.topic.fetch(
        ctx.topic1Pda,
      );
      console.log(
        "Topic submission count BEFORE submission:",
        topicAccBeforeSubmit.submissionCount.toNumber(),
      );

      [ctx.submissionPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("submission"),
          ctx.contributorKeypair.publicKey.toBuffer(),
          currentSubmissionIndex.toBuffer("le", 8),
        ],
        ctx.program.programId,
      );

      [ctx.submissionTopicLinkPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("submission_topic_link"),
          ctx.submissionPda.toBuffer(),
          ctx.topic1Pda.toBuffer(),
        ],
        ctx.program.programId,
      );

      const tx = await ctx.program.methods
        .submitDataToTopic(ctx.SUBMISSION_DATA, currentSubmissionIndex)
        .accounts({
          state: ctx.statePda,
          topic: ctx.topic1Pda,
          tempAlignMint: ctx.tempAlignMintPda,
          contributorTempAlignAccount: ctx.contributorTempAlignAccount,
          submission: ctx.submissionPda,
          submissionTopicLink: ctx.submissionTopicLinkPda,
          contributorProfile: ctx.contributorProfilePda,
          userTopicBalance: ctx.contributorTopic1BalancePda,
          contributor: ctx.contributorKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.contributorKeypair])
        .rpc();

      console.log("Submit data transaction signature:", tx);

      const submissionAcc = await ctx.program.account.submission.fetch(
        ctx.submissionPda,
      );
      expect(submissionAcc.contributor.toString()).to.equal(
        ctx.contributorKeypair.publicKey.toString(),
      );
      expect(submissionAcc.dataReference).to.equal(ctx.SUBMISSION_DATA);
      const now = Math.floor(Date.now() / 1000);
      expect(submissionAcc.timestamp.toNumber()).to.be.closeTo(now, 60);

      const linkAcc = await ctx.program.account.submissionTopicLink.fetch(
        ctx.submissionTopicLinkPda,
      );
      expect(linkAcc.submission.toString()).to.equal(
        ctx.submissionPda.toString(),
      );
      expect(linkAcc.topic.toString()).to.equal(ctx.topic1Pda.toString());
      expect(linkAcc.status.pending).to.not.be.undefined;
      expect(linkAcc.yesVotingPower.toNumber()).to.equal(0);
      expect(linkAcc.noVotingPower.toNumber()).to.equal(0);
      expect(linkAcc.totalCommittedVotes.toNumber()).to.equal(0);
      expect(linkAcc.totalRevealedVotes.toNumber()).to.equal(0);

      const expectedCommitEnd =
        submissionAcc.timestamp.toNumber() +
        topicAccBeforeSubmit.commitPhaseDuration.toNumber();
      const expectedRevealStart = expectedCommitEnd;
      const expectedRevealEnd =
        expectedRevealStart +
        topicAccBeforeSubmit.revealPhaseDuration.toNumber();
      expect(linkAcc.commitPhaseStart.toNumber()).to.equal(
        submissionAcc.timestamp.toNumber(),
      );
      expect(linkAcc.commitPhaseEnd.toNumber()).to.equal(expectedCommitEnd);
      expect(linkAcc.revealPhaseStart.toNumber()).to.equal(expectedRevealStart);
      expect(linkAcc.revealPhaseEnd.toNumber()).to.equal(expectedRevealEnd);

      const contributorTempAlignData = await getAccount(
        ctx.provider.connection,
        ctx.contributorTempAlignAccount,
      );
      const stateAcc = await ctx.program.account.state.fetch(ctx.statePda);
      expect(Number(contributorTempAlignData.amount)).to.equal(
        stateAcc.tokensToMint.toNumber(),
      );

      const topicAccAfterSubmit = await ctx.program.account.topic.fetch(
        ctx.topic1Pda,
      );
      console.log(
        "Topic submission count AFTER submission:",
        topicAccAfterSubmit.submissionCount.toNumber(),
      );
      expect(topicAccAfterSubmit.submissionCount.toNumber()).to.equal(
        topicAccBeforeSubmit.submissionCount.toNumber() + 1,
      );

      const contributorProfileAfter =
        await ctx.program.account.userProfile.fetch(ctx.contributorProfilePda);
      expect(contributorProfileAfter.userSubmissionCount.toNumber()).to.equal(
        currentSubmissionIndex.toNumber() + 1,
      );

      const balanceAccAfter = await ctx.program.account.userTopicBalance.fetch(
        ctx.contributorTopic1BalancePda,
      );
      expect(balanceAccAfter.tempAlignAmount.toNumber()).to.equal(
        stateAcc.tokensToMint.toNumber(),
      );
      expect(balanceAccAfter.tempRepAmount.toNumber()).to.equal(0);
      expect(balanceAccAfter.lockedTempRepAmount.toNumber()).to.equal(0);
    });
  });
}
