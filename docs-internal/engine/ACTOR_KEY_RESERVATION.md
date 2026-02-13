# Actor Key Reservation

Epoxy is used to globally store which datacenter an actor lives in for a given actor ID.

## Schema

In Epoxy KV store: `Actor key -> reservation ID`

The reservation ID includes the datacenter of where the actor lives.

## Consistency Limitations

Actors have 2 touch points with Epoxy:

- Reserve actor key (epaxos fast path = 1 RTT, epaxos slow path = 2 RTT)
- Resolve actor ID for key (fast path = 0 RTT, slow path = 1 RTT to nearest DC that has the key)

Resolving actor ID uses `kv_get_optimistic`, which assumes a value does not change after being set. This allows us to cache the actor's datacenter locally and never have to read from other nodes once we've resolved it once.

## Reservation ID

A reservation ID is a unique ID (which includes the datacenter label).

**Why not store the actor ID**

Actor IDs initially make sense to use since they include the datacenter ID that the actor lives in.

Actor IDs can be created and destroyed, but our consistency limitations above requires that we do not change the value after set. Therefore, because we cannot update keys to new actor IDs, we cannot store the actor ID.

**Why not store just the datacenter label**

A naiive approach would be to store just the datacenter label in epoxy where the actor lives.

See reservation chains below.

### Reservation Chains & Moving Reservation Datacenters (Future Work)

Eventually, actors need to be moved between datacenters OR destroyed then create in a different datacenter. 

To do this, we'll set up reservation ID chains. Reservation ID chains are used to resolve where a reservation was moved to. When attempting to resolve a reservation that was moved, that datacenter will return the pointer to the reservation that it was moved to.

Consider:

- Actor 1 created in DC A with key "foo"
	- Generates reservation ID X (in DC A)
	- Epoxy stores: foo = reservation X
- Actor 1 destroyed
- Actor 2 created in DC B with key "foo"
	- Generations reservation ID Y (in DC B)
	- DC A stores pointer from reservation X to reservation Y
	- Epoxy still has: foo = reservation X (does not get updated)
- Next request to key "foo":
	- Read reservation ID X (in DC A) from Epoxy
	- Client send request to DC A with reservation ID X
	- DC A returns pointer to reservation ID Y (in DC B)
		- Client caches this pointer
	- Client sends request to DC B with reservation ID Y
- If the client requests key "foo" again:
	- Read reservation ID X (in DC A) from Epoxy
	- Client reads cache for reservation X to reservation Y (in DC B)
	- Sends request to DC B

The resulting state is:

```
Epoxy:
- foo = reservation X (in DC A)

Pointer cache (DC A):
- Reservation X (in DC A) -> reservation Y (in DC B)
```

**Why not just use datacenter label**

Consider a chain of pointers between the following datacenters:

```
DC label: 1 -> 2 -> 1 -> 3
Pointer:    A    B    C
```

There is a race condition & possible infinite loop when:

- DC 2 has pointer to DC 1 (pointer B)
- DC 1 still has pointer to DC 2 (pointer A) (has not updated to DC 3, pointer C)

Using unique reservation IDs allow us to create IDs like `{dc}.{unique}`. The chain would look like:

```
Reservation ID: 1.1 -> 2.2 -> 1.3 -> 3.4
Pointer:            A      B      C
```

Now there cannot be a loop between DC 1 & 2 since clients will follow the chain of pointers.

_This section needs more elaboration on edge cases._

## Current Limitation: Keys are tied to a datacenter

Because we cannot change the value of the reservation ID, actors can only be created in the original for a given ID.

