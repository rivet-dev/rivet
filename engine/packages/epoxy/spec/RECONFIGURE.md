# EPaxos Reconfiguration Specification

This specification describes the reconfiguration process for EPaxos, specifically how new nodes join the cluster.

## Process

**Coordinator checks for configuration changes:**

1: Read current topology from rivet_config
2: Compare with existing cluster state to find new replicas
3: if new replicas found then
4:    Perform health checks on all new replicas
9:    Add new replicas to cluster config with status=Joining
10:   Send BeginLearningRequest to each new replica with current cluster config

**New replica R on receiving BeginLearningSignal from coordinator:**

14: for each existing replica E in config (excluding self) do
15:    Download log entries from replica E
16:    for each log entry in downloaded chunk do
17:       Replay log entry (without committing to KV)
18: Recover key-value state from replayed log entries
19:    for each unique key in log entries do
20:       Find all committed instances that touched this key
21:       Determine final value using highest sequence leaf entry
22:       Commit recovered value to KV store
23: Send CoordinatorUpdateReplicaStatusRequest(status=Active) to coordinator

**Coordinator on receiving replica status update to Active:**

24: Update replica status in cluster config
25: if status changed from non-Active to Active then
26:    increment epoch
27:    Send UpdateConfigRequest(new_config) to all replicas

**Any replica R on receiving UpdateConfigRequest(new_config):**

28: Update local cluster configuration to new_config
29: Recognize new active members in cluster

