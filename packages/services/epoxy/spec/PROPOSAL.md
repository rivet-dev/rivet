# EPaxos Proposal Specification

Based on Figure 2 & 3.

## Phase 1: Establish ordering constraints

**Replica L on receiving Request(γ) from a client becomes the designated leader for command γ (steps 2, 3 and 4 executed atomically):**

1: increment instance number i_L ← i_L + 1
   {Interf_L,γ is the set of instances Q.j such that the command recorded in cmds_L[Q][j] interferes w/ γ}
   -> src/replica/messages/lead_consensus.rs
2: seq_γ ← 1+ max ({cmds_L[Q][j].seq | Q.j ∈ Interf_L,γ} ∪ {0})
   -> src/replica/messages/lead_consensus.rs
3: deps_γ ← Interf_L,γ
   -> src/replica/messages/lead_consensus.rs
4: cmds_L[L][i_L] ← (γ, seq_γ, deps_γ, pre-accepted)
   -> src/replica/messages/lead_consensus.rs
5: send PreAccept(γ, seq_γ, deps_γ, L.i_L) to all other replicas in F, where F is a fast quorum that includes L
   -> src/replica/propose.rs

**Any replica R, on receiving PreAccept(γ, seq_γ, deps_γ, L.i) (steps 6, 7 and 8 executed atomically):**

6: update seq_γ ← max({seq_γ} ∪ {1 + cmds_R[Q][j].seq | Q.j ∈ Interf_R,γ})
   -> src/replica/messages/pre_accept.rs
7: update deps_γ ← deps_γ ∪ Interf_R,γ
   -> src/replica/messages/pre_accept.rs
8: cmds_R[L][i] ← (γ, seq_γ, deps_γ, pre-accepted)
   -> src/replica/messages/pre_accept.rs
9: reply PreAcceptOK(γ, seq_γ, deps_γ, L.i) to L
   -> src/replica/messages/pre_accept.rs

**Replica L (command leader for γ), on receiving at least ⌊N/2⌋ PreAcceptOK responses:**

10: if received PreAcceptOK's from all replicas in F \ {L}, with seq_γ and deps_γ the same in all replies (for some fast quorum F) then
    -> src/replica/messages/decide_path.rs
11:    run Commit phase for (γ, seq_γ, deps_γ) at L.i
       -> src/replica/messages/decide_path.rs
12: else
    -> src/replica/messages/decide_path.rs
13:    update deps_γ ← Union(deps_γ from all replies)
       -> src/replica/messages/decide_path.rs
14:    update seq_γ ← max({seq_γ of all replies})
       -> src/replica/messages/decide_path.rs
15:    run Paxos-Accept phase for (γ, seq_γ, deps_γ) at L.i
       -> src/replica/messages/decide_path.rs

## Phase 2: Paxos-Accept

**Command leader L, for (γ, seq_γ, deps_γ) at instance L.i:**

16: cmds_L[L][i] ← (γ, seq_γ, deps_γ, accepted)
    -> src/replica/messages/accepted.rs
17: send Accept(γ, seq_γ, deps_γ, L.i) to at least ⌊N/2⌋ other replicas
    -> src/replica/propose.rs

**Any replica R, on receiving Accept(γ, seq_γ, deps_γ, L.i):**

18: cmds_R[L][i] ← (γ, seq_γ, deps_γ, accepted)
    -> src/replica/messages/accept.rs
19: reply AcceptOK(γ, L.i) to L
    -> src/replica/messages/accept.rs

**Command leader L, on receiving at least ⌊N/2⌋ AcceptOK's:**

20: run Commit phase for (γ, seq_γ, deps_γ) at L.i
    -> src/replica/propose.rs

## Commit

**Command leader L, for (γ, seq_γ, deps_γ) at instance L.i:**

21: cmds_L[L][i] ← (γ, seq_γ, deps_γ, committed)
    -> src/replica/messages/committed.rs
22: send commit notification for γ to client
    -> src/replica/propose.rs (via return value)
23: send Commit(γ, seq_γ, deps_γ, L.i) to all other replicas
    -> src/replica/propose.rs

**Any replica R, on receiving Commit(γ, seq_γ, deps_γ, L.i):**

24: cmds_R[L][i] ← (γ, seq_γ, deps_γ, committed)
    -> src/replica/messages/commit.rs

## Explicit Prepare

**Replica Q for instance L.i of potentially failed replica L**

25: increment ballot number to epoch.(b+1).Q, (where epoch.b.R is the highest ballot number Q is aware of in instance L.i)
    -> src/ops/explicit_prepare.rs
26: send Prepare(epoch.(b+1).Q, L.i) to all replicas (including self) and wait for at least ⌊N/2⌋ + 1 replies
    -> src/ops/explicit_prepare.rs
27: let R be the set of replies w/ the highest ballot number
    -> src/ops/explicit_prepare.rs
28: if R contains a (γ, seq_γ, deps_γ, committed) then
    -> src/ops/explicit_prepare.rs
29:    run Commit phase for (γ, seq_γ, deps_γ) at L.i
    -> src/ops/explicit_prepare.rs
30: else if R contains an (γ, seq_γ, deps_γ, accepted) then
    -> src/ops/explicit_prepare.rs
31:    run Paxos-Accept phase for (γ, seq_γ, deps_γ) at L.i
    -> src/ops/explicit_prepare.rs
32: else if R contains at least ⌊N/2⌋ identical replies (γ, seq_γ, deps_γ, pre-accepted) for the default ballot epoch.0.L of instance L.i, and none of those replies is from L then
    -> src/ops/explicit_prepare.rs
33:    run Paxos-Accept phase for (γ, seq_γ, deps_γ) at L.i
    -> src/ops/explicit_prepare.rs
34: else if R contains at least one (γ, seq_γ, deps_γ, pre-accepted) then
    -> src/ops/explicit_prepare.rs
35:    start Phase 1 (at line 2) for γ at L.i, avoid fast path
    -> src/ops/explicit_prepare.rs
36: else
    -> src/ops/explicit_prepare.rs
37:    start Phase 1 (at line 2) for no-op at L.i, avoid fast path
    -> src/ops/explicit_prepare.rs

**Replica R, on receiving Prepare(epoch.b.Q, L.i) from Q**

38: if epoch.b.Q is larger than the most recent ballot number epoch.x.Y accepted for instance L.i then
    -> src/replica/messages/prepare.rs
39:    reply PrepareOK(cmds_R[L][i], epoch.x.Y, L.i)
    -> src/replica/messages/prepare.rs
40: else
    -> src/replica/messages/prepare.rs
41:    reply NACK
    -> src/replica/messages/prepare.rs

