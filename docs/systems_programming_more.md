# iximiuz-Inspired Challenges for Lux (Go Edition)

These challenges focus on Linux primitives, container internals, and low-level systems programming - perfect for understanding how Docker/Kubernetes actually work under the hood.

---

## Stage 9: Linux Namespaces (8 challenges)

### 5.1 Create Network Namespace
**Binary**: `./netns create <name>`
- Use `unshare(2)` syscall or exec `ip netns add`
- Create isolated network stack
- Verify namespace exists
- **E2E Tests**: lux verifies namespace creation, checks `/var/run/netns/`

### 5.2 Execute in Network Namespace
**Binary**: `./netns exec <name> <command>`
- Run command inside existing namespace
- Use `setns(2)` syscall
- Capture output correctly
- **E2E Tests**: lux runs commands, verifies isolation

### 5.3 PID Namespace Container
**Binary**: `./pidns-container <rootfs> <command>`
- Create PID namespace using `unshare(2)`
- Process becomes PID 1 inside
- Mount /proc correctly
- **E2E Tests**: lux verifies `ps aux` shows only container processes

### 5.4 Mount Namespace Isolation
**Binary**: `./mntns <command>`
- Create mount namespace
- Private mount propagation
- Isolate filesystem views
- **E2E Tests**: lux mounts filesystem, verifies isolation

### 5.5 UTS Namespace (Hostname)
**Binary**: `./uts-container <hostname> <command>`
- Set custom hostname in UTS namespace
- Doesn't affect host hostname
- **E2E Tests**: lux verifies hostname inside != host

### 5.6 IPC Namespace
**Binary**: `./ipcns <command>`
- Create IPC namespace
- Isolate message queues, semaphores
- Verify via `ipcs`
- **E2E Tests**: lux creates IPC objects, verifies isolation

### 5.7 User Namespace Mapping
**Binary**: `./userns --uid-map <map> <command>`
- Create user namespace
- Map UIDs/GIDs (e.g., root→unprivileged)
- Write uid_map, gid_map
- **E2E Tests**: lux verifies UID mapping, root inside != root outside

### 5.8 Multi-Namespace Container
**Binary**: `./container run <rootfs> <command>`
- Combine: PID, mount, network, UTS, IPC namespaces
- Full process isolation
- Basic container runtime
- **E2E Tests**: lux verifies all namespaces active

---

## Stage 10: Cgroups v2 (8 challenges)

### 5.9 CPU Quota Limiter
**Binary**: `./cgroup-cpu <percent> <command>`
- Create cgroup in `/sys/fs/cgroup`
- Set cpu.max (e.g., "50000 100000" for 50%)
- Move process to cgroup
- **E2E Tests**: lux runs CPU-intensive task, verifies throttling

### 5.10 Memory Limiter
**Binary**: `./cgroup-mem <limit-mb> <command>`
- Set memory.max
- Process gets OOM killed if exceeds
- Read memory.current for usage
- **E2E Tests**: lux allocates memory, verifies limit enforcement

### 5.11 Process Freezer
**Binary**: `./cgroup-freeze <pid>`
- Use cgroup.freeze to pause process
- Read cgroup.events to confirm
- Unfreeze with cgroup.freeze=0
- **E2E Tests**: lux starts process, freezes, verifies no CPU usage

### 5.12 I/O Bandwidth Limiter
**Binary**: `./cgroup-io <device> <bps> <command>`
- Set io.max (bytes per second)
- Throttle disk I/O
- **E2E Tests**: lux writes data, measures throughput

### 5.13 PID Limiter
**Binary**: `./cgroup-pids <max> <command>`
- Set pids.max
- Prevent fork bomb
- **E2E Tests**: lux tries to spawn more processes than limit

### 5.14 Cgroup Hierarchy Manager
**Binary**: `./cgroup tree <root>`
- Read cgroup.subtree_control
- Display cgroup tree structure
- Show resource usage per cgroup
- **E2E Tests**: lux creates hierarchy, verifies tree output

### 5.15 OOM Group Handler
**Binary**: `./cgroup-oom <command>`
- Set memory.oom.group=1
- When OOM, kill entire cgroup
- Monitor memory.events
- **E2E Tests**: lux triggers OOM, verifies all processes killed

### 5.16 Systemd Slice Integration
**Binary**: `./slice-runner <slice> <command>`
- Use systemd-run to create transient scope
- Run process in custom slice
- Parent-child limit inheritance
- **E2E Tests**: lux verifies systemd integration

---

## Stage 11: Virtual Networking (8 challenges)

### 5.17 Veth Pair Creator
**Binary**: `./veth create <name1> <name2>`
- Create virtual ethernet pair
- Place each end in different namespace
- Link them together
- **E2E Tests**: lux creates pair, verifies connectivity

