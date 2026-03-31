# Isolation Model Comparison

Comparing five isolation approaches from weakest to strongest security boundary, with focus on how each binds to the host kernel and what the actual trusted computing base is.

## Quick Reference

| | **Namespaces/Jailing** | **Docker (default)** | **agentOS/Secure-Exec** | **gVisor** | **Firecracker** |
|---|---|---|---|---|---|
| **Isolation primitive** | Kernel namespaces | Namespaces + seccomp + caps + LSM | Userspace TypeScript kernel | Userspace Go kernel (Sentry) | Hardware virtualization (VT-x/AMD-V) |
| **Host kernel shared?** | Yes (full) | Yes (filtered) | Yes (underneath Node.js) | Yes (minimal surface) | No (guest gets own kernel) |
| **Host syscalls reachable** | ~385 (all) | ~361 unconditional + ~65 conditional | All (via Node.js), jailing coming soon | 53-68 (seccomp-enforced) | ~40 (seccomp-enforced, from VMM only) |
| **Kernel exploit = host compromise?** | Yes | Yes | N/A (no guest kernel) | Only if in Sentry's 53-68 syscalls | No (only compromises guest) |
| **TCB size** | Host kernel | Host kernel + runc | V8 + Node.js + kernel code | Host kernel (53-68 paths) + Sentry (Go) | KVM + VMM (50K lines Rust) + Jailer |
| **Memory safety** | C (kernel) | C (kernel) | TypeScript/JS (GC'd) | Go (GC'd) | Rust (compile-time) |
| **Boot time** | Instant | ~50ms | Near-instant | 50-100ms | ~125ms |
| **Memory overhead** | Negligible | <10 MiB | Minimal | 10-50 MiB | <5 MiB |
| **Multi-tenant safe?** | No | No | No | Yes (with caveats) | Yes (production-proven at AWS scale) |

## 1. Linux Namespaces / Jailing (Weakest)

### What it is

Raw Linux kernel primitives that virtualize global resources. Namespaces change what a process can *see*, not what it can *do*.

### Namespace types

- **PID** (`CLONE_NEWPID`): Isolated process ID tree. Process sees its own PID 1.
- **Network** (`CLONE_NEWNET`): Isolated network stack, routing tables, firewall rules, port space.
- **Mount** (`CLONE_NEWNS`): Isolated filesystem mount points. Different view of the filesystem hierarchy.
- **UTS** (`CLONE_NEWUTS`): Isolated hostname and NIS domain name.
- **IPC** (`CLONE_NEWIPC`): Isolated System V IPC objects and POSIX message queues.
- **User** (`CLONE_NEWUSER`): Isolated UID/GID mappings. Root inside maps to unprivileged UID on host.
- **Cgroup** (`CLONE_NEWCGROUP`): Virtualized view of `/proc/[pid]/cgroup`.
- **Time** (`CLONE_NEWTIME`): Isolated `CLOCK_MONOTONIC` and `CLOCK_BOOTTIME`.

### How it binds to the host kernel

**Directly. The process makes syscalls straight to the host kernel.**

A namespaced process has access to the entire ~385 syscall table. Namespaces do not restrict syscalls at all. They only virtualize resource views (PID numbers, network stacks, mount trees). The kernel processes every syscall from a namespaced process identically to any other process, just with a different namespace context.

```
Process in namespace --> syscall --> Host kernel (all ~385 syscalls available)
                                     ^
                                     Only namespace context changes what
                                     the process sees, NOT what it can call
```

### Resource limiting via cgroups

cgroups (v1 or v2) limit resource consumption (CPU, memory, PIDs, block I/O) but do not prevent privilege escalation or restrict syscalls.

### Security gap

A kernel vulnerability in *any* of the ~385 syscall handlers is exploitable from within a namespace. There is zero syscall filtering, zero capability reduction, and zero MAC enforcement unless you add those layers yourself.

### When to use

Never alone for untrusted code. Raw namespaces are building blocks, not a complete isolation solution.

---

## 2. Docker (Default, on Linux)

### What it is

Namespaces + cgroups + seccomp + capability dropping + AppArmor/SELinux + `/proc`/`/sys` masking. Defense-in-depth on a shared kernel.

### What Docker adds beyond raw namespaces

