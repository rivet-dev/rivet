# Keys

## Key Hierarchy

```
/rivet/epoxy/
	replica/{replica_id}/
		config = {Config}                            # Replica configuration with epoch and peer info
		instance_number = {u32}                      # Track instance number for next instance
		current_ballot = {Ballot}                    # Current ballot number for this replica
		instance_ballot/{replica}/{slot} = {Ballot}  # Highest ballot seen for each instance
		log/{replica}/{slot}/entry = {LogEntry}
		key_instance/{key}/{replica}/{slot} = ()     # Used for checking interference
		key_value/{key}/
			committed_value = Vec<u8>                # Value written on commit
			optimistic_cached_value = Vec<u8>        # Value read from remote datacenters in kv::get_optimistic
```

### Design Notes

- `key_instances/{key}` is stored separately from `keys/{key}` because we need to efficiently be able to do a range scan over key_instances in order to find interference, so we need to minimize keys in that subspace
- `instance_number` should be replaced with `version_stamp` to reduce contention

### Contention

- `lead_consensus`
	- `instance_number`
	- `current_ballot`
- `find_interference` & `update_log`
	- `key_instances/{key}/({replica,slot})`
- `validate_ballot_for_instance`
	- `instance_ballot/{replica}/{slot}`