### 5.18 Bridge Network Setup
**Binary**: `./bridge <name> <subnet>`
- Create Linux bridge
- Assign IP address
- Enable IP forwarding
- **E2E Tests**: lux connects veth pairs to bridge

### 5.19 Container Network Connector
**Binary**: `./netconnect <netns> <bridge> <ip>`
- Create veth pair
- One end to netns, one to bridge
- Configure IP address and routing
- **E2E Tests**: lux pings from netns through bridge

### 5.20 NAT with iptables
**Binary**: `./nat-setup <internal-subnet> <external-if>`
- Configure MASQUERADE rule
- Enable IP forwarding
- Allow outbound from containers
- **E2E Tests**: lux verifies internet access from netns

### 5.21 Port Forwarding
**Binary**: `./portfwd <host-port> <container-ip:port>`
- Use iptables DNAT
- Forward host port to container
- **E2E Tests**: lux connects to host port, reaches container

### 5.22 Docker-like Network
**Binary**: `./docker-net create <name> <subnet>`
- Full Docker networking stack
- Bridge + NAT + port forwarding
- IP allocation for containers
- **E2E Tests**: lux spawns "containers", verifies connectivity

### 5.23 Network Namespace Inspector
**Binary**: `./netns-inspect <name>`
- List all interfaces in namespace
- Show IP addresses, routes
- Display iptables rules
- **E2E Tests**: lux creates network, verifies inspection output

### 5.24 Container to Container Ping
**Binary**: `./ping-test <netns1> <netns2>`
- Create two network namespaces
- Connect via bridge
- Ping between them
- **E2E Tests**: lux verifies ICMP packets

---

## Stage 12: Container Runtime (8 challenges)

### 5.25 OCI Runtime Spec Parser
**Binary**: `./oci-parse <config.json>`
- Parse OCI runtime spec
- Validate required fields
- Print configuration
- **E2E Tests**: lux provides spec, verifies parsing

### 5.26 Rootfs Setup
**Binary**: `./rootfs-setup <source> <target>`
- Create container rootfs
- Bind mount or copy
- Setup /proc, /sys, /dev
- **E2E Tests**: lux verifies filesystem structure

### 5.27 Container with Overlayfs
**Binary**: `./overlay-container <lower> <upper> <command>`
- Create overlay mount
- Lower (read-only), upper (read-write)
- Run process with overlay rootfs
- **E2E Tests**: lux writes files, verifies in upper dir only

### 5.28 Capability Dropper
**Binary**: `./drop-caps <command>`
- Drop all capabilities except essential
- Use `capset(2)` or `prctl(2)`
- Run unprivileged
- **E2E Tests**: lux verifies reduced capabilities

### 5.29 Seccomp Filter
**Binary**: `./seccomp <command>`
- Apply seccomp-bpf filter
- Block dangerous syscalls (e.g., reboot)
- Allow common syscalls
- **E2E Tests**: lux tries blocked syscalls, verifies denial

### 5.30 AppArmor Profile
**Binary**: `./apparmor <profile> <command>`
- Load AppArmor profile
- Restrict file access
- Run confined process
- **E2E Tests**: lux attempts restricted operations

### 5.31 Container Lifecycle Manager
**Binary**: `./runc create|start|kill|delete <id>`
- Implement basic runc commands
- Manage container state
- PID file, state.json
- **E2E Tests**: lux executes lifecycle, verifies states

### 5.32 Container Checkpoint/Restore (CRIU)
**Binary**: `./checkpoint <pid> <dir>`
- Use CRIU to checkpoint process
- Save state to disk
- Restore from checkpoint
- **E2E Tests**: lux checkpoints, kills, restores, verifies state

---

## Stage 13: Process Inspection (6 challenges)

### 5.33 /proc Reader
**Binary**: `./proc-read <pid>`
- Read `/proc/<pid>/status`
- Parse VmSize, VmRSS, etc.
- Display formatted output
- **E2E Tests**: lux spawns process, verifies stats

### 5.34 Namespace Inspector
**Binary**: `./ns-inspect <pid>`
- Read `/proc/<pid>/ns/*`
- Show namespace IDs
- Compare with other processes
- **E2E Tests**: lux verifies namespace detection

### 5.35 File Descriptor Lister
**Binary**: `./fd-list <pid>`
- Read `/proc/<pid>/fd/`
- Show open files, sockets, pipes
- Resolve symlinks
- **E2E Tests**: lux opens files, verifies listing

### 5.36 Memory Map Viewer
**Binary**: `./maps <pid>`
- Read `/proc/<pid>/maps`
- Parse memory regions
- Show permissions, offsets
- **E2E Tests**: lux verifies memory map parsing

### 5.37 Process Tree Builder
**Binary**: `./pstree`
- Read `/proc/*/stat` for all processes
- Build parent-child tree
- ASCII visualization
- **E2E Tests**: lux spawns hierarchy, verifies tree