1. **seccomp filter**: Blocks dangerous syscalls (raw namespaces have zero filtering)
2. **Capability dropping**: Drops ~27 capabilities, keeps only 14
3. **AppArmor/SELinux profile**: MAC policy (`docker-default` or `container_t`)
4. **`/proc` and `/sys` masking**: tmpfs over sensitive paths, `/dev/null` bind-mounts over `/proc/kcore`, `/proc/keys`, etc.
5. **`pivot_root` + unmount old root**: Old root completely inaccessible (unlike chroot)
6. **`no_new_privs` bit**: Prevents setuid escalation
7. **overlay2 filesystem**: Copy-on-write layered filesystem
8. **Bridge networking**: Automated veth pairs + iptables NAT

### Default seccomp profile

- **~361 syscalls unconditionally allowed**
- **~65 syscalls conditionally allowed** (require specific added capabilities)
- **~23+ syscalls completely blocked**: `io_uring_*`, `kexec_load`, `pivot_root`, `userfaultfd`, `vm86`, kernel module syscalls, etc.

### Default capabilities (14 granted, ~27 dropped)

Granted: `CHOWN`, `DAC_OVERRIDE`, `FSETID`, `FOWNER`, `MKNOD`, `NET_RAW`, `SETGID`, `SETUID`, `SETFCAP`, `SETPCAP`, `NET_BIND_SERVICE`, `SYS_CHROOT`, `KILL`, `AUDIT_WRITE`.

Critically NOT granted: `SYS_ADMIN`, `SYS_MODULE`, `SYS_RAWIO`, `SYS_PTRACE`, `NET_ADMIN`, `DAC_READ_SEARCH`, `BPF`.

### How it binds to the host kernel

**Still directly, but with a reduced attack surface.**

```
Container process --> seccomp filter --> Host kernel (~361 syscalls reachable)
                      ^                  ^
                      Blocks ~23+        Still processes all allowed syscalls
                      dangerous calls    on the shared kernel
```

The container process still makes real host kernel syscalls. seccomp reduces which ones, capabilities reduce what root can do, LSM profiles add MAC restrictions. But the kernel is shared. A vulnerability in any of the ~361 allowed syscall paths is exploitable.

### Known escape vectors

- **Kernel exploits**: Dirty COW (CVE-2016-5195), Dirty Pipe (CVE-2022-0847) give full host access from any container.
- **runc vulnerabilities**: CVE-2019-5736 (overwrite host runc binary), CVE-2024-21626 (FD leak), multiple 2025 CVEs.
- **`--privileged` flag**: Grants all capabilities, disables seccomp/AppArmor, gives access to all devices. Container can mount host filesystem, load kernel modules, `nsenter` into host namespaces.
- **Docker socket mount**: If `/var/run/docker.sock` is mounted, container can create new privileged containers.
- **`/proc/sys/kernel/core_pattern`**: Can specify a pipe program that runs on host.
- **cgroup `release_agent`**: Executes on host when last process in cgroup exits.

### Architecture

```
Docker CLI -> dockerd -> containerd -> containerd-shim -> runc -> container process
                                       ^
                                       Per-container parent process,
                                       survives containerd restarts
```

runc does the actual kernel setup: creates namespaces, configures cgroups, does `pivot_root`, masks `/proc`/`/sys`, drops capabilities, applies seccomp, applies LSM profiles, then `execve()` into the container entrypoint.

---

## 3. agentOS / Secure-Exec

### What it is

A POSIX-compatible operating system kernel written in TypeScript that virtualizes all I/O and process management. All syscalls from guest code are intercepted and mediated by the kernel before reaching the host.

**This is architecturally most similar to gVisor.** Both implement a userspace kernel that intercepts syscalls. The key differences are the language, platform, and enforcement mechanism.

### Architecture

```
Agent code (V8 isolate / Worker thread)
    |
    v
Syscall shim (SharedArrayBuffer RPC / Node.js module interception)
    |
    v
Secure-Exec Kernel (TypeScript)
    |-- VFS (in-memory, host dir, S3 backends)
    |-- Process Table (global PIDs, signals, waitpid across runtimes)
    |-- Socket Table (loopback in-kernel, external via HostNetworkAdapter)
    |-- Pipe Manager (64KB buffers, cross-runtime IPC)
    |-- PTY Manager (terminal emulation)
    |-- Permission Wrapper (deny-by-default)
    |
    v
Node.js APIs (fs, net, child_process, crypto)
    |
    v
Host kernel (all syscalls available to Node.js process)
```

### Runtime drivers

Three execution environments, all sharing the same kernel:

