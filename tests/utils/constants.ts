import { web3 } from "@coral-xyz/anchor";

export const TOKEN_PROGRAM_ID = new web3.PublicKey(
  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
);
export const ASSOCIATED_TOKEN_PROGRAM_ID = new web3.PublicKey(
  "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
);
export const SYSTEM_PROGRAM_ID = web3.SystemProgram.programId;
export const RENT_SYSVAR_ID = web3.SYSVAR_RENT_PUBKEY;
