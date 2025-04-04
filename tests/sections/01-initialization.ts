import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { web3 } from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID, getMint } from "@solana/spl-token";
import { TestContext } from "../utils/test-setup";

export function runInitializationTests(ctx: TestContext): void {
  describe("Protocol Initialization", () => {
    it("Initializes the protocol in multiple steps to prevent stack overflow", async () => {
      // Step 1: Initialize the state account
      const stateTx = await ctx.program.methods
        .initializeState()
        .accounts({
          state: ctx.statePda,
          authority: ctx.authorityKeypair.publicKey,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair])
        .rpc();

      console.log("Initialize state transaction signature:", stateTx);

      // Fetch the state account to verify initialization
      let stateAcc = await ctx.program.account.state.fetch(ctx.statePda);

      // Check initial state properties
      expect(stateAcc.authority.toString()).to.equal(
        ctx.authorityKeypair.publicKey.toString(),
      );
      expect(stateAcc.topicCount.toNumber()).to.equal(0);
      expect(stateAcc.tokensToMint.toNumber()).to.equal(0);

      // Check default voting phase durations (24 hours in seconds)
      expect(stateAcc.defaultCommitPhaseDuration.toNumber()).to.equal(
        24 * 60 * 60,
      );
      expect(stateAcc.defaultRevealPhaseDuration.toNumber()).to.equal(
        24 * 60 * 60,
      );

      // Step 2a: Initialize temp_align_mint
      const tempAlignTx = await ctx.program.methods
        .initializeTempAlignMint()
        .accounts({
          state: ctx.statePda,
          tempAlignMint: ctx.tempAlignMintPda,
          authority: ctx.authorityKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair])
        .rpc();

      console.log(
        "Initialize temp_align_mint transaction signature:",
        tempAlignTx,
      );

      // Step 2b: Initialize align_mint
      const alignTx = await ctx.program.methods
        .initializeAlignMint()
        .accounts({
          state: ctx.statePda,
          alignMint: ctx.alignMintPda,
          authority: ctx.authorityKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair])
        .rpc();

      console.log("Initialize align_mint transaction signature:", alignTx);

      // Step 2c: Initialize temp_rep_mint
      const tempRepTx = await ctx.program.methods
        .initializeTempRepMint()
        .accounts({
          state: ctx.statePda,
          tempRepMint: ctx.tempRepMintPda,
          authority: ctx.authorityKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair])
        .rpc();

      console.log("Initialize temp_rep_mint transaction signature:", tempRepTx);

      // Step 2d: Initialize rep_mint
      const repTx = await ctx.program.methods
        .initializeRepMint()
        .accounts({
          state: ctx.statePda,
          repMint: ctx.repMintPda,
          authority: ctx.authorityKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([ctx.authorityKeypair])
        .rpc();

      console.log("Initialize rep_mint transaction signature:", repTx);

      // Fetch the state account again to verify mint initialization
      stateAcc = await ctx.program.account.state.fetch(ctx.statePda);

      // Check that all mints are correctly set
      expect(stateAcc.tempAlignMint.toString()).to.equal(
        ctx.tempAlignMintPda.toString(),
      );
      expect(stateAcc.alignMint.toString()).to.equal(
        ctx.alignMintPda.toString(),
      );
      expect(stateAcc.tempRepMint.toString()).to.equal(
        ctx.tempRepMintPda.toString(),
      );
      expect(stateAcc.repMint.toString()).to.equal(ctx.repMintPda.toString());

      // Verify the mints exist and have the correct properties
      const tempAlignMintInfo = await getMint(
        ctx.provider.connection,
        ctx.tempAlignMintPda,
      );
      const alignMintInfo = await getMint(
        ctx.provider.connection,
        ctx.alignMintPda,
      );
      const tempRepMintInfo = await getMint(
        ctx.provider.connection,
        ctx.tempRepMintPda,
      );
      const repMintInfo = await getMint(
        ctx.provider.connection,
        ctx.repMintPda,
      );

      // Check all mints have 0 decimals
      expect(tempAlignMintInfo.decimals).to.equal(0);
      expect(alignMintInfo.decimals).to.equal(0);
      expect(tempRepMintInfo.decimals).to.equal(0);
      expect(repMintInfo.decimals).to.equal(0);

      // Check mint and freeze authorities are set to the state PDA
      expect(tempAlignMintInfo.mintAuthority.toString()).to.equal(
        ctx.statePda.toString(),
      );
      expect(tempAlignMintInfo.freezeAuthority.toString()).to.equal(
        ctx.statePda.toString(),
      );
      expect(alignMintInfo.mintAuthority.toString()).to.equal(
        ctx.statePda.toString(),
      );
      expect(alignMintInfo.freezeAuthority.toString()).to.equal(
        ctx.statePda.toString(),
      );
      expect(tempRepMintInfo.mintAuthority.toString()).to.equal(
        ctx.statePda.toString(),
      );
      expect(tempRepMintInfo.freezeAuthority.toString()).to.equal(
        ctx.statePda.toString(),
      );
      expect(repMintInfo.mintAuthority.toString()).to.equal(
        ctx.statePda.toString(),
      );
      expect(repMintInfo.freezeAuthority.toString()).to.equal(
        ctx.statePda.toString(),
      );
    });

    it("Sets tokens_to_mint to a non-zero value", async () => {
      // Define the new tokens to mint value
      const tokensToMint = 100;

      // Update tokens_to_mint value
      const tx = await ctx.program.methods
        .updateTokensToMint(new anchor.BN(tokensToMint))
        .accounts({
          state: ctx.statePda,
          authority: ctx.authorityKeypair.publicKey,
        })
        .signers([ctx.authorityKeypair])
        .rpc();

      console.log("Update tokens_to_mint transaction signature:", tx);

      // Verify the state was updated
      const stateAcc = await ctx.program.account.state.fetch(ctx.statePda);
      expect(stateAcc.tokensToMint.toNumber()).to.equal(tokensToMint);
    });
  });
}
