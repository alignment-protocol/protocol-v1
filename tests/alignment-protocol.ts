import * as anchor from "@coral-xyz/anchor";
import { AnchorProvider, Program, web3 } from "@coral-xyz/anchor";
import { AlignmentProtocol } from "../target/types/alignment_protocol";

// Import test sections
import { runInitializationTests } from "./sections/01-initialization";
import { runTopicManagementTests } from "./sections/02-topic-management";
import { runUserSetupTests } from "./sections/03-user-setup";
import { runSubmissionTests } from "./sections/04-submission";
import { runCrossTopicLinkingTests } from "./sections/05-cross-topic-linking";
import { runStakingTests } from "./sections/06-staking";
import { runVotingTests } from "./sections/07-voting";
import { runFinalizationTests } from "./sections/08-finalization";
import { runTokenLockingTests } from "./sections/09-token-locking-tests";
import { runValidationTests } from "./sections/10-validation-tests";

// Import test context type
import { TestContext } from "./utils/test-setup";

describe("Alignment Protocol Tests", () => {
  // Configure Anchor provider
  const provider = AnchorProvider.env();
  anchor.setProvider(provider);

  // Set up program
  const program = anchor.workspace
    .AlignmentProtocol as Program<AlignmentProtocol>;

  // Get the authority keypair from the provider's wallet
  const authorityKeypair = (provider.wallet as any).payer;

  // Generate additional keypairs for tests
  const contributorKeypair = web3.Keypair.generate();
  const validatorKeypair = web3.Keypair.generate();
  const user3Keypair = web3.Keypair.generate(); // For additional testing
  const oracleKeypair = web3.Keypair.generate(); // <-- Generate Oracle Keypair

  // Create context object
  const ctx: TestContext = {
    provider,
    program,
    authorityKeypair,
    oracleKeypair, // <-- Add to context initialization
    contributorKeypair,
    validatorKeypair,
    user3Keypair,
    statePda: null,
    tempAlignMintPda: null,
    alignMintPda: null,
    tempRepMintPda: null,
    repMintPda: null,
    topic1Pda: null,
    topic2Pda: null,
    contributorTempAlignAccount: null,
    contributorTempRepAccount: null,
    validatorTempAlignAccount: null,
    validatorTempRepAccount: null,
    user3TempAlignAccount: null,
    user3TempRepAccount: null,
    contributorAlignAta: null,
    contributorRepAta: null,
    validatorAlignAta: null,
    validatorRepAta: null,
    user3AlignAta: null,
    user3RepAta: null,
    contributorProfilePda: null,
    validatorProfilePda: null,
    user3ProfilePda: null,
    // --- Initialize all other fields as before ---
    contributorTopic1BalancePda: null,
    validatorTopic1BalancePda: null,
    user3Topic1BalancePda: null,
    submissionPda: null,
    validatorSubmissionPda: null,
    testSubmissionPda: null,
    user3SubmissionPda: null,
    validationSubmissionPda: null,
    submissionTopicLinkPda: null,
    validatorSubmissionTopicLinkPda: null,
    testSubmissionTopicLinkPda: null,
    user3SubmissionTopicLinkPda: null,
    validationSubmissionTopicLinkPda: null,
    crossTopicLinkPda: null,
    voteCommitPda: null,
    testVoteCommitPda: null,
    user3VoteCommitPda: null,
    validationVoteCommitPda: null,
    voteHash: [],
    testVoteHash: [],
    user3VoteHash: [],
    validationVoteHash: [],
    TOPIC1_NAME: "AI Safety",
    TOPIC1_DESCRIPTION:
      "Alignment, interpretability, and safety research for AI systems",
    TOPIC2_NAME: "Climate",
    TOPIC2_DESCRIPTION: "Climate change mitigation and adaptation strategies",
    SUBMISSION_DATA: "ipfs://QmULkt3mMt5K8XHnYYxmnvtUGZ4p1qGQgvTKYwXkUxBcmx",
    VOTE_NONCE: "my-secret-nonce-123",
    VOTE_CHOICE_YES: { yes: {} },
    VOTE_CHOICE_NO: { no: {} },
    VOTE_NONCE_VALIDATION: "nonce-for-validation-vote",
  };

  // Setup test accounts and PDAs
  before("Fund test accounts with SOL", async () => {
    console.log("Funding test accounts with SOL...");
    // Fund each test account with 1 SOL
    const lamports = 1 * web3.LAMPORTS_PER_SOL;

    // Build and send transactions for funding
    for (const keypair of [
      ctx.contributorKeypair,
      ctx.validatorKeypair,
      ctx.user3Keypair,
      ctx.oracleKeypair, // <-- Add oracleKeypair to the funding loop
    ]) {
      const tx = new web3.Transaction().add(
        web3.SystemProgram.transfer({
          fromPubkey: ctx.authorityKeypair.publicKey,
          toPubkey: keypair.publicKey,
          lamports,
        }),
      );
      // Sign with authority only
      await ctx.provider.sendAndConfirm(tx, [ctx.authorityKeypair]);
    }

    console.log("Authority:", ctx.authorityKeypair.publicKey.toBase58());
    console.log("Oracle:", ctx.oracleKeypair.publicKey.toBase58()); // <-- Log oracle pubkey
    console.log("Contributor:", ctx.contributorKeypair.publicKey.toBase58());
    console.log("Validator:", ctx.validatorKeypair.publicKey.toBase58());
    console.log("User3:", ctx.user3Keypair.publicKey.toBase58());
  });

  before("Derive program PDAs", () => {
    // State PDA
    [ctx.statePda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("state")],
      ctx.program.programId,
    );

    // Token mint PDAs
    [ctx.tempAlignMintPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("temp_align_mint")],
      ctx.program.programId,
    );

    [ctx.alignMintPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("align_mint")],
      ctx.program.programId,
    );

    [ctx.tempRepMintPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("temp_rep_mint")],
      ctx.program.programId,
    );

    [ctx.repMintPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("rep_mint")],
      ctx.program.programId,
    );

    // User profile PDAs
    [ctx.contributorProfilePda] = web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("user_profile"),
        ctx.contributorKeypair.publicKey.toBuffer(),
      ],
      ctx.program.programId,
    );

    [ctx.validatorProfilePda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("user_profile"), ctx.validatorKeypair.publicKey.toBuffer()],
      ctx.program.programId,
    );

    [ctx.user3ProfilePda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("user_profile"), ctx.user3Keypair.publicKey.toBuffer()],
      ctx.program.programId,
    );
  });

  // Run all test sections in sequence
  runInitializationTests(ctx);
  runTopicManagementTests(ctx);
  runUserSetupTests(ctx);
  runSubmissionTests(ctx);
  runCrossTopicLinkingTests(ctx);
  runStakingTests(ctx);
  runVotingTests(ctx);
  runFinalizationTests(ctx);
  runTokenLockingTests(ctx);
  runValidationTests(ctx);
});
