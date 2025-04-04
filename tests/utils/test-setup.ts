import { Program, AnchorProvider, web3, BN } from "@coral-xyz/anchor";
import { AlignmentProtocol } from "../../target/types/alignment_protocol";

// Setup utility functions and shared variables
export interface TestContext {
  // Programs
  provider: AnchorProvider;
  program: Program<AlignmentProtocol>;

  // Keypairs
  authorityKeypair: web3.Keypair;
  contributorKeypair: web3.Keypair;
  validatorKeypair: web3.Keypair;
  user3Keypair: web3.Keypair;

  // PDAs
  statePda: web3.PublicKey;
  tempAlignMintPda: web3.PublicKey;
  alignMintPda: web3.PublicKey;
  tempRepMintPda: web3.PublicKey;
  repMintPda: web3.PublicKey;

  // Topic PDAs
  topic1Pda: web3.PublicKey;
  topic2Pda: web3.PublicKey;

  // User token accounts
  // Temporary Token PDAs
  contributorTempAlignAccount: web3.PublicKey;
  contributorTempRepAccount: web3.PublicKey;
  validatorTempAlignAccount: web3.PublicKey;
  validatorTempRepAccount: web3.PublicKey;
  user3TempAlignAccount: web3.PublicKey;
  user3TempRepAccount: web3.PublicKey;

  // Permanent Token ATAs
  contributorAlignAta: web3.PublicKey;
  contributorRepAta: web3.PublicKey;
  validatorAlignAta: web3.PublicKey;
  validatorRepAta: web3.PublicKey;
  user3AlignAta: web3.PublicKey;
  user3RepAta: web3.PublicKey;

  // User profiles
  contributorProfilePda: web3.PublicKey;
  validatorProfilePda: web3.PublicKey;
  user3ProfilePda: web3.PublicKey;

  // User Topic Balances
  contributorTopic1BalancePda: web3.PublicKey;
  validatorTopic1BalancePda: web3.PublicKey;

  // Submission tracking
  submissionPda: web3.PublicKey;
  validatorSubmissionPda: web3.PublicKey;
  submissionTopicLinkPda: web3.PublicKey;
  validatorSubmissionTopicLinkPda: web3.PublicKey;
  crossTopicLinkPda: web3.PublicKey;

  // Vote tracking
  voteCommitPda: web3.PublicKey;
  voteHash: number[];

  // Constants
  TOPIC1_NAME: string;
  TOPIC1_DESCRIPTION: string;
  TOPIC2_NAME: string;
  TOPIC2_DESCRIPTION: string;
  SUBMISSION_DATA: string;
  VOTE_NONCE: string;
  VOTE_CHOICE_YES: any;
}