### 5.38 Cgroup Path Resolver
**Binary**: `./cgroup-path <pid>`
- Read `/proc/<pid>/cgroup`
- Resolve cgroup path
- Show hierarchy
- **E2E Tests**: lux verifies cgroup detection

---

## Stage 14: Container Debugging (8 challenges)

### 5.39 Nsenter Clone
**Binary**: `./nsenter <pid> <command>`
- Enter all namespaces of target PID
- Execute command inside
- Similar to `nsenter(1)`
- **E2E Tests**: lux enters container, runs commands

### 5.40 Container Shell Injector
**Binary**: `./inject-shell <container-id>`
- Find container PID
- Inject shell into container namespaces
- Interactive mode
- **E2E Tests**: lux injects, executes commands

### 5.41 Host Command in Container
**Binary**: `./host-exec <netns> <command>`
- Keep mount namespace from host
- Use target network namespace
- Access container network from host tools
- **E2E Tests**: lux uses host curl to reach container service

### 5.42 Container Signal Sender
**Binary**: `./container-signal <id> <signal>`
- Find container init process (PID 1)
- Send signal (SIGUSR1, SIGTERM)
- Handle PID namespace translation
- **E2E Tests**: lux sends signals, verifies handler execution

### 5.43 Container Logs Tailer
**Binary**: `./logs <container-id>`
- Find container stdout/stderr
- Tail logs from `/proc/<pid>/fd/1` or log file
- Follow mode (-f)
- **E2E Tests**: lux captures container output

### 5.44 Container File Copier
**Binary**: `./cp-from-container <id>:<src> <dest>`
- Use tar to stream files
- Handle permissions correctly
- Work without shell in container
- **E2E Tests**: lux copies files, verifies content

### 5.45 Container Resource Monitor
**Binary**: `./container-stats <id>`
- Read cgroup metrics
- Show CPU, memory, I/O usage
- Real-time updates
- **E2E Tests**: lux verifies stats accuracy

### 5.46 Container Network Debug
**Binary**: `./net-debug <container-id>`
- Show interfaces, IPs, routes
- Display iptables rules affecting container
- Trace packet path
- **E2E Tests**: lux analyzes connectivity issues

---

## Stage 15: Advanced Container Tech (6 challenges)

### 5.47 Pause Container
**Binary**: `./pause`
- Minimal container that just sleeps
- Used by Kubernetes for pod networking
- Holds namespaces open
- **E2E Tests**: lux verifies namespace sharing

### 5.48 Sidecar Container
**Binary**: `./sidecar <main-container-id>`
- Share some namespaces with main container
- Independent mount namespace
- Pod-like behavior
- **E2E Tests**: lux verifies selective namespace sharing

### 5.49 Container Image Puller
**Binary**: `./pull <image>`
- Download OCI image from registry
- Handle manifest, layers
- Extract to filesystem
- **E2E Tests**: lux verifies image download

### 5.50 Container Image Builder
**Binary**: `./build <dockerfile> <tag>`
- Parse Dockerfile
- Execute RUN, COPY, CMD
- Create OCI image layers
- **E2E Tests**: lux builds image, verifies layers

### 5.51 Container Registry Server
**Binary**: `./registry <port>`
- Implement OCI Distribution API
- Push/pull images
- Manifest handling
- **E2E Tests**: lux pushes/pulls images

### 5.52 Rootless Container Runtime
**Binary**: `./rootless <command>`
- Run containers without root
- User namespaces for UID mapping
- Unprivileged overlayfs
- **E2E Tests**: lux runs as non-root, verifies isolation

---

## Key Learning Outcomes

These challenges teach:

1. **How containers actually work** - No Docker abstraction, raw Linux primitives
2. **Kubernetes pod model** - Namespace sharing, pause containers, sidecars
3. **Security isolation** - Capabilities, seccomp, AppArmor
4. **Resource management** - Cgroups v2 controllers
5. **Container networking** - Veth pairs, bridges, NAT, iptables
6. **Debugging production containers** - Inspect, execute, signal handling
7. **OCI standards** - Runtime spec, image format

## Testing Approach

Each challenge uses E2E testing by lux:
- **Spawn processes/containers**: lux creates test environments
- **Check kernel state**: Read `/proc`, `/sys/fs/cgroup` to verify
- **Test isolation**: Verify namespaces, cgroups work correctly
- **Validate behavior**: Ping, signal, inspect commands

Example test flow:
```
Challenge: Create Network Namespace
1. User builds: go build -o netns main.go
2. lux runs: ./netns create test-ns
3. lux verifies:
   ✓ /var/run/netns/test-ns exists
   ✓ ip netns list shows test-ns
   ✓ Namespace has lo interface
4. lux cleanup: delete namespace
```

Perfect for understanding container internals before learning Docker/Kubernetes!
