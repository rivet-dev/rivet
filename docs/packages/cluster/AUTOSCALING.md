# Autoscaling

The autoscaler service runs every 15 seconds.

## Why memory?

The autoscaler uses CPU usage for GG nodes and memory usage for job nodes. This is because certain cloud providers like linode do not provide an actual value for the speed of the CPU, but rather the amount of cores. This is problematic because we use Nomad's API for determining the usage on any given node, and it returns its stats in MHz.

## Hardware failover

Before a job server provisioned, we don't know for sure what its specs will be because of the hardware failover system in `cluster-server-provision`. In the autoscaling process, all servers that aren't provisioned yet are assumed to have the specs of the first hardware option in the list.

### Failover has lower specs

In the event that the hardware which ended up being provisioned has lower specs than the first hardware in the list, the autoscaler will calculate the error between how much was expected and how much was actually provisioned. This error number corresponds to how many more servers might be needed to reach the desired server count.

Here is an example of the process in action:

| time since start | desired count | expected total memory | actual total memory |
| ---------------- | ------------- | --------------------- | ------------------- |
| 0s               | 2             | 2000MB                | 0MB                 |

We start with 0 servers provisioned, and 2 desired. Our config consists of two hardwares, the first having 1000MB of memory and the second having 500MB of memory. With our failover system if the first one fails to provision, the second will be provisioned.

| time since start | desired count | expected total memory | actual total memory |
| ---------------- | ------------- | --------------------- | ------------------- |
| 0s               | 2             | 2000MB                | 0MB                 |
| 15s              | 3             | 2000MB                | 1000MB              |

After the first iteration, the autoscaler provisioned 2 servers which both ended up failing over and only providing a total of 1000MB of memory. The autoscaler then proceeds to calculate the error like so:

```rust
ceil(expected - actual) / expected_memory_per_server)

ceil((2000 - 1000) / 1000) = 1
```

So an extra server was added to the desired count.

Now, if the next server to be provisioned ends up having 1000MB like it should, we will end up having the original amount of desired memory.

| time since start | desired count | expected total memory | actual total memory |
| ---------------- | ------------- | --------------------- | ------------------- |
| 0s               | 2             | 2000MB                | 0MB                 |
| 15s              | 3             | 2000MB                | 1000MB              |
| 30s              | 3             | 2000MB                | 2000MB              |

The error calculation would now be:

```rust
ceil((3000 - 2000) / 1000) = 1
```

So the error count stays the same and we stay at 3 desired servers.

However, if the server provisioned was again a failover server, we would have this scenario:

| time since start | desired count | expected total memory | actual total memory |
| ---------------- | ------------- | --------------------- | ------------------- |
| 0s               | 2             | 2000MB                | 0MB                 |
| 15s              | 3             | 2000MB                | 1000MB              |
| 30s              | 4             | 2000MB                | 1500MB              |

We end up with two extra servers to provision atop our original 2.

```rust
ceil((3000 - 1500) / 1000) = 2
```

| time since start | desired count | expected total memory | actual total memory |
| ---------------- | ------------- | --------------------- | ------------------- |
| 0s               | 2             | 2000MB                | 0MB                 |
| 15s              | 3             | 2000MB                | 1000MB              |
| 30s              | 4             | 2000MB                | 1500MB              |
| 45s              | 4             | 2000MB                | 2000MB              |

And finally we reach the desired capacity.

### Failover has higher specs

In the event that the failover hardware has higher specs than the desired amount, there is no error system that reduces the desired count to account for this difference. This is because there is no direct correlation between desired count and the hardware being provisioned and destroyed. Thus, if hardware with higher than expected specs is provisioned, that extra space will not be taken into account.

If it was taken into account in a similar error system as failover with lower specs, it would look like this:

| time since start | desired count | expected total memory | actual total memory |
| ---------------- | ------------- | --------------------- | ------------------- |
| 0s               | 1             | 1000MB                | 2000MB              |

Error:

```rust
ceil(expected - actual) / expected_memory_per_server)

ceil((1000 - 2000) / 1000) = -1
```

The original desired count + error would be 0, destroying the only server and causing the capacity to drop to 0. If the higher-spec'd failover kept getting provisioned, this would end up in a loop.

## Job server autoscaling

The nomad topology for each job server in a datacenter is fetched and the memory is aggregated. This value is then divided by the expected memory capacity (the capacity of the first hardware in the config), which determines the minimum expected server count required to accommodate the current usage. Then, we add the error value (discussed above) and the margin value which is configured in the namespace config.

### Autoscaling via machine learning

Coming soon

## GG server autoscaling

Because we do not need to be preemptive with GG servers, the autoscaling is a bit more simple.

-   If the current CPU usage is more than 20% under the total, add a server.
-   If the current CPU usage is less than 130% under the total, remove a server.

Examples:

```rust
// 3 servers
total_cpu = 300
cpu_usage = 285

// result: add a server
```

```rust
// 1 server
total_cpu = 100
cpu_usage = 70

// result: do nothing
```

```rust
// 4 servers
total_cpu = 400
cpu_usage = 250

// result: remove a server
```
