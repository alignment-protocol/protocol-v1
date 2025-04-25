# Vedas MVP – High-Impact Implementation Guide

This document distils the **hundreds of checklist lines** into a **single
action-oriented roadmap** focused on the work that directly affects the “Web-2
feel” of Vedas. Finish these items (roughly in order) and the rest of the
tasks can follow without blocking the user experience.

## Legend

🔴 = Critical blocker 🟠 = Important 🟢 = Nice-to-have

---

0.  Bird’s-eye Timeline

---

|  Week | Milestone                                                                                    |
| ----: | -------------------------------------------------------------------------------------------- |
| **0** | Meta-tx helper merged in Alignment Protocol + backend signer lib skeleton                    |
| **1** | submit / commit / reveal endpoints live – UI can run the **core loop** without extra pop-ups |
| **2** | Light indexer (Helius webhook or custom listener) fills Supabase – UI shows _fresh_ data     |
| **3** | Auto-onboard (profile + faucet) + keeper bot for finalisation                                |
| **4** | Smoke E2E on devnet, push to staging                                                         |

---

1. Meta-Transaction Plumbing (🔴 BLocker)

---

### 1.1 On-chain work (Alignment Protocol)

Task list

| Status | Task                                                                                                                                                                                                                     |
| :----: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
|   ❌   | Add `verify_meta_tx()` helper – parse `instructions_sysvar`, check ed25519 ix, domain separator `"ALIGN_META_TX"`, recent_blockhash + nonce                                                                              |
|   ❌   | Create `Nonce` PDA: seeds `[b"meta_nonce", user]`, data = `u64 counter, bump`                                                                                                                                            |
|   ❌   | Patch **Category B** instructions (`submit_data_to_topic`, `stake_topic_specific_tokens`, `commit_vote`, `reveal_vote`, `request_ai_validation`) to accept `{ user_pk, payload_nonce }` args and call `verify_meta_tx()` |
|   ❌   | Bump CU budget (~25k) to cover one ed25519 verify                                                                                                                                                                        |

### 1.2 Backend relayer library

- Expose `buildMetaTx(programId, ix, sessionKey?, signerKeypair)` ⇒ returns `Transaction`:
  1. ed25519_program::verify
  2. protocol instruction with meta args
- Store `nonce` in Redis so multiple frontend pods don’t race.
- For session-key mode:
  – On `/api/onboard` have user sign **once** → save `{session_pubkey, expires_at}` in Supabase.

### 1.3 Frontend integration

- At wallet connect, call `/api/onboard` → receive `session_pubkey` & expiry.
- Subsequent writes hit `/api/tx` which returns a signature only – no wallet pop-up.

---

2. Fresh Data Path (🔴 Blocker)

---

### 2.1 Emit program events

- `TopicCreated`, `SubmissionLinked`, `VoteCommitted`, `VoteRevealed`, `SubmissionFinalized`, `VoteFinalized`
- Keep payloads ≤ 1 kB (log limit 1,024 bytes).

### 2.2 Indexer

| Status | Task                                                                                            |
| :----: | ----------------------------------------------------------------------------------------------- |
|   ❌   | Use Helius webhook or custom WebSocket listener to capture logs where `program == ALIGNMENT_ID` |
|   ❌   | Decode events → upsert rows in Supabase tables `stories`, `proposals`, `votes`                  |
|   ❌   | Back-fill on start-up with `getProgramAccounts` + memcmp if missed slots                        |

### 2.3 Backend read layer

- API endpoints simply `SELECT` from Supabase – no RPC latency.

---

3. Write-Path Wrappers (🔴)

---

| Endpoint                 | Alignment instruction  | Notes                                                                 |
| ------------------------ | ---------------------- | --------------------------------------------------------------------- |
| POST `/api/proposals`    | `submit_data_to_topic` | FormData: `storyId`, `text` → off-chain store snippet, meta-tx submit |
| POST `/api/votes/commit` | `commit_vote`          | Inputs: `proposalId`, `hash`, `nonce`                                 |
| POST `/api/votes/reveal` | `reveal_vote`          | Inputs: `proposalId`, `choice`, `nonce`                               |

All three MUST: (1) ensure user profile exists, (2) use relayer lib, (3) return `{txSig}` for toast.

---

4. Auto-Onboard & Faucet (🟠)

---

- Endpoint `/api/onboard` steps:
  1. Create `UserProfile`, `temp` token accounts via signature-free ixs.
  2. Airdrop tiny SOL if needed (devnet), mint starter `tempAlign` & `tempRep` to user’s protocol accounts.
  3. Return `session_pubkey` (see 1.3).

---

5. Finalisation Keeper (🟠)

---

- Cron every N seconds → query Supabase for `submissions` with `reveal_end < now AND status == Pending`.
- Send meta-tx `finalize_submission` (+ `finalize_vote` loop for each commit).
- Collect optional `finalization_reward` (to be added to `State`).

---

6. CLI / Dev Utilities (🟢)

---

- Fix `alignment vote finalize --voter <pk>`
- Script `./scripts/devnet-reset.sh` → resets local validator, seeds faucet, starts keeper & indexer.

---

## Appendix A – Meta-Tx Payload Format

```
sha256(
  "ALIGN_META_TX" ||               // 12 bytes domain separator
  instruction_discriminator[8] ||   // first 8 bytes of Anchor idl ix hash
  serialized_ix_args ||             // borsh-encoded
  nonce:u64 ||
  recent_blockhash[32]
)
```

Nonce is stored/checked in `["meta_nonce", user]` PDA.

---

## Appendix B – Example Event Schema (JSON)

```
// TopicCreated
{
  "topic": "Pubkey",  "creator": "Pubkey",  "name": "string"
}

// SubmissionFinalized
{
  "link": "Pubkey",  "status": "Accepted|Rejected",
  "yes_power": "u64",  "no_power": "u64"
}
```

---

## Appendix C – Supabase Table Hints

```sql
-- stories
id uuid primary key,
topic_pubkey text unique not null,
title text,
creator text,
created_at timestamptz default now()

-- proposals
id uuid primary key,
story_id uuid references stories(id),
submission_pubkey text unique,
author text,
text text,
status text check (status in ('Pending','Accepted','Rejected')),
created_at timestamptz default now()

-- votes
proposal_id uuid references proposals(id),
voter text,
choice text,
revealed bool,
finalized bool,
primary key (proposal_id, voter)
```

---

End of file
