import * as anchor from "@coral-xyz/anchor";
import { Program, AnchorProvider, web3 } from "@coral-xyz/anchor";
import { AlignmentProtocol } from "../target/types/alignment_protocol";
import { expect } from "chai";
import {
  TOKEN_PROGRAM_ID,
  getAccount,
  getMint,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import * as fs from "fs";

describe("alignment-protocol-devnet", () => {
  // Set up provider for devnet
  const provider = AnchorProvider.env();
  anchor.setProvider(provider);

  // Our program from the workspace
  const program = anchor.workspace.AlignmentProtocol as Program<AlignmentProtocol>;

  // Keypairs:
  // 1) authorityKeypair: "admin/deployer"
  // 2) userKeypair: normal user who will create an ATA and submit data
  const secretKeyString = fs.readFileSync("/Users/cheul/.config/solana/id.json", "utf8");
  const secretKey = Uint8Array.from(JSON.parse(secretKeyString));
  const authorityKeypair = web3.Keypair.fromSecretKey(secretKey);
  const userKeypair = web3.Keypair.generate();

  let statePda: web3.PublicKey;
  let mintPda: web3.PublicKey;
  let userAta: web3.PublicKey;

  // -----------------------------------
  // 1) Before All: Transfer SOL to user
  // -----------------------------------
  before("Transfer devnet SOL to the userKeypair", async () => {
    // 0.1 SOL
    const lamports = 0.1 * web3.LAMPORTS_PER_SOL;
    // Build & send tx
    const tx = new web3.Transaction().add(
      web3.SystemProgram.transfer({
        fromPubkey: authorityKeypair.publicKey,
        toPubkey: userKeypair.publicKey,
        lamports,
      })
    );
    const sigU = await provider.sendAndConfirm(tx, [authorityKeypair]);
    console.log("Transfer signature:", sigU);
    console.log("Authority:", authorityKeypair.publicKey.toBase58());
    console.log("User:", userKeypair.publicKey.toBase58());
  });

  // Derive PDAs just once (assuming they won't change).
  before("Derive statePda and mintPda", () => {
    [statePda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("state")],
      program.programId
    );
    [mintPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("mint")],
      program.programId
    );
  });

  // -----------------------------------------------------------
  // 2) Test: Initialize if not already
  // -----------------------------------------------------------
  it("Initializes the protocol if State is missing", async () => {
    // Attempt to fetch the State account
    let stateAccount = null;
    try {
      stateAccount = await program.account.state.fetch(statePda);
    } catch (err) {
      // If fetch fails, the state doesn't exist yet
    }

    if (stateAccount) {
      console.log("State already exists. Skipping initialization.");
    } else {
      const initSig = await program.methods
        .initialize()
        .accounts({
          state: statePda,
          mint: mintPda,
          authority: authorityKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: web3.SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([authorityKeypair])
        .rpc();
      console.log("initialize() txSig:", initSig);
      stateAccount = await program.account.state.fetch(statePda);
      // submissionCount starts at 0
      expect(stateAccount.submissionCount.toNumber()).to.equal(0);
    }

    console.log("stateAccount:", stateAccount);

    // Verify the state is now available
    expect(stateAccount.authority.toBase58()).to.equal(
      authorityKeypair.publicKey.toBase58()
    );
    
    // Mint must have decimals=0
    const mintInfo = await getMint(provider.connection, mintPda);
    expect(mintInfo.decimals).to.equal(0);
  });

  // -----------------------------------------------------------
  // 3) Test: Update tokensToMint if it's 0
  // -----------------------------------------------------------
  it("Updates tokens_to_mint to 1 (if zero)", async () => {
    const stateAccount = await program.account.state.fetch(statePda);
    const currentTokensToMint = stateAccount.tokensToMint.toNumber();
    console.log("tokens_to_mint current:", currentTokensToMint);

    if (currentTokensToMint === 0) {
      const txSig = await program.methods
        .updateTokensToMint(new anchor.BN(1))
        .accounts({
          state: statePda,
          authority: authorityKeypair.publicKey,
        })
        .signers([authorityKeypair])
        .rpc();
      console.log("updateTokensToMint txSig:", txSig);

      const updatedState = await program.account.state.fetch(statePda);
      expect(updatedState.tokensToMint.toNumber()).to.equal(1);
    } else {
      console.log("tokens_to_mint is already nonzero; skipping update.");
    }
  });

  // -----------------------------------------------------------
  // 4) Test: Create the user's ATA
  // -----------------------------------------------------------
  it("Explicitly creates user ATA, then checks second creation fails", async () => {
    // Derive user ATA
    userAta = await getAssociatedTokenAddress(mintPda, userKeypair.publicKey);

    // Check if the ATA already exists
    let existingAtaInfo = await provider.connection.getAccountInfo(userAta);
    if (existingAtaInfo) {
      console.log("User ATA already exists. Skipping creation.");
    } else {
      // Create it
      const txSig = await program.methods
        .createUserAta()
        .accounts({
          payer: userKeypair.publicKey,
          user: userKeypair.publicKey,
          mint: mintPda,
          userAta: userAta,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([userKeypair])
        .rpc();
      console.log("createUserAta tx:", txSig);

      // Confirm it exists now
      existingAtaInfo = await provider.connection.getAccountInfo(userAta);
      expect(existingAtaInfo).to.not.be.null;
      console.log("ATA created at", userAta.toBase58());
    }

    // Attempt second creation -> expect error
    let threw = false;
    try {
      await program.methods
        .createUserAta()
        .accounts({
          payer: userKeypair.publicKey,
          user: userKeypair.publicKey,
          mint: mintPda,
          userAta: userAta,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([userKeypair])
        .rpc();
    } catch (e) {
      threw = true;
      console.log("As expected, second creation failed:", e.error);
    }
    expect(threw).to.be.true;
  });

  // -----------------------------------------------------------
  // 5) Test: Submit data
  // -----------------------------------------------------------
  it("Submits data from user, awarding tokens", async () => {
    // Before: get the user's ATA balance
    const stateAccount = await program.account.state.fetch(statePda);
    const currCount = stateAccount.submissionCount.toNumber();

    const ataInfoBefore = await getAccount(provider.connection, userAta);
    const beforeBalance = Number(ataInfoBefore.amount);
    console.log("User ATA token balance (before):", beforeBalance);

    // Derive next submission PDA
    const [submissionPda] = web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("submission"),
        new anchor.BN(currCount).toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );

    // Submit
    const dataStr = "Test data from devnet user";
    const txSig = await program.methods
      .submitData(dataStr)
      .accounts({
        state: statePda,
        mint: mintPda,
        contributorAta: userAta,
        submission: submissionPda,
        contributor: userKeypair.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([userKeypair])
      .rpc();
    console.log("submitData txSig:", txSig);

    // Check state
    const updatedState = await program.account.state.fetch(statePda);
    expect(updatedState.submissionCount.toNumber()).to.equal(currCount + 1);

    // Check new Submission account
    const submissionAccount = await program.account.submission.fetch(submissionPda);
    expect(submissionAccount.contributor.toBase58()).to.equal(
      userKeypair.publicKey.toBase58()
    );
    expect(submissionAccount.data).to.equal(dataStr);
    console.log("Submission account:", submissionAccount);

    // Confirm user now has 1 more token (assuming tokens_to_mint=1)
    const ataInfoAfter = await getAccount(provider.connection, userAta);
    const afterBalance = Number(ataInfoAfter.amount);
    console.log("User ATA token balance (after):", afterBalance);
    expect(afterBalance).to.equal(beforeBalance + 1);

    // Finally, log the state account
    const finalStateAccount = await program.account.state.fetch(statePda);
    console.log("stateAccount:", finalStateAccount);
  });
});