- **Node.js Runtime**: V8 isolates via `node-ivm`. All fs/net/process calls shimmed to go through kernel via SharedArrayBuffer RPC.
- **WasmVM Runtime**: Coreutils, busybox, sh, bash compiled to WebAssembly. Runs in Worker threads with WASI polyfill routing syscalls to kernel.
- **Python Runtime**: CPython via Pyodide in Worker thread.

### How it binds to the host kernel

**Indirectly, through Node.js, but the full Node.js API surface is available to the kernel.**

```
Guest code -> Kernel permission check -> TypeScript kernel -> Node.js -> Host kernel
              ^                          ^                    ^
              Deny-by-default            Mediates all I/O     Full libuv/V8/OpenSSL
              per-path/socket/env        No direct bypass     surface available
```

The kernel itself is a normal Node.js process. It can call any Node.js API and therefore any host syscall that Node.js/libuv uses. The isolation comes from:

1. **V8 isolate heap separation**: Guest JS code cannot access kernel memory.
2. **Worker thread isolation**: WASM processes run in isolated threads.
3. **Syscall interception**: All guest I/O goes through kernel-controlled shims.
4. **Permission checks**: Deny-by-default on four domains (fs, network, childProcess, env).

### Permission system

```typescript
interface Permissions {
  fs?: (request: FsAccessRequest) => { allow: boolean; reason?: string };
  network?: (request: NetworkAccessRequest) => { allow: boolean; reason?: string };
  childProcess?: (request: ChildProcessAccessRequest) => { allow: boolean; reason?: string };
  env?: (request: EnvAccessRequest) => { allow: boolean; reason?: string };
}
```

Programmable, fine-grained, per-path/per-socket/per-env-var decisions. This is more flexible than any other model in this comparison.

### Security boundaries

**Layer 1: Runtime isolation** (V8 isolate heap / Worker thread). No shared JS state between processes.

**Layer 2: Kernel permission checks** (deny-by-default). All I/O mediated.

**Layer 3 (coming soon): OS-level jailing** (namespaces + seccomp on the Node.js process). This will restrict the host syscall surface available to the kernel itself, similar to how gVisor's Sentry self-imposes seccomp. This closes the biggest gap with gVisor and makes agentOS a 3-step escape chain.

### How it compares to gVisor (the closest analog)

| | **Secure-Exec** | **gVisor** |
|---|---|---|
| Kernel language | TypeScript | Go |
| Syscall interception | SharedArrayBuffer RPC / module shimming | seccomp `SIGSYS` trap / KVM ring switch |
| Host syscall restriction | None yet (full Node.js surface), jailing coming soon | seccomp-enforced 53-68 syscalls |
| Filesystem proxy | VFS backends (in-memory, host dir, S3) | Gofer process (LISAFS protocol) |
| Network stack | Socket table + HostNetworkAdapter | Netstack (full userspace TCP/IP) |
| Enforcement mechanism | Software (V8 isolate boundary + permission code) | Software (Go memory safety) + seccomp hardware |
| Can bypass kernel? | Only via V8 isolate escape (~1-2/year historically) | Only via Sentry escape + host seccomp bypass |

The critical gap (being addressed): gVisor applies a seccomp filter to *itself* (the Sentry), restricting it to 53-68 host syscalls. Even if the Sentry is fully compromised, the attacker can only use those 53-68 calls. Secure-Exec's kernel currently runs as unrestricted Node.js. OS-level jailing (namespaces + seccomp on the Node.js process) is coming soon, which will restrict the host syscall surface and add a third isolation layer.

### Strengths unique to this model

1. **Cross-runtime process management**: Node, WASM, Python share unified process table with `waitpid()` across runtimes.
2. **Programmable permissions**: Fine-grained policy functions, not just allow/deny.
3. **Near-instant boot**: No guest kernel to start.
4. **No hardware dependency**: Runs anywhere Node.js runs.
5. **Host tools RPC**: Intentional, controlled escape hatch for host resource access.

### Shared kernel caveat

All processes in a Secure-Exec instance share the same TypeScript kernel. They share process table metadata, clock resolution, and heap state. This is agent-sandboxing, not multi-tenant isolation.

---

## 4. gVisor

### What it is

