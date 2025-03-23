import { expect } from "chai";
import { web3 } from "@coral-xyz/anchor";
import {
  TOKEN_PROGRAM_ID,
  getAccount,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import { ASSOCIATED_TOKEN_PROGRAM_ID } from "../utils/constants";
import { TestContext } from "../utils/test-setup";

export function runUserSetupTests(ctx: TestContext): void {
  describe("User Setup", () => {
    it("Creates user profiles for contributor and validator", async () => {
      // Create a profile for the contributor
      let tx = await ctx.program.methods
        .createUserProfile()
        .accounts({
          state: ctx.statePda,
          userProfile: ctx.contributorProfilePda,
          user: ctx.contributorKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.contributorKeypair])
        .rpc();

      console.log("Create contributor profile transaction signature:", tx);

      // Create a profile for the validator
      tx = await ctx.program.methods
        .createUserProfile()
        .accounts({
          state: ctx.statePda,
          userProfile: ctx.validatorProfilePda,
          user: ctx.validatorKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.validatorKeypair])
        .rpc();

      console.log("Create validator profile transaction signature:", tx);

      // Create a profile for the user3
      tx = await ctx.program.methods
        .createUserProfile()
        .accounts({
          state: ctx.statePda,
          userProfile: ctx.user3ProfilePda,
          user: ctx.user3Keypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.user3Keypair])
        .rpc();

      console.log("Create user3 profile transaction signature:", tx);

      // Verify the contributor profile was created correctly
      const contributorProfile = await ctx.program.account.userProfile.fetch(
        ctx.contributorProfilePda
      );
      expect(contributorProfile.user.toString()).to.equal(
        ctx.contributorKeypair.publicKey.toString()
      );
      expect(contributorProfile.permanentRepAmount.toNumber()).to.equal(0);
      expect(contributorProfile.topicTokens.length).to.equal(0);

      // Verify the validator profile was created correctly
      const validatorProfile = await ctx.program.account.userProfile.fetch(
        ctx.validatorProfilePda
      );
      expect(validatorProfile.user.toString()).to.equal(
        ctx.validatorKeypair.publicKey.toString()
      );
      expect(validatorProfile.permanentRepAmount.toNumber()).to.equal(0);
      expect(validatorProfile.topicTokens.length).to.equal(0);

      // Verify the user3 profile was created correctly
      const user3Profile = await ctx.program.account.userProfile.fetch(
        ctx.user3ProfilePda
      );
      expect(user3Profile.user.toString()).to.equal(
        ctx.user3Keypair.publicKey.toString()
      );
      expect(user3Profile.permanentRepAmount.toNumber()).to.equal(0);
      expect(user3Profile.topicTokens.length).to.equal(0);
    });

    it("Creates token accounts for all users and token types", async () => {
      // Calculate PDAs for protocol-owned temporary token accounts
      [ctx.contributorTempAlignAccount] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("user_temp_align"),
          ctx.contributorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId
      );

      [ctx.contributorTempRepAccount] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("user_temp_rep"),
          ctx.contributorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId
      );

      [ctx.validatorTempAlignAccount] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("user_temp_align"),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId
      );

      [ctx.validatorTempRepAccount] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("user_temp_rep"),
          ctx.validatorKeypair.publicKey.toBuffer(),
        ],
        ctx.program.programId
      );

      [ctx.user3TempAlignAccount] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("user_temp_align"),
          ctx.user3Keypair.publicKey.toBuffer(),
        ],
        ctx.program.programId
      );

      [ctx.user3TempRepAccount] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("user_temp_rep"),
          ctx.user3Keypair.publicKey.toBuffer(),
        ],
        ctx.program.programId
      );

      // Calculate ATAs for permanent tokens (user-owned)
      ctx.contributorAlignAta = await getAssociatedTokenAddress(
        ctx.alignMintPda,
        ctx.contributorKeypair.publicKey
      );

      ctx.contributorRepAta = await getAssociatedTokenAddress(
        ctx.repMintPda,
        ctx.contributorKeypair.publicKey
      );

      ctx.validatorAlignAta = await getAssociatedTokenAddress(
        ctx.alignMintPda,
        ctx.validatorKeypair.publicKey
      );

      ctx.validatorRepAta = await getAssociatedTokenAddress(
        ctx.repMintPda,
        ctx.validatorKeypair.publicKey
      );

      ctx.user3AlignAta = await getAssociatedTokenAddress(
        ctx.alignMintPda,
        ctx.user3Keypair.publicKey
      );

      ctx.user3RepAta = await getAssociatedTokenAddress(
        ctx.repMintPda,
        ctx.user3Keypair.publicKey
      );

      // Create protocol-owned tempAlign account for contributor
      let tx = await ctx.program.methods
        .createUserTempAlignAccount()
        .accounts({
          state: ctx.statePda,
          payer: ctx.authorityKeypair.publicKey,
          user: ctx.contributorKeypair.publicKey,
          mint: ctx.tempAlignMintPda,
          tokenAccount: ctx.contributorTempAlignAccount,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair, ctx.contributorKeypair])
        .rpc();

      console.log(
        "Create contributor's protocol-owned tempAlign account transaction signature:",
        tx
      );

      // Create protocol-owned tempRep account for contributor
      tx = await ctx.program.methods
        .createUserTempRepAccount()
        .accounts({
          state: ctx.statePda,
          payer: ctx.authorityKeypair.publicKey,
          user: ctx.contributorKeypair.publicKey,
          mint: ctx.tempRepMintPda,
          tokenAccount: ctx.contributorTempRepAccount,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair, ctx.contributorKeypair])
        .rpc();

      console.log(
        "Create contributor's protocol-owned tempRep account transaction signature:",
        tx
      );

      // Create protocol-owned tempAlign account for validator
      tx = await ctx.program.methods
        .createUserTempAlignAccount()
        .accounts({
          state: ctx.statePda,
          payer: ctx.authorityKeypair.publicKey,
          user: ctx.validatorKeypair.publicKey,
          mint: ctx.tempAlignMintPda,
          tokenAccount: ctx.validatorTempAlignAccount,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair, ctx.validatorKeypair])
        .rpc();

      console.log(
        "Create validator's protocol-owned tempAlign account transaction signature:",
        tx
      );

      // Create protocol-owned tempRep account for validator
      tx = await ctx.program.methods
        .createUserTempRepAccount()
        .accounts({
          state: ctx.statePda,
          payer: ctx.authorityKeypair.publicKey,
          user: ctx.validatorKeypair.publicKey,
          mint: ctx.tempRepMintPda,
          tokenAccount: ctx.validatorTempRepAccount,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair, ctx.validatorKeypair])
        .rpc();

      console.log(
        "Create validator's protocol-owned tempRep account transaction signature:",
        tx
      );

      // Create protocol-owned tempAlign account for user3
      tx = await ctx.program.methods
        .createUserTempAlignAccount()
        .accounts({
          state: ctx.statePda,
          payer: ctx.authorityKeypair.publicKey,
          user: ctx.user3Keypair.publicKey,
          mint: ctx.tempAlignMintPda,
          tokenAccount: ctx.user3TempAlignAccount,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair, ctx.user3Keypair])
        .rpc();

      console.log(
        "Create user3's protocol-owned tempAlign account transaction signature:",
        tx
      );

      // Create protocol-owned tempRep account for user3
      tx = await ctx.program.methods
        .createUserTempRepAccount()
        .accounts({
          state: ctx.statePda,
          payer: ctx.authorityKeypair.publicKey,
          user: ctx.user3Keypair.publicKey,
          mint: ctx.tempRepMintPda,
          tokenAccount: ctx.user3TempRepAccount,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair, ctx.user3Keypair])
        .rpc();

      console.log(
        "Create user3's protocol-owned tempRep account transaction signature:",
        tx
      );

      // Create ATA for contributor's permanent Align
      tx = await ctx.program.methods
        .createUserAta()
        .accounts({
          state: ctx.statePda,
          payer: ctx.authorityKeypair.publicKey,
          user: ctx.contributorKeypair.publicKey,
          mint: ctx.alignMintPda,
          userAta: ctx.contributorAlignAta,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair, ctx.contributorKeypair])
        .rpc();

      console.log("Create contributor's Align ATA transaction signature:", tx);

      // Create ATA for contributor's permanent Rep
      tx = await ctx.program.methods
        .createUserAta()
        .accounts({
          state: ctx.statePda,
          payer: ctx.authorityKeypair.publicKey,
          user: ctx.contributorKeypair.publicKey,
          mint: ctx.repMintPda,
          userAta: ctx.contributorRepAta,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair, ctx.contributorKeypair])
        .rpc();

      console.log("Create contributor's Rep ATA transaction signature:", tx);

      // Create ATA for validator's permanent Align
      tx = await ctx.program.methods
        .createUserAta()
        .accounts({
          state: ctx.statePda,
          payer: ctx.authorityKeypair.publicKey,
          user: ctx.validatorKeypair.publicKey,
          mint: ctx.alignMintPda,
          userAta: ctx.validatorAlignAta,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: web3.ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair, ctx.validatorKeypair])
        .rpc();

      console.log("Create validator's Align ATA transaction signature:", tx);

      // Create ATA for validator's permanent Rep
      tx = await ctx.program.methods
        .createUserAta()
        .accounts({
          state: ctx.statePda,
          payer: ctx.authorityKeypair.publicKey,
          user: ctx.validatorKeypair.publicKey,
          mint: ctx.repMintPda,
          userAta: ctx.validatorRepAta,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: web3.ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair, ctx.validatorKeypair])
        .rpc();

      console.log("Create validator's Rep ATA transaction signature:", tx);

      // Create ATA for user3's permanent Align
      tx = await ctx.program.methods
        .createUserAta()
        .accounts({
          state: ctx.statePda,
          payer: ctx.authorityKeypair.publicKey,
          user: ctx.user3Keypair.publicKey,
          mint: ctx.alignMintPda,
          userAta: ctx.user3AlignAta,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair, ctx.user3Keypair])
        .rpc();

      console.log("Create user3's Align ATA transaction signature:", tx);

      // Create ATA for user3's permanent Rep
      tx = await ctx.program.methods
        .createUserAta()
        .accounts({
          state: ctx.statePda,
          payer: ctx.authorityKeypair.publicKey,
          user: ctx.user3Keypair.publicKey,
          mint: ctx.repMintPda,
          userAta: ctx.user3RepAta,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair, ctx.user3Keypair])
        .rpc();

      console.log("Create user3's Rep ATA transaction signature:", tx);

      // Verify temporary token accounts (protocol-owned)
      // Contributor's temporary token accounts
      const contributorTempAlignData = await getAccount(
        ctx.provider.connection,
        ctx.contributorTempAlignAccount
      );
      expect(contributorTempAlignData.mint.toString()).to.equal(
        ctx.tempAlignMintPda.toString()
      );
      expect(contributorTempAlignData.owner.toString()).to.equal(
        ctx.statePda.toString()
      ); // State PDA owns account
      expect(Number(contributorTempAlignData.amount)).to.equal(0);

      const contributorTempRepData = await getAccount(
        ctx.provider.connection,
        ctx.contributorTempRepAccount
      );
      expect(contributorTempRepData.mint.toString()).to.equal(
        ctx.tempRepMintPda.toString()
      );
      expect(contributorTempRepData.owner.toString()).to.equal(
        ctx.statePda.toString()
      ); // State PDA owns account
      expect(Number(contributorTempRepData.amount)).to.equal(0);

      // Validator's temporary token accounts
      const validatorTempAlignData = await getAccount(
        ctx.provider.connection,
        ctx.validatorTempAlignAccount
      );
      expect(validatorTempAlignData.mint.toString()).to.equal(
        ctx.tempAlignMintPda.toString()
      );
      expect(validatorTempAlignData.owner.toString()).to.equal(
        ctx.statePda.toString()
      ); // State PDA owns account
      expect(Number(validatorTempAlignData.amount)).to.equal(0);

      const validatorTempRepData = await getAccount(
        ctx.provider.connection,
        ctx.validatorTempRepAccount
      );
      expect(validatorTempRepData.mint.toString()).to.equal(
        ctx.tempRepMintPda.toString()
      );
      expect(validatorTempRepData.owner.toString()).to.equal(
        ctx.statePda.toString()
      ); // State PDA owns account
      expect(Number(validatorTempRepData.amount)).to.equal(0);

      // User3's temporary token accounts
      const user3TempAlignData = await getAccount(
        ctx.provider.connection,
        ctx.user3TempAlignAccount
      );
      expect(user3TempAlignData.mint.toString()).to.equal(
        ctx.tempAlignMintPda.toString()
      );
      expect(user3TempAlignData.owner.toString()).to.equal(
        ctx.statePda.toString()
      ); // State PDA owns account
      expect(Number(user3TempAlignData.amount)).to.equal(0);

      const user3TempRepData = await getAccount(
        ctx.provider.connection,
        ctx.user3TempRepAccount
      );
      expect(user3TempRepData.mint.toString()).to.equal(
        ctx.tempRepMintPda.toString()
      );
      expect(user3TempRepData.owner.toString()).to.equal(
        ctx.statePda.toString()
      ); // State PDA owns account
      expect(Number(user3TempRepData.amount)).to.equal(0);

      // Verify permanent token accounts (user-owned ATAs)
      // Contributor's permanent token ATAs
      const contributorAlignData = await getAccount(
        ctx.provider.connection,
        ctx.contributorAlignAta
      );
      expect(contributorAlignData.mint.toString()).to.equal(
        ctx.alignMintPda.toString()
      );
      expect(contributorAlignData.owner.toString()).to.equal(
        ctx.contributorKeypair.publicKey.toString()
      );
      expect(Number(contributorAlignData.amount)).to.equal(0);

      const contributorRepData = await getAccount(
        ctx.provider.connection,
        ctx.contributorRepAta
      );
      expect(contributorRepData.mint.toString()).to.equal(
        ctx.repMintPda.toString()
      );
      expect(contributorRepData.owner.toString()).to.equal(
        ctx.contributorKeypair.publicKey.toString()
      );
      expect(Number(contributorRepData.amount)).to.equal(0);

      // Validator's permanent token ATAs
      const validatorAlignData = await getAccount(
        ctx.provider.connection,
        ctx.validatorAlignAta
      );
      expect(validatorAlignData.mint.toString()).to.equal(
        ctx.alignMintPda.toString()
      );
      expect(validatorAlignData.owner.toString()).to.equal(
        ctx.validatorKeypair.publicKey.toString()
      );
      expect(Number(validatorAlignData.amount)).to.equal(0);

      const validatorRepData = await getAccount(
        ctx.provider.connection,
        ctx.validatorRepAta
      );
      expect(validatorRepData.mint.toString()).to.equal(
        ctx.repMintPda.toString()
      );
      expect(validatorRepData.owner.toString()).to.equal(
        ctx.validatorKeypair.publicKey.toString()
      );
      expect(Number(validatorRepData.amount)).to.equal(0);

      // User3's permanent token ATAs
      const user3AlignData = await getAccount(
        ctx.provider.connection,
        ctx.user3AlignAta
      );
      expect(user3AlignData.mint.toString()).to.equal(
        ctx.alignMintPda.toString()
      );
      expect(user3AlignData.owner.toString()).to.equal(
        ctx.user3Keypair.publicKey.toString()
      );
      expect(Number(user3AlignData.amount)).to.equal(0);

      const user3RepData = await getAccount(
        ctx.provider.connection,
        ctx.user3RepAta
      );
      expect(user3RepData.mint.toString()).to.equal(
        ctx.repMintPda.toString()
      );
      expect(user3RepData.owner.toString()).to.equal(
        ctx.user3Keypair.publicKey.toString()
      );
      expect(Number(user3RepData.amount)).to.equal(0);
    });
  });
}
