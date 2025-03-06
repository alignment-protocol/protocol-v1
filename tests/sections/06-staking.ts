import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { web3 } from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { TestContext } from "../utils/test-setup";

export function runStakingTests(ctx: TestContext): void {
  describe("Staking", () => {
    it("Stakes tempAlign tokens for tempRep tokens", async () => {
      // The contributor's tempRep account was already created in previous test

      // Check the state to make sure we're working with the right submission count
      const preStakeState = await ctx.program.account.state.fetch(ctx.statePda);
      console.log(
        "State submission count before staking:",
        preStakeState.submissionCount.toNumber()
      );

      // Define the staking amount - stake half of the earned tempAlign tokens
      const stakeAmount = 50;

      // Stake topic-specific tokens for the contributor
      const tx = await ctx.program.methods
        .stakeTopicSpecificTokens(new anchor.BN(stakeAmount))
        .accounts({
          state: ctx.statePda,
          topic: ctx.topic1Pda,
          userProfile: ctx.contributorProfilePda,
          tempAlignMint: ctx.tempAlignMintPda,
          tempRepMint: ctx.tempRepMintPda,
          userTempAlignAccount: ctx.contributorTempAlignAccount,
          userTempRepAccount: ctx.contributorTempRepAccount,
          user: ctx.contributorKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.contributorKeypair])
        .rpc();

      console.log("Stake tokens transaction signature:", tx);

      // Verify that tempAlign tokens were burned
      const tempAlignAccount = await getAccount(
        ctx.provider.connection,
        ctx.contributorTempAlignAccount
      );
      expect(Number(tempAlignAccount.amount)).to.equal(100 - stakeAmount); // 50 burned

      // Verify that tempRep tokens were minted
      const tempRepAccount = await getAccount(
        ctx.provider.connection,
        ctx.contributorTempRepAccount
      );
      expect(Number(tempRepAccount.amount)).to.equal(stakeAmount); // 50 minted

      // Verify that the user profile's topic-specific token balances were updated
      const contributorProfile = await ctx.program.account.userProfile.fetch(
        ctx.contributorProfilePda
      );
      const topicTokenEntry = contributorProfile.topicTokens.find(
        (pair) => pair.topicId.toNumber() === 0 // Topic ID 0
      );
      expect(topicTokenEntry).to.not.be.undefined;

      // Now that we've already checked that topicTokenEntry exists
      expect(topicTokenEntry.topicId.toNumber()).to.equal(0);
      expect(topicTokenEntry.token.tempAlignAmount.toNumber()).to.equal(
        100 - stakeAmount
      ); // 50 remaining
      expect(topicTokenEntry.token.tempRepAmount.toNumber()).to.equal(
        stakeAmount
      ); // 50 earned

      // Now, have the validator also submit data to get tempAlign tokens
      // The validator's tempAlign account was already created in previous test

      // The program uses the state's submission_count as the seed for each new submission
      // Let's fetch the state account again to get the fresh submission count
      const updatedStateAcc = await ctx.program.account.state.fetch(
        ctx.statePda
      );
      const currentSubCount = updatedStateAcc.submissionCount.toNumber();

      console.log(
        "Current submission count for validator submission:",
        currentSubCount
      );

      // Derive a new submission PDA for validator submission using the current count
      const submissionCountBuffer = new anchor.BN(currentSubCount).toBuffer(
        "le",
        8
      );
      const [validatorSubmissionPda] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("submission"), submissionCountBuffer],
        ctx.program.programId
      );

      console.log(
        "Validator submission PDA:",
        validatorSubmissionPda.toBase58()
      );
      console.log(
        "Submission count buffer:",
        Array.from(submissionCountBuffer)
      );

      // Derive a new submission-topic link PDA for validator submission
      const [validatorSubmissionTopicLinkPda] =
        web3.PublicKey.findProgramAddressSync(
          [
            Buffer.from("submission_topic_link"),
            validatorSubmissionPda.toBuffer(),
            ctx.topic1Pda.toBuffer(),
          ],
          ctx.program.programId
        );

      // Have validator submit data to earn tokens
      const validatorSubmissionTx = await ctx.program.methods
        .submitDataToTopic("validator-test-submission")
        .accounts({
          state: ctx.statePda,
          topic: ctx.topic1Pda,
          tempAlignMint: ctx.tempAlignMintPda,
          contributorTempAlignAccount: ctx.validatorTempAlignAccount,
          submission: validatorSubmissionPda,
          submissionTopicLink: validatorSubmissionTopicLinkPda,
          contributorProfile: ctx.validatorProfilePda,
          contributor: ctx.validatorKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.validatorKeypair])
        .rpc();

      console.log(
        "Validator submission transaction signature:",
        validatorSubmissionTx
      );

      // Check the state again after validator's submission
      const stateAfterValidatorSubmission =
        await ctx.program.account.state.fetch(ctx.statePda);
      console.log(
        "Submission count AFTER validator submission:",
        stateAfterValidatorSubmission.submissionCount.toNumber()
      );

      // Verify validator received tempAlign tokens
      const validatorTempAlignData = await getAccount(
        ctx.provider.connection,
        ctx.validatorTempAlignAccount
      );
      expect(Number(validatorTempAlignData.amount)).to.equal(100); // tokens_to_mint value

      // Now stake validator's tempAlign for tempRep so they can vote
      // Define stake amount for validator
      const validatorStakeAmount = 50;

      // Validator's tempRep account was already created in the setup

      // Stake validator's tokens
      const validatorStakeTx = await ctx.program.methods
        .stakeTopicSpecificTokens(new anchor.BN(validatorStakeAmount))
        .accounts({
          state: ctx.statePda,
          topic: ctx.topic1Pda,
          userProfile: ctx.validatorProfilePda,
          tempAlignMint: ctx.tempAlignMintPda,
          tempRepMint: ctx.tempRepMintPda,
          userTempAlignAccount: ctx.validatorTempAlignAccount,
          userTempRepAccount: ctx.validatorTempRepAccount,
          user: ctx.validatorKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.validatorKeypair])
        .rpc();

      console.log("Validator stake transaction signature:", validatorStakeTx);

      // Verify validator's tempAlign tokens were burned and tempRep tokens were minted
      const updatedValidatorTempAlignData = await getAccount(
        ctx.provider.connection,
        ctx.validatorTempAlignAccount
      );
      expect(Number(updatedValidatorTempAlignData.amount)).to.equal(
        100 - validatorStakeAmount
      );

      const validatorTempRepData = await getAccount(
        ctx.provider.connection,
        ctx.validatorTempRepAccount
      );
      expect(Number(validatorTempRepData.amount)).to.equal(
        validatorStakeAmount
      );

      // Verify validator's user profile was updated with the topic tokens
      const validatorProfile = await ctx.program.account.userProfile.fetch(
        ctx.validatorProfilePda
      );
      const validatorTopicTokenEntry = validatorProfile.topicTokens.find(
        (pair) => pair.topicId.toNumber() === 0 // Topic ID 0
      );
      expect(validatorTopicTokenEntry).to.not.be.undefined;
      expect(validatorTopicTokenEntry.topicId.toNumber()).to.equal(0);
      expect(
        validatorTopicTokenEntry.token.tempAlignAmount.toNumber()
      ).to.equal(100 - validatorStakeAmount);
      expect(validatorTopicTokenEntry.token.tempRepAmount.toNumber()).to.equal(
        validatorStakeAmount
      );
    });
  });
}