A userspace kernel written in Go that reimplements Linux syscall semantics. Guest processes make syscalls that are intercepted and handled entirely by the Sentry (gVisor's kernel), which then makes a minimal set of host syscalls.

**This is the closest analog to Secure-Exec/agentOS**, but with hardware-assisted enforcement and a restricted host syscall surface.

### Architecture

```
Application process
    |
    v
Syscall trap (seccomp SIGSYS on systrap platform, or VM exit on KVM platform)
    |
    v
Sentry (userspace kernel, Go)
    |-- VFS2 (full virtual filesystem)
    |-- Netstack (userspace TCP/IP stack)
    |-- Memory management (backed by single memfd)
    |-- Process scheduling (goroutines)
    |-- Implements 274 of 350 Linux amd64 syscalls
    |
    |-- [filesystem access] --> Gofer process (LISAFS protocol)
    |                           |
    |                           v
    |                       Host filesystem
    |
    v
Host kernel (only 53-68 syscalls reachable, seccomp-enforced)
```

### Platform layer (how syscalls are intercepted)

**Systrap (default since mid-2023):**
- Uses `SECCOMP_RET_TRAP` to deliver `SIGSYS` when the application attempts a syscall.
- Shared memory communication between stub threads and Sentry.
- On x86-64, patches `syscall` instructions with `jmp` to trampoline, bypassing seccomp overhead entirely after first interception.

**KVM:**
- Sentry runs in guest ring 0, application in guest ring 3.
- Most syscalls handled without VM exit (ring 3 -> ring 0 transition within the guest).
- Best on bare metal. Poor with nested virtualization.

**ptrace (deprecated):**
- Used `PTRACE_SYSEMU`. Very high context-switch overhead.

### How it binds to the host kernel

**Minimally. The Sentry applies a seccomp filter to itself.**

```
Application -> Sentry (handles syscall in userspace) -> Host kernel
                                                         ^
                                                         Only 53-68 syscalls allowed
                                                         (seccomp self-imposed)
```

Critically blocked from the Sentry: `open`, `socket`, `execve`, `fork`, `mount`, `ptrace`. The Sentry cannot open files, create sockets, or spawn processes on the host. Filesystem access goes through the Gofer process.

Compare:
- Raw namespaces: ~385 host syscalls
- Docker default: ~361 host syscalls (from the container process)
- gVisor: 53-68 host syscalls (from the Sentry, which is a separate process from the application)

### Filesystem: Gofer

A separate Go process that mediates all host filesystem access. Communicates with the Sentry via LISAFS protocol. The Sentry operates in an empty mount namespace and cannot open files itself. In directfs mode (now default), the Gofer donates FDs to the Sentry, with seccomp enforcing `O_NOFOLLOW` to prevent symlink traversal attacks.

### Network: Netstack

gVisor implements a complete userspace TCP/IP stack. No host kernel networking code is involved for packet processing. This eliminates the entire host kernel network stack as attack surface. Throughput: ~17 Gbps vs 42 Gbps native (significant but acceptable for security).

### Security properties

- **Memory-safe kernel**: Go eliminates buffer overflows, use-after-free, double-free. Logic bugs are still possible.
- **Self-imposed seccomp**: Even if the Sentry is fully compromised, attacker can only use 53-68 host syscalls.
- **Separate filesystem process**: Gofer runs in its own seccomp sandbox.
- **CVE resistance**: Immune to vulnerabilities in syscall paths it doesn't implement. Example: CVE-2020-14386 (`PACKET_RX_RING`) was unexploitable because gVisor never implemented that code path.

### Performance overhead

- **CPU-bound work**: Near-native.
- **Syscall-heavy workloads**: 3-4x slower (Redis small ops, high-rate I/O).
- **Network throughput**: ~40% of native (Netstack overhead).
- **Memory**: 10-50 MiB per sandbox.

---

## 5. Firecracker (Strongest)

### What it is

A lightweight Virtual Machine Monitor (VMM) that uses hardware virtualization (KVM + VT-x/AMD-V) to run microVMs. Each VM gets its own Linux kernel. The guest never shares a kernel with the host.

### Architecture

```
Guest application
    |
    v
Guest Linux kernel (entirely separate from host)
    |
    v
VM Exit (hardware trap, CPU-enforced)
    |
    v
Firecracker VMM (50K lines Rust, single process)
    |-- virtio-net (network)
    |-- virtio-block (storage)
    |-- virtio-vsock (host communication)
    |-- Serial console
    |-- Keyboard controller (reset only)
    |
    v
Jailer sandbox (namespaces + chroot + seccomp + privilege drop)
    |
    v
Host kernel (~40 syscalls allowed, seccomp-enforced)
```

### How it binds to the host kernel

**Through the narrowest possible pipe, with hardware enforcement.**

The guest never makes host syscalls. The CPU hardware enforces this. When the guest does something that requires VMM intervention (I/O to a virtio device, for example), the CPU traps (VM Exit) and transfers control to the Firecracker VMM process on the host.

```
Guest process -> Guest kernel -> VM Exit (hardware) -> KVM -> Firecracker VMM -> ~40 syscalls -> Host kernel
                 ^                ^                           ^
                 Separate kernel  CPU enforces boundary       Rust, 50K lines
                 (exploit only    (no software bypass)        (memory-safe)
                 affects guest)
```

The Firecracker VMM process itself is sandboxed by the Jailer:
- Linux namespaces (all types)
- chroot into restricted filesystem
- seccomp-bpf filter allowing only ~40 host syscalls with parameter validation
- Runs as unprivileged user after setup
- Cannot regain privileges

### Device model

Only 5 emulated devices (QEMU has hundreds). Each device is a potential attack surface, so minimizing them is a security strategy:
- **virtio-net**: Network interface
- **virtio-block**: Block storage
- **virtio-vsock**: VM sockets for host-guest communication
- **Serial console**: Logging
- **Keyboard controller**: Guest reset only

No USB, GPU, audio, or any other device.

### Security boundary depth (4 independent layers)

```
Layer 1: CPU hardware (VT-x/AMD-V)
         Guest cannot execute host instructions. Period.
         Breaking this requires a CPU microarchitecture bug.

Layer 2: Firecracker VMM (Rust, 50K lines)
         Translates virtio requests to host I/O.
         Memory-safe. Minimal device model.

Layer 3: Jailer (namespaces + seccomp + chroot)
         Even if VMM is compromised, attacker is in a sandbox
         with ~40 syscalls and no filesystem access.

Layer 4: Privilege separation
         Unprivileged process. Cannot escalate.
```

A guest escape requires breaching ALL FOUR layers. Each is independent. This is why AWS trusts it for Lambda (billions of untrusted invocations on shared hardware).

### Performance

- Boot: ~125ms (vs 1-2s traditional VMs)
- Memory: <5 MiB per microVM
- Density: thousands per host
- Creation rate: up to 150 microVMs/second/host

---

## Comparative Analysis

### Host Kernel Attack Surface

```
Namespaces:    ████████████████████████████████████████  ~385 syscalls (full kernel)
Docker:        ██████████████████████████████████████    ~361 syscalls (seccomp filtered)
agentOS:       ████████████████████████████████████████  ~385 syscalls (Node.js unrestricted, jailing coming soon)
gVisor:        ██████                                    53-68 syscalls (self-imposed seccomp)
Firecracker:   █████                                     ~40 syscalls (Jailer seccomp, from VMM only)
```

Note: agentOS's host syscall surface is currently comparable to raw namespaces because the kernel runs as an unrestricted Node.js process. The difference is that *guest code* cannot directly invoke those syscalls. It must go through the TypeScript kernel. OS-level jailing (namespaces + seccomp) is coming soon, which will significantly reduce this surface.

### Where the Security Boundary Lives

```
                        Hardware        Software kernel    Software checks
                        enforced?       reimplemented?     only?

Namespaces:                                                     X
Docker:                                                         X
agentOS:          X (jail, soon)             X (partial)         X
gVisor:              X (seccomp)            X (full)
Firecracker:         X (VT-x + seccomp)
```

### What a Kernel Exploit Gets You

- **Namespaces/Docker**: Full host compromise. The container process runs on the host kernel. A kernel bug in any reachable syscall = game over.
- **agentOS**: N/A in the traditional sense. There's no guest kernel to exploit. But a V8 isolate escape gives you access to the TypeScript kernel, and a bug in the TypeScript kernel gives you the Node.js/host surface. With jailing (coming soon), even a kernel bug would only expose a restricted set of host syscalls.
- **gVisor**: Guest kernel exploit is meaningless (the Sentry is not a real kernel, it's a Go process). You'd need a logic bug in the Sentry + a way to exploit one of 53-68 host syscalls.
- **Firecracker**: Guest kernel exploit only compromises the guest. You still need to escape the hardware VM boundary + VMM + Jailer to reach the host.

### Escape Chain Length

```
Namespaces:    1 step  (kernel exploit)
Docker:        1 step  (kernel exploit, or runc bug, or misconfiguration)
agentOS:       3 steps (V8 escape -> kernel bug -> jail escape)  [jailing coming soon]
gVisor:        2 steps (Sentry logic bug -> exploit one of 53-68 host syscalls)
Firecracker:   4 steps (guest kernel -> VM escape -> VMM bug -> Jailer escape)
```

Note: Without jailing, agentOS is currently 2 steps (V8 escape -> kernel bug). With jailing, even after breaching the V8 isolate and exploiting a kernel bug, the attacker must also escape the OS-level jail (namespaces + seccomp) to reach the full host.

### The gVisor-agentOS Parallel

agentOS and gVisor are architecturally very similar. Both:

- Reimplement OS syscall semantics in a memory-safe language
- Intercept all guest syscalls before they reach the host
- Maintain a virtual filesystem layer
- Run guest processes in isolated contexts

The key differences that make gVisor stronger:

1. **Self-imposed seccomp**: gVisor restricts *its own* host syscall surface to 53-68 calls. agentOS's kernel is currently unrestricted Node.js. OS-level jailing is coming soon to close this gap.
2. **Separate filesystem process**: gVisor's Gofer runs in its own seccomp sandbox. agentOS accesses host filesystem directly from the kernel process.
3. **Userspace network stack**: gVisor implements full TCP/IP (Netstack), eliminating host kernel networking attack surface. agentOS delegates external connections to Node.js `net`/`dgram`.
4. **Hardware-assisted interception** (optional): gVisor's KVM platform uses hardware ring separation. Systrap uses seccomp hardware traps. agentOS relies on V8/SharedArrayBuffer for interception.

What makes agentOS more flexible:

1. **Programmable permissions**: Fine-grained policy functions per-path/per-socket. gVisor is binary (in sandbox or not).
2. **Cross-runtime process management**: Node + WASM + Python unified process table.
3. **Host tools RPC**: Intentional, controlled escape hatch for host resource access.
4. **No hardware dependency**: Runs anywhere Node.js runs.
5. **Near-instant boot**: No guest kernel, no Sentry startup.

### Hardening Roadmap for agentOS

**Coming soon: OS-level jailing.** The Node.js kernel process will be run inside a jail (namespaces + seccomp), restricting the host syscall surface available even if the kernel itself is compromised. This is the single highest-impact improvement and brings agentOS to a 3-step escape chain (V8 escape -> kernel bug -> jail escape), comparable in structure to gVisor.

Further hardening opportunities:

1. **Separate filesystem access into a child process**: Like gVisor's Gofer, mediate host filesystem access through a separately sandboxed process.
2. **Implement in-kernel networking**: Handle TCP/IP within the kernel rather than delegating to Node.js, reducing host kernel network stack exposure.

---

## References

### gVisor
- [gVisor architecture overview](https://gvisor.dev/docs/architecture_guide/)
- [gVisor security model](https://gvisor.dev/docs/architecture_guide/security/)
- [gVisor platforms](https://gvisor.dev/docs/architecture_guide/platforms/)
- [Google Security Blog - gVisor](https://security.googleblog.com/2018/05/open-sourcing-gvisor-sandboxed-container.html)

### Firecracker
- [Firecracker design](https://firecracker-microvm.github.io/)
- [Amazon Science - How Firecracker VMs work](https://www.amazon.science/blog/how-awss-firecracker-virtual-machines-work/)
- [Firecracker internals](https://www.talhoffman.com/2021/07/18/firecracker-internals/)
- [Unixism - Firecracker deep dive](https://unixism.net/2019/10/how-aws-firecracker-works-a-deep-dive/)

### Docker / Containers
- [Docker seccomp documentation](https://docs.docker.com/engine/security/seccomp/)
- [moby/profiles default seccomp](https://github.com/moby/profiles/blob/main/seccomp/default.json)
- [runc CVE history](https://www.cncf.io/blog/2025/11/28/runc-container-breakout-vulnerabilities-a-technical-overview/)

### Linux Namespaces
- [namespaces(7) man page](https://man7.org/linux/man-pages/man7/namespaces.7.html)
- [cgroups(7) man page](https://man7.org/linux/man-pages/man7/cgroups.7.html)
- [Container security fundamentals - Datadog](https://securitylabs.datadoghq.com/articles/container-security-fundamentals-part-6/)
