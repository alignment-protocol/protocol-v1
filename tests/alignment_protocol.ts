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

  // We'll define two separate keypairs:
  // 1) authorityKeypair: the "admin" or "deployer" controlling "DUp7hZ..."
  //    (In real usage, you'd import the actual key from a file or environment.)
  // 2) userKeypair: a normal user who will create an ATA and submit data

  // Load the keypair from the JSON file
  const secretKeyString = fs.readFileSync("/Users/cheul/.config/solana/id.json", "utf8");
  const secretKey = Uint8Array.from(JSON.parse(secretKeyString));

  // Instead of generating a new keypair, load it from the file:
  const authorityKeypair = web3.Keypair.fromSecretKey(secretKey);
  const userKeypair = web3.Keypair.generate();

  let statePda: web3.PublicKey;
  let mintPda: web3.PublicKey;

  it("Airdrop devnet SOL to both authority and user", async () => {
    // Airdrop user
    let sigU = await provider.connection.requestAirdrop(
      userKeypair.publicKey,
      2 * web3.LAMPORTS_PER_SOL
    );

    const latestBlockhash = await provider.connection.getLatestBlockhash();

    await provider.connection.confirmTransaction({
      ...latestBlockhash,
      signature: sigU,
    });

    console.log("Authority:", authorityKeypair.publicKey.toBase58());
    console.log("User:", userKeypair.publicKey.toBase58());
  });

  it("Initializes the protocol with authority = DUp7... (or the test authority)", async () => {
    // We'll just sign with authorityKeypair we have. For real usage, you'd need
    // the private key matching "DUp7hZ..."
    // But let's pretend we do to demonstrate storing a "hard-coded" authority.

    [statePda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("state")],
      program.programId
    );
    [mintPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("mint")],
      program.programId
    );

    let txSig = await program.methods
      .initialize()
      .accounts({
        state: statePda,
        mint: mintPda,
        authority: authorityKeypair.publicKey, // The actual signer paying fees
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authorityKeypair])
      .rpc();
    console.log("Init txSig:", txSig);

    // Check that state is created
    const stateAccount = await program.account.state.fetch(statePda);
    console.log("State Account:", stateAccount);

    // stateAccount.authority might or might not match forcedAuthorityPubkey,
    // depending on how your code in 'initialize' is written.
    // If you're storing authority_key() from the param, verify here:
    expect(stateAccount.authority.toBase58()).to.equal(
      authorityKeypair.publicKey.toBase58()
    );
    // or: forcedAuthorityPubkey.toBase58()

    expect(stateAccount.submissionCount.toNumber()).to.equal(0);

    // Confirm the mint is correct
    const mintInfo = await getMint(provider.connection, mintPda);
    expect(mintInfo.decimals).to.equal(0);
  });

  it("Checks and creates the user's ATA explicitly", async () => {
    // Derive the user's ATA
    const userAta = await getAssociatedTokenAddress(mintPda, userKeypair.publicKey);

    // Check if the ATA already exists
    let ataAccountInfo = await provider.connection.getAccountInfo(userAta);
    if (ataAccountInfo !== null) {
      console.log("ATA already exists at", userAta.toBase58(), "Skipping creation...");
    } else {
      // The ATA doesn't exist, let's create it
      const tx = await program.methods
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
      console.log("createUserAta tx:", tx);

      // Confirm now it exists
      ataAccountInfo = await provider.connection.getAccountInfo(userAta);
      expect(ataAccountInfo).to.not.be.null;
      console.log("ATA created at", userAta.toBase58());
    }

    // For a "thorough" test, we can attempt a second creation and expect an error:
    let threwError = false;
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
    } catch (err) {
      threwError = true;
      console.log("As expected, second attempt to create the same ATA failed.");
    }
    expect(threwError).to.be.true;
  });

  it("Submits data from the user and mints tokens to the user's ATA", async () => {
    // We'll read the current submission_count from the state
    const stateAccount = await program.account.state.fetch(statePda);
    const currentCount = stateAccount.submissionCount.toNumber();

    // The next submission PDA
    const [submissionPda] = web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("submission"),
        new anchor.BN(currentCount).toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );

    // The user already has an ATA
    const userAta = await getAssociatedTokenAddress(mintPda, userKeypair.publicKey);

    // We'll pass some data
    const dataStr = "Test data from devnet user";
    const tokensToMint = new anchor.BN(42);

    const txSig = await program.methods
      .submitData(dataStr, tokensToMint)
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

    // Now the submission_count should have incremented
    const updatedState = await program.account.state.fetch(statePda);
    expect(updatedState.submissionCount.toNumber()).to.equal(currentCount + 1);

    // Fetch the new Submission account
    const submissionAccount = await program.account.submission.fetch(submissionPda);
    expect(submissionAccount.contributor.toBase58()).to.equal(
      userKeypair.publicKey.toBase58()
    );
    expect(submissionAccount.data).to.equal(dataStr);
    console.log("Submission Account:", submissionAccount);

    // Confirm the user now has 42 tokens
    const ataInfo = await getAccount(provider.connection, userAta);
    console.log("User ATA token balance:", Number(ataInfo.amount));
    expect(Number(ataInfo.amount)).to.equal(42);
  });
});
