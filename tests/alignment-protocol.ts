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

describe("alignment-protocol", () => {
  // Set up provider for localnet
  const provider = AnchorProvider.env();
  anchor.setProvider(provider);

  // Our program from the workspace
  const program = anchor.workspace.AlignmentProtocol as Program<AlignmentProtocol>;

  // Load authority keypair from local solana config for tests
  const secretKeyString = fs.readFileSync("/Users/cheul/.config/solana/id.json", "utf8");
  const secretKey = Uint8Array.from(JSON.parse(secretKeyString));
  const authorityKeypair = web3.Keypair.fromSecretKey(secretKey);
  
  // Generate additional keypairs for tests
  const contributorKeypair = web3.Keypair.generate();
  const validatorKeypair = web3.Keypair.generate();
  const user3Keypair = web3.Keypair.generate(); // For additional testing
  
  // PDAs and account variables
  let statePda: web3.PublicKey;
  let tempAlignMintPda: web3.PublicKey;
  let alignMintPda: web3.PublicKey;
  let tempRepMintPda: web3.PublicKey;
  let repMintPda: web3.PublicKey;
  
  // Topic PDAs
  let topic1Pda: web3.PublicKey;
  let topic2Pda: web3.PublicKey;
  
  // User ATAs
  let contributorTempAlignAta: web3.PublicKey;
  let contributorAlignAta: web3.PublicKey;
  let validatorTempRepAta: web3.PublicKey;
  let validatorRepAta: web3.PublicKey;
  
  // User profiles
  let contributorProfilePda: web3.PublicKey;
  let validatorProfilePda: web3.PublicKey;
  
  // Submission tracking
  let submissionPda: web3.PublicKey;
  let submissionTopicLinkPda: web3.PublicKey;
  let crossTopicLinkPda: web3.PublicKey;
  
  // Vote tracking
  let voteCommitPda: web3.PublicKey;
  
  // Constants for testing
  const TOPIC1_NAME = "AI Safety";
  const TOPIC1_DESCRIPTION = "Alignment, interpretability, and safety research for AI systems";
  const TOPIC2_NAME = "Climate";
  const TOPIC2_DESCRIPTION = "Climate change mitigation and adaptation strategies";
  const SUBMISSION_DATA = "ipfs://QmULkt3mMt5K8XHnYYxmnvtUGZ4p1qGQgvTKYwXkUxBcmx";
  
  // Vote nonce and secrets
  const VOTE_NONCE = "my-secret-nonce-123";
  const VOTE_CHOICE_YES = { yes: {} };
  let voteHash: number[] = [];

  // ========== BEFORE HOOKS ==========

  before("Fund test accounts with SOL", async () => {
    // Fund each test account with 1 SOL
    const lamports = 1 * web3.LAMPORTS_PER_SOL;
    
    // Build and send transactions for funding
    for (const keypair of [contributorKeypair, validatorKeypair, user3Keypair]) {
      const tx = new web3.Transaction().add(
        web3.SystemProgram.transfer({
          fromPubkey: authorityKeypair.publicKey,
          toPubkey: keypair.publicKey,
          lamports,
        })
      );
      await provider.sendAndConfirm(tx, [authorityKeypair]);
    }
    
    console.log("Authority:", authorityKeypair.publicKey.toBase58());
    console.log("Contributor:", contributorKeypair.publicKey.toBase58());
    console.log("Validator:", validatorKeypair.publicKey.toBase58());
  });

  before("Derive program PDAs", () => {
    // State PDA
    [statePda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("state")],
      program.programId
    );
    
    // Token mint PDAs
    [tempAlignMintPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("temp_align_mint")],
      program.programId
    );
    
    [alignMintPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("align_mint")],
      program.programId
    );
    
    [tempRepMintPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("temp_rep_mint")],
      program.programId
    );
    
    [repMintPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("rep_mint")],
      program.programId
    );
    
    // User profile PDAs
    [contributorProfilePda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("user_profile"), contributorKeypair.publicKey.toBuffer()],
      program.programId
    );
    
    [validatorProfilePda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("user_profile"), validatorKeypair.publicKey.toBuffer()],
      program.programId
    );
  });

  // ========== TEST SECTION 1: INITIALIZATION ==========

  it("Initializes the protocol in multiple steps to prevent stack overflow", async () => {
    // Step 1: Initialize the state account
    const stateTx = await program.methods
      .initializeState()
      .accounts({
        state: statePda,
        authority: authorityKeypair.publicKey,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authorityKeypair])
      .rpc();
    
    console.log("Initialize state transaction signature:", stateTx);
    
    // Fetch the state account to verify initialization
    let stateAcc = await program.account.state.fetch(statePda);
    
    // Check initial state properties
    expect(stateAcc.authority.toString()).to.equal(authorityKeypair.publicKey.toString());
    expect(stateAcc.submissionCount.toNumber()).to.equal(0);
    expect(stateAcc.topicCount.toNumber()).to.equal(0);
    expect(stateAcc.tokensToMint.toNumber()).to.equal(0);
    
    // Check default voting phase durations (24 hours in seconds)
    expect(stateAcc.defaultCommitPhaseDuration.toNumber()).to.equal(24 * 60 * 60);
    expect(stateAcc.defaultRevealPhaseDuration.toNumber()).to.equal(24 * 60 * 60);
    
    // Step 2a: Initialize temp_align_mint
    const tempAlignTx = await program.methods
      .initializeTempAlignMint()
      .accounts({
        state: statePda,
        tempAlignMint: tempAlignMintPda,
        authority: authorityKeypair.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authorityKeypair])
      .rpc();
    
    console.log("Initialize temp_align_mint transaction signature:", tempAlignTx);
    
    // Step 2b: Initialize align_mint
    const alignTx = await program.methods
      .initializeAlignMint()
      .accounts({
        state: statePda,
        alignMint: alignMintPda,
        authority: authorityKeypair.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authorityKeypair])
      .rpc();
    
    console.log("Initialize align_mint transaction signature:", alignTx);
    
    // Step 2c: Initialize temp_rep_mint
    const tempRepTx = await program.methods
      .initializeTempRepMint()
      .accounts({
        state: statePda,
        tempRepMint: tempRepMintPda,
        authority: authorityKeypair.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authorityKeypair])
      .rpc();
    
    console.log("Initialize temp_rep_mint transaction signature:", tempRepTx);
    
    // Step 2d: Initialize rep_mint
    const repTx = await program.methods
      .initializeRepMint()
      .accounts({
        state: statePda,
        repMint: repMintPda,
        authority: authorityKeypair.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authorityKeypair])
      .rpc();
    
    console.log("Initialize rep_mint transaction signature:", repTx);
    
    // Fetch the state account again to verify mint initialization
    stateAcc = await program.account.state.fetch(statePda);
    
    // Check that all mints are correctly set
    expect(stateAcc.tempAlignMint.toString()).to.equal(tempAlignMintPda.toString());
    expect(stateAcc.alignMint.toString()).to.equal(alignMintPda.toString());
    expect(stateAcc.tempRepMint.toString()).to.equal(tempRepMintPda.toString());
    expect(stateAcc.repMint.toString()).to.equal(repMintPda.toString());
    
    // Verify the mints exist and have the correct properties
    const tempAlignMintInfo = await getMint(provider.connection, tempAlignMintPda);
    const alignMintInfo = await getMint(provider.connection, alignMintPda);
    const tempRepMintInfo = await getMint(provider.connection, tempRepMintPda);
    const repMintInfo = await getMint(provider.connection, repMintPda);
    
    // Check all mints have 0 decimals
    expect(tempAlignMintInfo.decimals).to.equal(0);
    expect(alignMintInfo.decimals).to.equal(0);
    expect(tempRepMintInfo.decimals).to.equal(0);
    expect(repMintInfo.decimals).to.equal(0);
    
    // Check mint and freeze authorities are set to the state PDA
    expect(tempAlignMintInfo.mintAuthority.toString()).to.equal(statePda.toString());
    expect(tempAlignMintInfo.freezeAuthority.toString()).to.equal(statePda.toString());
    expect(alignMintInfo.mintAuthority.toString()).to.equal(statePda.toString());
    expect(alignMintInfo.freezeAuthority.toString()).to.equal(statePda.toString());
    expect(tempRepMintInfo.mintAuthority.toString()).to.equal(statePda.toString());
    expect(tempRepMintInfo.freezeAuthority.toString()).to.equal(statePda.toString());
    expect(repMintInfo.mintAuthority.toString()).to.equal(statePda.toString());
    expect(repMintInfo.freezeAuthority.toString()).to.equal(statePda.toString());
  });

  it("Sets tokens_to_mint to a non-zero value", async () => {
    // Define the new tokens to mint value
    const tokensToMint = 100;
    
    // Update tokens_to_mint value
    const tx = await program.methods
      .updateTokensToMint(new anchor.BN(tokensToMint))
      .accounts({
        state: statePda,
        authority: authorityKeypair.publicKey,
      })
      .signers([authorityKeypair])
      .rpc();
    
    console.log("Update tokens_to_mint transaction signature:", tx);
    
    // Verify the state was updated
    const stateAcc = await program.account.state.fetch(statePda);
    expect(stateAcc.tokensToMint.toNumber()).to.equal(tokensToMint);
  });

  // ========== TEST SECTION 2: TOPIC MANAGEMENT ==========

  it("Creates the first topic", async () => {
    // Derive the topic PDA for the first topic (ID = 0)
    [topic1Pda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("topic"), Buffer.from([0, 0, 0, 0, 0, 0, 0, 0])],
      program.programId
    );
    
    // Create the first topic
    const tx = await program.methods
      .createTopic(
        TOPIC1_NAME,
        TOPIC1_DESCRIPTION,
        null,  // Use default commit phase duration
        null   // Use default reveal phase duration
      )
      .accounts({
        state: statePda,
        topic: topic1Pda,
        authority: authorityKeypair.publicKey,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authorityKeypair])
      .rpc();
    
    console.log("Create first topic transaction signature:", tx);
    
    // Fetch and verify the topic data
    const topicAcc = await program.account.topic.fetch(topic1Pda);
    expect(topicAcc.id.toNumber()).to.equal(0);
    expect(topicAcc.name).to.equal(TOPIC1_NAME);
    expect(topicAcc.description).to.equal(TOPIC1_DESCRIPTION);
    expect(topicAcc.authority.toString()).to.equal(authorityKeypair.publicKey.toString());
    expect(topicAcc.submissionCount.toNumber()).to.equal(0);
    expect(topicAcc.isActive).to.be.true;
    
    // Verify that the topic count in state was incremented
    const stateAcc = await program.account.state.fetch(statePda);
    expect(stateAcc.topicCount.toNumber()).to.equal(1);
    
    // Verify the default durations were set correctly
    expect(topicAcc.commitPhaseDuration.toNumber()).to.equal(stateAcc.defaultCommitPhaseDuration.toNumber());
    expect(topicAcc.revealPhaseDuration.toNumber()).to.equal(stateAcc.defaultRevealPhaseDuration.toNumber());
  });

  it("Creates a second topic", async () => {
    // Derive the topic PDA for the second topic (ID = 1)
    [topic2Pda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("topic"), Buffer.from([1, 0, 0, 0, 0, 0, 0, 0])],
      program.programId
    );
    
    // Create the second topic with custom phase durations
    const customCommitDuration = 12 * 60 * 60; // 12 hours in seconds
    const customRevealDuration = 12 * 60 * 60; // 12 hours in seconds
    
    const tx = await program.methods
      .createTopic(
        TOPIC2_NAME,
        TOPIC2_DESCRIPTION,
        new anchor.BN(customCommitDuration),
        new anchor.BN(customRevealDuration)
      )
      .accounts({
        state: statePda,
        topic: topic2Pda,
        authority: authorityKeypair.publicKey,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authorityKeypair])
      .rpc();
    
    console.log("Create second topic transaction signature:", tx);
    
    // Fetch and verify the topic data
    const topicAcc = await program.account.topic.fetch(topic2Pda);
    expect(topicAcc.id.toNumber()).to.equal(1);
    expect(topicAcc.name).to.equal(TOPIC2_NAME);
    expect(topicAcc.description).to.equal(TOPIC2_DESCRIPTION);
    expect(topicAcc.authority.toString()).to.equal(authorityKeypair.publicKey.toString());
    expect(topicAcc.submissionCount.toNumber()).to.equal(0);
    expect(topicAcc.isActive).to.be.true;
    
    // Verify that the topic count in state was incremented
    const stateAcc = await program.account.state.fetch(statePda);
    expect(stateAcc.topicCount.toNumber()).to.equal(2);
    
    // Verify the custom durations were set correctly
    expect(topicAcc.commitPhaseDuration.toNumber()).to.equal(customCommitDuration);
    expect(topicAcc.revealPhaseDuration.toNumber()).to.equal(customRevealDuration);
  });

  // ========== TEST SECTION 3: USER SETUP ==========

  it("Creates user profiles for contributor and validator", async () => {
    // Create a profile for the contributor
    let tx = await program.methods
      .createUserProfile()
      .accounts({
        state: statePda,
        userProfile: contributorProfilePda,
        user: contributorKeypair.publicKey,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([contributorKeypair])
      .rpc();
    
    console.log("Create contributor profile transaction signature:", tx);
    
    // Create a profile for the validator
    tx = await program.methods
      .createUserProfile()
      .accounts({
        state: statePda,
        userProfile: validatorProfilePda,
        user: validatorKeypair.publicKey,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([validatorKeypair])
      .rpc();
    
    console.log("Create validator profile transaction signature:", tx);
    
    // Verify the contributor profile was created correctly
    const contributorProfile = await program.account.userProfile.fetch(contributorProfilePda);
    expect(contributorProfile.user.toString()).to.equal(contributorKeypair.publicKey.toString());
    expect(contributorProfile.permanentRepAmount.toNumber()).to.equal(0);
    expect(contributorProfile.topicTokens.length).to.equal(0);
    
    // Verify the validator profile was created correctly
    const validatorProfile = await program.account.userProfile.fetch(validatorProfilePda);
    expect(validatorProfile.user.toString()).to.equal(validatorKeypair.publicKey.toString());
    expect(validatorProfile.permanentRepAmount.toNumber()).to.equal(0);
    expect(validatorProfile.topicTokens.length).to.equal(0);
  });

  it("Creates ATAs for all users and token types", async () => {
    // Calculate the ATAs for contributor
    contributorTempAlignAta = await getAssociatedTokenAddress(
      tempAlignMintPda,
      contributorKeypair.publicKey
    );
    
    contributorAlignAta = await getAssociatedTokenAddress(
      alignMintPda,
      contributorKeypair.publicKey
    );
    
    // Calculate the ATAs for validator
    validatorTempRepAta = await getAssociatedTokenAddress(
      tempRepMintPda,
      validatorKeypair.publicKey
    );
    
    validatorRepAta = await getAssociatedTokenAddress(
      repMintPda,
      validatorKeypair.publicKey
    );
    
    // Create ATA for contributor's tempAlign
    let tx = await program.methods
      .createUserAta()
      .accounts({
        state: statePda,
        payer: authorityKeypair.publicKey,
        user: contributorKeypair.publicKey,
        mint: tempAlignMintPda,
        userAta: contributorTempAlignAta,
        systemProgram: web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: anchor.web3.ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authorityKeypair, contributorKeypair])
      .rpc();
    
    console.log("Create contributorTempAlignAta transaction signature:", tx);
    
    // Create ATA for contributor's Align
    tx = await program.methods
      .createUserAta()
      .accounts({
        state: statePda,
        payer: authorityKeypair.publicKey,
        user: contributorKeypair.publicKey,
        mint: alignMintPda,
        userAta: contributorAlignAta,
        systemProgram: web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: anchor.web3.ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authorityKeypair, contributorKeypair])
      .rpc();
    
    console.log("Create contributorAlignAta transaction signature:", tx);
    
    // Create ATA for validator's tempRep
    tx = await program.methods
      .createUserAta()
      .accounts({
        state: statePda,
        payer: authorityKeypair.publicKey,
        user: validatorKeypair.publicKey,
        mint: tempRepMintPda,
        userAta: validatorTempRepAta,
        systemProgram: web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: anchor.web3.ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authorityKeypair, validatorKeypair])
      .rpc();
    
    console.log("Create validatorTempRepAta transaction signature:", tx);
    
    // Create ATA for validator's Rep
    tx = await program.methods
      .createUserAta()
      .accounts({
        state: statePda,
        payer: authorityKeypair.publicKey,
        user: validatorKeypair.publicKey,
        mint: repMintPda,
        userAta: validatorRepAta,
        systemProgram: web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: anchor.web3.ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authorityKeypair, validatorKeypair])
      .rpc();
    
    console.log("Create validatorRepAta transaction signature:", tx);
    
    // Verify that all token accounts were created
    const contributorTempAlignAccount = await getAccount(
      provider.connection,
      contributorTempAlignAta
    );
    expect(contributorTempAlignAccount.mint.toString()).to.equal(tempAlignMintPda.toString());
    expect(contributorTempAlignAccount.owner.toString()).to.equal(contributorKeypair.publicKey.toString());
    expect(Number(contributorTempAlignAccount.amount)).to.equal(0);
    
    const contributorAlignAccount = await getAccount(
      provider.connection,
      contributorAlignAta
    );
    expect(contributorAlignAccount.mint.toString()).to.equal(alignMintPda.toString());
    expect(contributorAlignAccount.owner.toString()).to.equal(contributorKeypair.publicKey.toString());
    expect(Number(contributorAlignAccount.amount)).to.equal(0);
    
    const validatorTempRepAccount = await getAccount(
      provider.connection,
      validatorTempRepAta
    );
    expect(validatorTempRepAccount.mint.toString()).to.equal(tempRepMintPda.toString());
    expect(validatorTempRepAccount.owner.toString()).to.equal(validatorKeypair.publicKey.toString());
    expect(Number(validatorTempRepAccount.amount)).to.equal(0);
    
    const validatorRepAccount = await getAccount(
      provider.connection,
      validatorRepAta
    );
    expect(validatorRepAccount.mint.toString()).to.equal(repMintPda.toString());
    expect(validatorRepAccount.owner.toString()).to.equal(validatorKeypair.publicKey.toString());
    expect(Number(validatorRepAccount.amount)).to.equal(0);
  });

  // ========== TEST SECTION 4: SUBMISSION FLOW ==========

  it("Submits data to the first topic", async () => {
    // Derive the submission PDA (using the current submission count = 0)
    [submissionPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("submission"), Buffer.from([0, 0, 0, 0, 0, 0, 0, 0])],
      program.programId
    );
    
    // Derive the submission-topic link PDA
    [submissionTopicLinkPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("submission_topic_link"), submissionPda.toBuffer(), topic1Pda.toBuffer()],
      program.programId
    );
    
    // Submit data to the first topic
    const tx = await program.methods
      .submitDataToTopic(SUBMISSION_DATA)
      .accounts({
        state: statePda,
        topic: topic1Pda,
        tempAlignMint: tempAlignMintPda,
        contributorAta: contributorTempAlignAta,
        submission: submissionPda,
        submissionTopicLink: submissionTopicLinkPda,
        contributorProfile: contributorProfilePda,
        contributor: contributorKeypair.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([contributorKeypair])
      .rpc();
    
    console.log("Submit data transaction signature:", tx);
    
    // Verify the submission was created correctly
    const submissionAcc = await program.account.submission.fetch(submissionPda);
    expect(submissionAcc.contributor.toString()).to.equal(contributorKeypair.publicKey.toString());
    expect(submissionAcc.dataReference).to.equal(SUBMISSION_DATA);
    
    // Verify the submission-topic link was created correctly
    const linkAcc = await program.account.submissionTopicLink.fetch(submissionTopicLinkPda);
    expect(linkAcc.submission.toString()).to.equal(submissionPda.toString());
    expect(linkAcc.topic.toString()).to.equal(topic1Pda.toString());
    expect(linkAcc.status.pending).to.not.be.undefined; // Check that status is Pending
    expect(linkAcc.yesVotingPower.toNumber()).to.equal(0);
    expect(linkAcc.noVotingPower.toNumber()).to.equal(0);
    expect(linkAcc.totalCommittedVotes.toNumber()).to.equal(0);
    expect(linkAcc.totalRevealedVotes.toNumber()).to.equal(0);
    
    // Verify that tempAlign tokens were minted to the contributor
    const contributorTempAlignAccount = await getAccount(
      provider.connection,
      contributorTempAlignAta
    );
    expect(Number(contributorTempAlignAccount.amount)).to.equal(100); // Should match tokensToMint = 100
    
    // Verify that the submission count was incremented in state and topic
    const stateAcc = await program.account.state.fetch(statePda);
    expect(stateAcc.submissionCount.toNumber()).to.equal(1);
    
    const topicAcc = await program.account.topic.fetch(topic1Pda);
    expect(topicAcc.submissionCount.toNumber()).to.equal(1);
    
    // Verify the contributor's topic-specific token balance was updated
    const contributorProfile = await program.account.userProfile.fetch(contributorProfilePda);
    const topicTokenEntry = contributorProfile.topicTokens.find(
      (pair) => pair.topicId.toNumber() === 0 // Topic ID 0
    );
    expect(topicTokenEntry).to.not.be.undefined;
    
    // Now that we've already checked that topicTokenEntry exists
    expect(topicTokenEntry.topicId.toNumber()).to.equal(0);
    expect(topicTokenEntry.token.tempAlignAmount.toNumber()).to.equal(100);
    expect(topicTokenEntry.token.tempRepAmount.toNumber()).to.equal(0);
  });

  // ========== TEST SECTION 5: CROSS-TOPIC LINKING ==========

  it("Links the submission to the second topic", async () => {
    // Derive the cross-topic link PDA
    [crossTopicLinkPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("submission_topic_link"), submissionPda.toBuffer(), topic2Pda.toBuffer()],
      program.programId
    );
    
    // Link the submission to the second topic
    const tx = await program.methods
      .linkSubmissionToTopic()
      .accounts({
        state: statePda,
        topic: topic2Pda,
        submission: submissionPda,
        submissionTopicLink: crossTopicLinkPda,
        authority: authorityKeypair.publicKey,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authorityKeypair])
      .rpc();
    
    console.log("Cross-topic linking transaction signature:", tx);
    
    // Verify the cross-topic link was created correctly
    const linkAcc = await program.account.submissionTopicLink.fetch(crossTopicLinkPda);
    expect(linkAcc.submission.toString()).to.equal(submissionPda.toString());
    expect(linkAcc.topic.toString()).to.equal(topic2Pda.toString());
    expect(linkAcc.status.pending).to.not.be.undefined; // Check that status is Pending
    expect(linkAcc.yesVotingPower.toNumber()).to.equal(0);
    expect(linkAcc.noVotingPower.toNumber()).to.equal(0);
    expect(linkAcc.totalCommittedVotes.toNumber()).to.equal(0);
    expect(linkAcc.totalRevealedVotes.toNumber()).to.equal(0);
    
    // Verify that the topic's submission count was incremented
    const topicAcc = await program.account.topic.fetch(topic2Pda);
    expect(topicAcc.submissionCount.toNumber()).to.equal(1);
    
    // Verify the state's submission count did NOT change (still 1)
    const stateAcc = await program.account.state.fetch(statePda);
    expect(stateAcc.submissionCount.toNumber()).to.equal(1);
  });

  // ========== TEST SECTION 6: STAKING ==========

  it("Stakes tempAlign tokens for tempRep tokens", async () => {
    // First, let's create a tempRep token account for the contributor
    let contributorTempRepAta = await getAssociatedTokenAddress(
      tempRepMintPda,
      contributorKeypair.publicKey
    );
    
    // Create ATA for contributor's tempRep
    let creationTx = await program.methods
      .createUserAta()
      .accounts({
        state: statePda,
        payer: authorityKeypair.publicKey,
        user: contributorKeypair.publicKey,
        mint: tempRepMintPda,
        userAta: contributorTempRepAta,
        systemProgram: web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: anchor.web3.ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authorityKeypair, contributorKeypair])
      .rpc();
    
    console.log("Create contributorTempRepAta transaction signature:", creationTx);
    
    // Define the staking amount - stake half of the earned tempAlign tokens
    const stakeAmount = 50;
    
    // Stake topic-specific tokens for the contributor
    const tx = await program.methods
      .stakeTopicSpecificTokens(new anchor.BN(stakeAmount))
      .accounts({
        state: statePda,
        topic: topic1Pda,
        userProfile: contributorProfilePda,
        tempAlignMint: tempAlignMintPda,
        tempRepMint: tempRepMintPda,
        userTempAlignAta: contributorTempAlignAta,
        userTempRepAta: contributorTempRepAta,
        user: contributorKeypair.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([contributorKeypair])
      .rpc();
    
    console.log("Stake tokens transaction signature:", tx);
    
    // Verify that tempAlign tokens were burned
    const tempAlignAccount = await getAccount(
      provider.connection,
      contributorTempAlignAta
    );
    expect(Number(tempAlignAccount.amount)).to.equal(100 - stakeAmount); // 50 burned
    
    // Verify that tempRep tokens were minted
    const tempRepAccount = await getAccount(
      provider.connection,
      contributorTempRepAta
    );
    expect(Number(tempRepAccount.amount)).to.equal(stakeAmount); // 50 minted
    
    // Verify that the user profile's topic-specific token balances were updated
    const contributorProfile = await program.account.userProfile.fetch(contributorProfilePda);
    const topicTokenEntry = contributorProfile.topicTokens.find(
      (pair) => pair.topicId.toNumber() === 0 // Topic ID 0
    );
    expect(topicTokenEntry).to.not.be.undefined;
    
    // Now that we've already checked that topicTokenEntry exists
    expect(topicTokenEntry.topicId.toNumber()).to.equal(0);
    expect(topicTokenEntry.token.tempAlignAmount.toNumber()).to.equal(100 - stakeAmount); // 50 remaining
    expect(topicTokenEntry.token.tempRepAmount.toNumber()).to.equal(stakeAmount); // 50 earned
    
    // Now, have the validator also submit data to get tempAlign tokens
    // First, create tempAlign ATA for validator
    let validatorTempAlignAta = await getAssociatedTokenAddress(
      tempAlignMintPda,
      validatorKeypair.publicKey
    );
    
    // Create ATA for validator's tempAlign tokens if it doesn't exist yet
    try {
      await getAccount(provider.connection, validatorTempAlignAta);
    } catch (e) {
      creationTx = await program.methods
        .createUserAta()
        .accounts({
          state: statePda,
          payer: authorityKeypair.publicKey,
          user: validatorKeypair.publicKey,
          mint: tempAlignMintPda,
          userAta: validatorTempAlignAta,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: anchor.web3.ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([authorityKeypair, validatorKeypair])
        .rpc();
      
      console.log("Create validatorTempAlignAta transaction signature:", creationTx);
    }
    
    // Derive a new submission PDA for validator submission (using submission count = 1)
    const [validatorSubmissionPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("submission"), Buffer.from([1, 0, 0, 0, 0, 0, 0, 0])],
      program.programId
    );
    
    // Derive a new submission-topic link PDA for validator submission
    const [validatorSubmissionTopicLinkPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("submission_topic_link"), validatorSubmissionPda.toBuffer(), topic1Pda.toBuffer()],
      program.programId
    );
    
    // Have validator submit data to earn tokens
    const validatorSubmissionTx = await program.methods
      .submitDataToTopic("validator-test-submission")
      .accounts({
        state: statePda,
        topic: topic1Pda,
        tempAlignMint: tempAlignMintPda,
        contributorAta: validatorTempAlignAta,
        submission: validatorSubmissionPda,
        submissionTopicLink: validatorSubmissionTopicLinkPda,
        contributorProfile: validatorProfilePda,
        contributor: validatorKeypair.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([validatorKeypair])
      .rpc();
    
    console.log("Validator submission transaction signature:", validatorSubmissionTx);
    
    // Verify validator received tempAlign tokens
    const validatorTempAlignAccount = await getAccount(
      provider.connection,
      validatorTempAlignAta
    );
    expect(Number(validatorTempAlignAccount.amount)).to.equal(100); // tokens_to_mint value
    
    // Now stake validator's tempAlign for tempRep so they can vote
    // Define stake amount for validator
    const validatorStakeAmount = 50;
    
    // Create ATA for validator's tempRep tokens if needed
    try {
      await getAccount(provider.connection, validatorTempRepAta);
    } catch (e) {
      creationTx = await program.methods
        .createUserAta()
        .accounts({
          state: statePda,
          payer: authorityKeypair.publicKey,
          user: validatorKeypair.publicKey,
          mint: tempRepMintPda,
          userAta: validatorTempRepAta,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: anchor.web3.ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([authorityKeypair, validatorKeypair])
        .rpc();
      
      console.log("Create validatorTempRepAta transaction signature:", creationTx);
    }
    
    // Stake validator's tokens
    const validatorStakeTx = await program.methods
      .stakeTopicSpecificTokens(new anchor.BN(validatorStakeAmount))
      .accounts({
        state: statePda,
        topic: topic1Pda,
        userProfile: validatorProfilePda,
        tempAlignMint: tempAlignMintPda,
        tempRepMint: tempRepMintPda,
        userTempAlignAta: validatorTempAlignAta,
        userTempRepAta: validatorTempRepAta,
        user: validatorKeypair.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([validatorKeypair])
      .rpc();
    
    console.log("Validator stake transaction signature:", validatorStakeTx);
    
    // Verify validator's tempAlign tokens were burned and tempRep tokens were minted
    const updatedValidatorTempAlignAccount = await getAccount(
      provider.connection,
      validatorTempAlignAta
    );
    expect(Number(updatedValidatorTempAlignAccount.amount)).to.equal(100 - validatorStakeAmount);
    
    const validatorTempRepAccount = await getAccount(
      provider.connection,
      validatorTempRepAta
    );
    expect(Number(validatorTempRepAccount.amount)).to.equal(validatorStakeAmount);
    
    // Verify validator's user profile was updated with the topic tokens
    const validatorProfile = await program.account.userProfile.fetch(validatorProfilePda);
    const validatorTopicTokenEntry = validatorProfile.topicTokens.find(
      (pair) => pair.topicId.toNumber() === 0 // Topic ID 0
    );
    expect(validatorTopicTokenEntry).to.not.be.undefined;
    expect(validatorTopicTokenEntry.topicId.toNumber()).to.equal(0);
    expect(validatorTopicTokenEntry.token.tempAlignAmount.toNumber()).to.equal(100 - validatorStakeAmount);
    expect(validatorTopicTokenEntry.token.tempRepAmount.toNumber()).to.equal(validatorStakeAmount);
  });

  // ========== TEST SECTION 7: VOTING ==========

  it("Commits a vote on the submission", async () => {
    // Calculate vote hash from vote choice, nonce, validator and submission-topic link
    const message = Buffer.concat([
      validatorKeypair.publicKey.toBuffer(),
      submissionTopicLinkPda.toBuffer(),
      Buffer.from([0]), // Yes vote is 0
      Buffer.from(VOTE_NONCE),
    ]);
    
    const voteHashArray = Array.from(
      await crypto.subtle.digest("SHA-256", message).then(b => new Uint8Array(b))
    );
    voteHash = voteHashArray;
    
    // Derive the vote commit PDA
    [voteCommitPda] = web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("vote_commit"),
        submissionTopicLinkPda.toBuffer(),
        validatorKeypair.publicKey.toBuffer(),
      ],
      program.programId
    );
    
    // Define vote amount
    const voteAmount = 25; // Half of the staked tempRep from earlier
    const isPermanentRep = false; // Use temporary reputation
    
    // Commit the vote
    const tx = await program.methods
      .commitVote(
        voteHash,
        new anchor.BN(voteAmount),
        isPermanentRep
      )
      .accounts({
        state: statePda,
        submissionTopicLink: submissionTopicLinkPda,
        topic: topic1Pda,
        submission: submissionPda,
        voteCommit: voteCommitPda,
        userProfile: validatorProfilePda,
        validator: validatorKeypair.publicKey,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([validatorKeypair])
      .rpc();
    
    console.log("Vote commit transaction signature:", tx);
    
    // Verify the vote commit was created correctly
    const voteCommitAcc = await program.account.voteCommit.fetch(voteCommitPda);
    expect(voteCommitAcc.submissionTopicLink.toString()).to.equal(submissionTopicLinkPda.toString());
    expect(voteCommitAcc.validator.toString()).to.equal(validatorKeypair.publicKey.toString());
    
    // Compare vote hash
    const fetchedHashArray = Array.from(voteCommitAcc.voteHash);
    expect(fetchedHashArray).to.deep.equal(voteHash);
    
    expect(voteCommitAcc.revealed).to.be.false;
    expect(voteCommitAcc.finalized).to.be.false;
    expect(voteCommitAcc.voteChoice).to.be.null;
    expect(voteCommitAcc.voteAmount.toNumber()).to.equal(voteAmount);
    expect(voteCommitAcc.isPermanentRep).to.equal(isPermanentRep);
    
    // Verify the submission-topic link vote count was incremented
    const linkAcc = await program.account.submissionTopicLink.fetch(submissionTopicLinkPda);
    expect(linkAcc.totalCommittedVotes.toNumber()).to.equal(1);
    expect(linkAcc.totalRevealedVotes.toNumber()).to.equal(0);
  });

  it("Reveals the committed vote", async () => {
    // Reveal the vote with the same choice and nonce used in the commit
    const tx = await program.methods
      .revealVote(
        VOTE_CHOICE_YES, // The Yes vote choice
        VOTE_NONCE       // The nonce used in the commit
      )
      .accounts({
        state: statePda,
        submissionTopicLink: submissionTopicLinkPda,
        topic: topic1Pda,
        submission: submissionPda,
        voteCommit: voteCommitPda,
        userProfile: validatorProfilePda,
        validator: validatorKeypair.publicKey,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([validatorKeypair])
      .rpc();
    
    console.log("Vote reveal transaction signature:", tx);
    
    // Verify the vote commit was updated correctly
    const voteCommitAcc = await program.account.voteCommit.fetch(voteCommitPda);
    expect(voteCommitAcc.revealed).to.be.true;
    expect(voteCommitAcc.voteChoice).to.not.be.null;
    expect(voteCommitAcc.voteChoice.yes).to.not.be.undefined;
    
    // Verify the submission-topic link vote counts were updated
    const linkAcc = await program.account.submissionTopicLink.fetch(submissionTopicLinkPda);
    expect(linkAcc.totalRevealedVotes.toNumber()).to.equal(1);
    
    // The vote amount was 25, and the quadratic voting power is sqrt(25) = 5
    const expectedVotingPower = 5;
    expect(linkAcc.yesVotingPower.toNumber()).to.equal(expectedVotingPower);
    expect(linkAcc.noVotingPower.toNumber()).to.equal(0);
  });

  // ========== TEST SECTION 8: FINALIZATION ==========

  it("Finalizes the submission", async () => {
    // In a real scenario, we would need to wait for the reveal phase to end
    // For testing, we'll forcibly set the timestamps in the program to be short durations
    
    // For this test, we'll simply assume we're past the reveal phase end
    // In a real scenario, we'd need to wait for the actual phase end
    console.log("Note: In a production environment, we would need to wait for the reveal phase to end");
    
    // Finalize the submission
    const tx = await program.methods
      .finalizeSubmission()
      .accounts({
        state: statePda,
        submissionTopicLink: submissionTopicLinkPda,
        topic: topic1Pda,
        submission: submissionPda,
        contributorProfile: contributorProfilePda,
        contributorTempAlignAta: contributorTempAlignAta,
        contributorAlignAta: contributorAlignAta,
        tempAlignMint: tempAlignMintPda,
        alignMint: alignMintPda,
        authority: authorityKeypair.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([authorityKeypair])
      .rpc();
    
    console.log("Finalize submission transaction signature:", tx);
    
    // Verify the submission-topic link status changed to Accepted
    const linkAcc = await program.account.submissionTopicLink.fetch(submissionTopicLinkPda);
    expect(linkAcc.status.accepted).to.not.be.undefined;
    
    // Verify that tempAlign tokens were converted to Align tokens
    const tempAlignAccount = await getAccount(
      provider.connection,
      contributorTempAlignAta
    );
    const alignAccount = await getAccount(
      provider.connection,
      contributorAlignAta
    );
    
    // The tokens_to_mint is 100, we've already burned 50 for staking, so there's 50 left
    // All 50 remaining should have been burned and converted to Align
    expect(Number(tempAlignAccount.amount)).to.equal(0);
    expect(Number(alignAccount.amount)).to.equal(50);
    
    // Verify the contributor's topic-specific token balances were updated
    const contributorProfile = await program.account.userProfile.fetch(contributorProfilePda);
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
      provider.connection,
      validatorRepAta
    ).catch(() => null);
    
    if (!validatorRep) {
      const tx = await program.methods
        .createUserAta()
        .accounts({
          state: statePda,
          payer: authorityKeypair.publicKey,
          user: validatorKeypair.publicKey,
          mint: repMintPda,
          userAta: validatorRepAta,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: anchor.web3.ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([authorityKeypair, validatorKeypair])
        .rpc();
      
      console.log("Create validatorRepAta transaction signature:", tx);
    }
    
    // Finalize the vote
    const tx = await program.methods
      .finalizeVote()
      .accounts({
        state: statePda,
        submissionTopicLink: submissionTopicLinkPda,
        topic: topic1Pda,
        submission: submissionPda,
        voteCommit: voteCommitPda,
        validatorProfile: validatorProfilePda,
        validatorTempRepAta: validatorTempRepAta,
        validatorRepAta: validatorRepAta,
        tempRepMint: tempRepMintPda,
        repMint: repMintPda,
        authority: authorityKeypair.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([authorityKeypair])
      .rpc();
    
    console.log("Finalize vote transaction signature:", tx);
    
    // Verify the vote was finalized
    const voteCommitAcc = await program.account.voteCommit.fetch(voteCommitPda);
    expect(voteCommitAcc.finalized).to.be.true;
    
    // Verify the validator's tempRep tokens were converted to permanent Rep
    const tempRepAccount = await getAccount(
      provider.connection,
      validatorTempRepAta
    );
    const repAccount = await getAccount(
      provider.connection,
      validatorRepAta
    );
    
    // Since we voted yes and the submission was accepted (vote with consensus),
    // 25 tempRep tokens should be converted to 25 permanent Rep tokens
    expect(Number(tempRepAccount.amount)).to.equal(0); // All converted
    expect(Number(repAccount.amount)).to.equal(25);
    
    // Verify that the validator's profile was updated
    const validatorProfile = await program.account.userProfile.fetch(validatorProfilePda);
    expect(validatorProfile.permanentRepAmount.toNumber()).to.equal(25);
    
    // Verify the validator's topic-specific token balances were updated
    const topicTokenEntry = validatorProfile.topicTokens.find(
      (pair) => pair.topicId.toNumber() === 0 // Topic ID 0
    );
    
    if (topicTokenEntry) {
      expect(topicTokenEntry.topicId.toNumber()).to.equal(0);
      expect(topicTokenEntry.token.tempRepAmount.toNumber()).to.equal(0); // All converted
    }
  });
});