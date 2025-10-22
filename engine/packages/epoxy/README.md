# Epoxy

Epoxy is a geo-distributed, strongly-consistent KV store based on the EPaxos protocol.

## EPaxos Overview

### Resources

Paper:

https://www.cs.cmu.edu/~dga/papers/epaxos-sosp2013.pdf

Reference implementations:

- https://github.com/efficient/epaxos/tree/master/src/epaxos
- https://github.com/pisimulation/epaxos/tree/master

TODO: Correctness issue: https://github.com/otrack/on-epaxos-correctness

Talks:

- Original paper: https://www.youtube.com/watch?v=KxoWlUZNKn8&t=357s
- Optimization: https://www.youtube.com/watch?v=aysKrS_SAXo

Other resources:

- Atlas follow up: https://software.imdea.org/~gotsman/papers/atlas-eurosys20.pdf

### Glossary

- **Replica** — one of the datacenters running epaxos
- **Coordinator** — the workflow that manages adding and removing replicas from the cluster
- **Instance** — per-replica slot (R.1, R.2, …); at most one command is chosen per slot. **This is not the same as a replica.**
- **Command** — an operation to be ran on the KV store
- **Command leader** — the replica that received the client’s command (not a fixed global leader)
- **Interference (interf)** — two commands "interfere" if they conflict (i.e., same key)
- **Dependencies (deps)** — set of interfering instances that a command depends on.
- **Sequence number (seq)** — integer used (with deps) to break cycles and order execution.
- **Log** — each replica’s private log of instances and their attributes/status.
- **Ballot number** — monotonic proposal id; includes an epoch prefix to order configurations.
- **Epoch** — configuration/version used to order ballots across reconfigurations.

### Interference Overview

Interference is at the core of EPaxos. Interference is used to determine if a command can be committed on the fast path without executing the full slow-path Paxos protocol.

Some notes related to Rivet's workload requirements relating to interference:

- Any write to the same key is an interference, regardless of the operation kind
	- The only case where we could optimize this more is if commands are communicative (i.e. simila to UDB's atomic operations)
- Even though we use CAS for actor keys:
	- This counts as a write regardless of the previous value, since CAS commands are non-communicative
	- CAS get written to the log even though it may have no effect on the value
	- If there is no conflict, the fast-path will return a success since CAS is equivalent to a write
	- The CAS will be written to the log regardless, but we will determine if the write was a success when committed
- Consistent reads also count as interference, which is not good for read-heavy operations
	- Rivet is designed to ensure that values do not change after they are set so we can use optimistic reads instead (see dev-docs/ACTOR_KEY_RESERVATION.md)
	- This pattern is similar to the lease pattern that the paper proposes

## Workloads Depending On Epoxy

- `pegboard::workflows::actor::actor_keys::Propose`: Uses check or set operation for the actor key's reservation ID
- `pegboard::ops::get_reservation_for_key`: Uses an optimistic read to find the reservation ID to resolve the actual actor ID

## Architecture

### Coordinator Workflow

The leader datacenter holds a coordinator workflow. This workflow detect config changes and handle:

- Gracefully adding replicas
- Propagating cluster config changes
- Epoch number

See `spec/RECONFIGURE.md` for more information.

### Replcia Workflow

Each datacenter has its own replica workflow that is responsible for:

- Downloading instances from other replicas
- Notifying coordinator when joined cluster

### Messages

All peer-to-peer communication is done via the `POST /v{version}/epoxy/message` using the versioned BARE epoxy protocol.

### Proposals

Proposals are the mechanism for establishing consensus when making a chance to the KV store.

Proposals are not implemented as a workflow since they're:

- Inherently designed to be fallable so they don't benefit from the overhead of workflows
- They can operate in paralle, while a single replica workflow cannot — though a workflow-per-proposal would technically work fine

See `spec/PROPOSAL.md` for more information.

## Coordinator & Dynamic Topology Configuration

**Do not use `rivet_config::config::Topology` (i.e. `ctx.config().topology()`) from this service.** Instead, read the cluster config propagated from the coordinator with `epoxy::ops::read_cluster_config`.

The leader datacenter runs a coordinator workflow in charge of coordinating config changes across all peers.

This ensures that peers receive config changes at the exact same time.

## Joining Replicas

### Reconfigure Abort

The coordinator will attempt to propagate the config change to all new replicas when a config change is detected.

If the config is invalid (i.e. the replica cannot be reached), the workflow will keep retrying indefinitely.

Reconfigure will be aborted if there is a config change detected. In this case, all changes will be abandoned and the coordinator will attempt to reconfigure with the new config.

### Error in EPaxos Join Specification

The paper has an incorrect explanation of adding new replicas. It describes the join process as:

1. Broadcast _Join_ with new epoch
2. Download instances from other replicas
3. Replica begins voting

The issue with this is that the quorum count will have changed, but the replica cannot vote yet. This means that if you add too many replicas at the same time you will cause a complete outage until the new replicas have downloaded all instances and began voting.

Instead, we opt to:

1. Add replica as a learner (starts receiving commits)
2. Download instances from other replicas
3. Corodinator increments voting once node is active
4. Replica begins voting

This enables clusters to add many new replicas at once without causing an outage.

### Misc Notes

- Downloading replicas receive commits from other replicas even though they are not part of the quorum while they are downloading. This ensures that they have received all messages, even though they are not part of the quorum.

## Future Optimizations

## Integrate Explicit Prepare

Explicit Prepare is	implemented but not integrated anywhere.

### Avoiding Execution Livelock (§4.9)

The paper recommends prioritizing earlier commands. We don't have functionality to do this right now, but the current design has almost no contention so this is not a concern.

### Batching (§7.6)

The paper recommends batching requests for a "9x" improvement in throughput. We don't need to do this since we (a) execute requests in parallel, (b) assume low contention, and (c) built the system to be stateless + scale horizontally with UDB.

### Optimized Epaxos (§4.4)

The paper recommends an optimize recovery mechanism that requires less replicas for consensus.


