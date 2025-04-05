import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { web3, BN } from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { TestContext } from "../utils/test-setup";

export function runStakingTests(ctx: TestContext): void {
  describe("Staking", () => {
    // We need UserTopicBalance for the validator too
    before(
      "Initialize UserTopicBalance for Validator and Topic 1",
      async () => {
        [ctx.validatorTopic1BalancePda] = web3.PublicKey.findProgramAddressSync(
          [
            Buffer.from("user_topic_balance"),
            ctx.validatorKeypair.publicKey.toBuffer(),
            ctx.topic1Pda.toBuffer(),
          ],
          ctx.program.programId,
        );

        const tx = await ctx.program.methods
          .initializeUserTopicBalance()
          .accounts({
            user: ctx.validatorKeypair.publicKey,
            userProfile: ctx.validatorProfilePda,
            topic: ctx.topic1Pda,
            userTopicBalance: ctx.validatorTopic1BalancePda,
            systemProgram: web3.SystemProgram.programId,
            // rent: web3.SYSVAR_RENT_PUBKEY, // Implicit
          })
          .signers([ctx.validatorKeypair])
          .rpc();
        console.log("Initialize Validator Topic 1 Balance TX:", tx);
        // Verify initialization (optional, but good practice)
        const balanceAcc = await ctx.program.account.userTopicBalance.fetch(
          ctx.validatorTopic1BalancePda,
        );
        expect(balanceAcc.user.toString()).to.equal(
          ctx.validatorKeypair.publicKey.toString(),
        );
        expect(balanceAcc.topic.toString()).to.equal(ctx.topic1Pda.toString());
        expect(balanceAcc.tempAlignAmount.toNumber()).to.equal(0);
        expect(balanceAcc.tempRepAmount.toNumber()).to.equal(0);
        expect(balanceAcc.lockedTempRepAmount.toNumber()).to.equal(0);
      },
    );

    it("Stakes tempAlign tokens for tempRep tokens for contributor", async () => {
      // Fetch initial balances from the UserTopicBalance account
      const balanceBefore = await ctx.program.account.userTopicBalance.fetch(
        ctx.contributorTopic1BalancePda,
      );
      const tempAlignBefore = balanceBefore.tempAlignAmount.toNumber();
      const tempRepBefore = balanceBefore.tempRepAmount.toNumber();

      // Fetch the global tempAlign token account balance before stake
      const globalTempAlignBefore = await getAccount(
        ctx.provider.connection,
        ctx.contributorTempAlignAccount,
      );
      const initialGlobalAmount = Number(globalTempAlignBefore.amount);

      // Fetch tokensToMint from state
      const tokensToMint = (
        await ctx.program.account.state.fetch(ctx.statePda)
      ).tokensToMint.toNumber();
      // Verify the initial balance in UserTopicBalance matches tokensToMint from the previous submission step
      expect(tempAlignBefore).to.equal(tokensToMint);

      // Define the staking amount
      const stakeAmount = new BN(tokensToMint / 2); // Stake 50

      console.log(
        `Contributor staking ${stakeAmount.toNumber()} tempAlign for topic ${ctx.topic1Pda.toBase58()}`,
      );
      console.log(
        ` -> UserTopicBalance before: Align=${tempAlignBefore}, Rep=${tempRepBefore}`,
      );
      console.log(` -> Global TempAlign ATA before: ${initialGlobalAmount}`);

      // Stake topic-specific tokens for the contributor
      const tx = await ctx.program.methods
        .stakeTopicSpecificTokens(stakeAmount)
        .accounts({
          state: ctx.statePda,
          topic: ctx.topic1Pda,
          userProfile: ctx.contributorProfilePda,
          userTopicBalance: ctx.contributorTopic1BalancePda,
          tempAlignMint: ctx.tempAlignMintPda,
          tempRepMint: ctx.tempRepMintPda,
          userTempAlignAccount: ctx.contributorTempAlignAccount,
          userTempRepAccount: ctx.contributorTempRepAccount,
          user: ctx.contributorKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([ctx.contributorKeypair])
        .rpc();

      console.log("Contributor stake tokens transaction signature:", tx);

      // Verify global tempAlign tokens were burned
      const globalTempAlignAfter = await getAccount(
        ctx.provider.connection,
        ctx.contributorTempAlignAccount,
      );
      const finalGlobalAmount = Number(globalTempAlignAfter.amount);
      console.log(` -> Global TempAlign ATA after: ${finalGlobalAmount}`);
      expect(finalGlobalAmount).to.equal(
        initialGlobalAmount - stakeAmount.toNumber(),
      );

      // Verify global tempRep tokens were minted
      const globalTempRepAfter = await getAccount(
        ctx.provider.connection,
        ctx.contributorTempRepAccount,
      );
      console.log(
        ` -> Global TempRep ATA after: ${Number(globalTempRepAfter.amount)}`,
      );
      expect(Number(globalTempRepAfter.amount)).to.equal(
        stakeAmount.toNumber(),
      ); // Assumes starting from 0

      // Verify the UserTopicBalance account was updated
      const balanceAfter = await ctx.program.account.userTopicBalance.fetch(
        ctx.contributorTopic1BalancePda,
      );
      console.log(
        ` -> UserTopicBalance after: Align=${balanceAfter.tempAlignAmount.toNumber()}, Rep=${balanceAfter.tempRepAmount.toNumber()}`,
      );

      // Check that tempAlignAmount decreased
      expect(balanceAfter.tempAlignAmount.toNumber()).to.equal(
        tempAlignBefore - stakeAmount.toNumber(),
      );
      // Check that tempRepAmount increased
      expect(balanceAfter.tempRepAmount.toNumber()).to.equal(
        tempRepBefore + stakeAmount.toNumber(),
      );
      expect(balanceAfter.lockedTempRepAmount.toNumber()).to.equal(0); // Should not change yet
    });

    it("Allows validator to submit and stake tokens", async () => {
      // --- Validator Submits Data ---
      const validatorProfileBefore =
        await ctx.program.account.userProfile.fetch(ctx.validatorProfilePda);
      const validatorSubmissionIndex =
        validatorProfileBefore.userSubmissionCount;
      console.log(
        `Validator submitting data with index ${validatorSubmissionIndex.toNumber()}`,
      );

      // Derive validator's submission PDA
      [ctx.validatorSubmissionPda] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("submission"),
          ctx.validatorKeypair.publicKey.toBuffer(),
          validatorSubmissionIndex.toBuffer("le", 8),
        ],
        ctx.program.programId,
      );
      console.log(
        "Validator submission PDA:",
        ctx.validatorSubmissionPda.toBase58(),
      );

      // Derive validator's submission-topic link PDA
      [ctx.validatorSubmissionTopicLinkPda] =
        web3.PublicKey.findProgramAddressSync(
          [
            Buffer.from("submission_topic_link"),
            ctx.validatorSubmissionPda.toBuffer(),
            ctx.topic1Pda.toBuffer(),
          ],
          ctx.program.programId,
        );
      console.log(
        "Validator link PDA:",
        ctx.validatorSubmissionTopicLinkPda.toBase58(),
      );

      // Have validator submit data to earn tokens
      const validatorSubmissionTx = await ctx.program.methods
        .submitDataToTopic(
          "validator-test-submission",
          validatorSubmissionIndex,
        ) // Pass index
        .accounts({
          state: ctx.statePda,
          topic: ctx.topic1Pda,
          tempAlignMint: ctx.tempAlignMintPda,
          contributorTempAlignAccount: ctx.validatorTempAlignAccount,
          submission: ctx.validatorSubmissionPda,
          submissionTopicLink: ctx.validatorSubmissionTopicLinkPda,
          contributorProfile: ctx.validatorProfilePda,
          userTopicBalance: ctx.validatorTopic1BalancePda,
          contributor: ctx.validatorKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
        })
        .signers([ctx.validatorKeypair])
        .rpc();

      console.log(
        "Validator submission transaction signature:",
        validatorSubmissionTx,
      );

      // Verify validator received global tempAlign tokens
      const validatorGlobalTempAlignData = await getAccount(
        ctx.provider.connection,
        ctx.validatorTempAlignAccount,
      );
      const tokensToMint = (
        await ctx.program.account.state.fetch(ctx.statePda)
      ).tokensToMint.toNumber();
      const validatorInitialGlobalAmount = Number(
        validatorGlobalTempAlignData.amount,
      );
      expect(validatorInitialGlobalAmount).to.equal(tokensToMint);

      // Check validator UserTopicBalance tempAlign was credited
      const validatorBalanceAfterSubmit =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.validatorTopic1BalancePda,
        );
      console.log(
        `Validator UserTopicBalance after submit: Align=${validatorBalanceAfterSubmit.tempAlignAmount.toNumber()}, Rep=${validatorBalanceAfterSubmit.tempRepAmount.toNumber()}`,
      );
      expect(validatorBalanceAfterSubmit.tempAlignAmount.toNumber()).to.equal(
        tokensToMint,
      ); // Check credit

      // --- Validator Stakes Tokens ---
      const validatorStakeAmount = new BN(tokensToMint / 2); // Stake 50
      const validatorBalanceBeforeStake =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.validatorTopic1BalancePda,
        );
      const validatorTempAlignBeforeStake =
        validatorBalanceBeforeStake.tempAlignAmount.toNumber();
      const validatorTempRepBeforeStake =
        validatorBalanceBeforeStake.tempRepAmount.toNumber();

      console.log(
        `Validator staking ${validatorStakeAmount.toNumber()} tempAlign for topic ${ctx.topic1Pda.toBase58()}`,
      );
      console.log(
        ` -> UserTopicBalance before: Align=${validatorTempAlignBeforeStake}, Rep=${validatorTempRepBeforeStake}`,
      );
      console.log(
        ` -> Global TempAlign ATA before: ${validatorInitialGlobalAmount}`,
      );

      // Stake validator's tokens
      const validatorStakeTx = await ctx.program.methods
        .stakeTopicSpecificTokens(validatorStakeAmount)
        .accounts({
          state: ctx.statePda,
          topic: ctx.topic1Pda,
          userProfile: ctx.validatorProfilePda,
          userTopicBalance: ctx.validatorTopic1BalancePda,
          tempAlignMint: ctx.tempAlignMintPda,
          tempRepMint: ctx.tempRepMintPda,
          userTempAlignAccount: ctx.validatorTempAlignAccount,
          userTempRepAccount: ctx.validatorTempRepAccount,
          user: ctx.validatorKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([ctx.validatorKeypair])
        .rpc();

      console.log("Validator stake transaction signature:", validatorStakeTx);

      // Verify validator's global tempAlign tokens were burned
      const validatorGlobalTempAlignAfterStake = await getAccount(
        ctx.provider.connection,
        ctx.validatorTempAlignAccount,
      );
      const validatorFinalGlobalAmount = Number(
        validatorGlobalTempAlignAfterStake.amount,
      );
      console.log(
        ` -> Global TempAlign ATA after: ${validatorFinalGlobalAmount}`,
      );
      expect(validatorFinalGlobalAmount).to.equal(
        validatorInitialGlobalAmount - validatorStakeAmount.toNumber(),
      );

      // Verify validator's global tempRep tokens were minted
      const validatorGlobalTempRepAfterStake = await getAccount(
        ctx.provider.connection,
        ctx.validatorTempRepAccount,
      );
      console.log(
        ` -> Global TempRep ATA after: ${Number(validatorGlobalTempRepAfterStake.amount)}`,
      );
      expect(Number(validatorGlobalTempRepAfterStake.amount)).to.equal(
        validatorStakeAmount.toNumber(),
      ); // Assumes starting from 0

      // Verify validator's UserTopicBalance account was updated
      const validatorBalanceAfterStake =
        await ctx.program.account.userTopicBalance.fetch(
          ctx.validatorTopic1BalancePda,
        );
      console.log(
        ` -> UserTopicBalance after: Align=${validatorBalanceAfterStake.tempAlignAmount.toNumber()}, Rep=${validatorBalanceAfterStake.tempRepAmount.toNumber()}`,
      );

      // Check tempAlignAmount decreased
      expect(validatorBalanceAfterStake.tempAlignAmount.toNumber()).to.equal(
        validatorTempAlignBeforeStake - validatorStakeAmount.toNumber(),
      );
      // Check tempRepAmount increased
      expect(validatorBalanceAfterStake.tempRepAmount.toNumber()).to.equal(
        validatorTempRepBeforeStake + validatorStakeAmount.toNumber(),
      );
      expect(
        validatorBalanceAfterStake.lockedTempRepAmount.toNumber(),
      ).to.equal(0);
    });
  });
}
