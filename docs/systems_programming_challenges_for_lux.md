# Systems Programming Challenges for Lux

## Stage 1: Network Fundamentals (6 challenges)

### 4.1 TCP Echo Client
**Binary**: `./client <host> <port>`
- Connect to server, echo stdin ↔ stdout
- Handle connection errors, EOF, timeouts
- **E2E Tests**: lux spawns echo server, verifies data flow

### 4.2 TCP Echo Server
**Binary**: `./server <port>`
- Accept connections, echo back messages
- Handle multiple concurrent clients
- Graceful shutdown on SIGTERM
- **E2E Tests**: lux connects multiple clients, verifies echo, tests signals

### 4.3 UDP Echo Server
**Binary**: `./udpserver <port>`
- Receive UDP packets, echo back to sender
- Handle packet loss scenarios
- No connection state needed
- **E2E Tests**: lux sends UDP packets, verifies responses

### 4.4 Port Scanner
**Binary**: `./portscan <host> <start-port> <end-port>`
- Scan range of ports, report open ones
- Concurrent scanning with worker pool
- Timeout handling
- **E2E Tests**: lux starts servers on specific ports, verifies detection

### 4.5 HTTP Client (Raw Sockets)
**Binary**: `./httpclient <url>`
- Parse URL, establish TCP connection
- Send HTTP/1.1 GET request manually
- Parse response, print body
- **E2E Tests**: lux runs HTTP server, verifies request format

### 4.6 Netcat Clone
**Binary**: `./nc <host> <port>` or `./nc -l <port>`
- Client mode: connect and relay stdin/stdout
- Server mode: listen and relay
- Both TCP and UDP support (`-u` flag)
- **E2E Tests**: bidirectional communication, both modes

---

## Stage 2: Process & Signal Management (6 challenges)

### 4.7 Shell Command Executor
**Binary**: `./exec <command> [args...]`
- Execute external commands
- Capture stdout/stderr separately
- Report exit codes
- **E2E Tests**: lux verifies command execution, output capture

### 4.8 Process Monitor
**Binary**: `./pmon <pid>`
- Monitor process CPU, memory usage
- Report every second until process exits
- Handle process not found
- **E2E Tests**: lux spawns processes, verifies stats reporting

### 4.9 Signal Handler
**Binary**: `./sighandler`
- Catch SIGTERM, SIGINT, SIGUSR1, SIGUSR2
- Graceful cleanup on SIGTERM
- State dump on SIGUSR1
- **E2E Tests**: lux sends signals, verifies handlers

### 4.10 Daemon Process
**Binary**: `./daemon start|stop|status`
- Fork into background
- Write PID file
- Detach from terminal
- **E2E Tests**: lux verifies daemonization, PID file, process isolation

### 4.11 Job Scheduler
**Binary**: `./scheduler <interval> <command>`
- Run command every N seconds
- Handle command failures
- Graceful shutdown
- **E2E Tests**: lux verifies execution timing, counts runs

### 4.12 Process Pool Manager
**Binary**: `./pool <workers> <command>`
- Spawn N worker processes
- Distribute work via stdin lines
- Collect results
- **E2E Tests**: lux sends work items, verifies parallel processing

---

## Stage 3: File System & I/O (8 challenges)

### 4.13 File Watcher
**Binary**: `./watch <directory>`
- Monitor directory for changes
- Report: created, modified, deleted files
- Use inotify (Linux) or kqueue (BSD/macOS)
- **E2E Tests**: lux creates/modifies files, verifies notifications

### 4.14 Log Rotator
**Binary**: `./rotate <logfile> <max-size-mb>`
- Monitor log file size
- Rotate when exceeds limit (rename to .1, .2, etc.)
- Keep last N rotations
- **E2E Tests**: lux writes data, verifies rotation

### 4.15 File Synchronizer
**Binary**: `./sync <source> <dest>`
- Copy only changed files
- Use checksums or timestamps
- Handle nested directories
- **E2E Tests**: lux creates file tree, modifies files, verifies sync

### 4.16 Tail Implementation
**Binary**: `./tail [-f] [-n lines] <file>`
- Print last N lines
- Follow mode: watch for new lines
- Handle log rotation
- **E2E Tests**: lux appends data, verifies output

### 4.17 Directory Tree Walker
**Binary**: `./tree <path> [--maxdepth N]`
- Recursively traverse directories
- Print tree structure
- Handle symlinks (don't loop)
- **E2E Tests**: lux creates nested dirs, verifies output

### 4.18 Memory-Mapped File Reader
**Binary**: `./mmap <file> <offset> <length>`
- Use mmap to read large files
- Print specified byte range
- Handle invalid ranges
- **E2E Tests**: lux creates large file, verifies byte-accurate reading

### 4.19 Disk Usage Analyzer
**Binary**: `./du <path>`
- Calculate directory sizes recursively
- Sort by size, format output (KB/MB/GB)
- Handle permissions errors
- **E2E Tests**: lux creates file hierarchy with known sizes

### 4.20 Binary File Differ
**Binary**: `./bindiff <file1> <file2>`
- Compare binary files byte-by-byte
- Report first difference offset
- Hex dump of differing regions
- **E2E Tests**: lux creates binary files, verifies diff detection

---

## Stage 4: Inter-Process Communication (6 challenges)

### 4.21 Named Pipe (FIFO) Server
**Binary**: `./fifoserver <pipe-path>`
- Create named pipe
- Read messages, respond to same pipe
- Handle multiple readers/writers
- **E2E Tests**: lux writes to pipe, verifies responses

### 4.22 Unix Domain Socket Server
**Binary**: `./unixserver <socket-path>`
- Listen on Unix socket
- Echo or process commands
- Handle concurrent connections
- **E2E Tests**: lux connects via Unix socket

### 4.23 Shared Memory Queue
**Binary**: `./shmqueue <name> <size>`
- Create shared memory segment
- Implement lock-free circular buffer
- Multiple producers/consumers
- **E2E Tests**: lux spawns multiple processes, verifies data passing

### 4.24 Message Queue
**Binary**: `./mq send|recv <queue-name>`
- Send/receive messages to named queue
- Priority handling
- Non-blocking operations
- **E2E Tests**: lux sends messages, verifies FIFO/priority

### 4.25 Pipe Chain Executor
**Binary**: `./pipechain <cmd1> | <cmd2> | <cmd3>`
- Execute pipeline of commands
- Connect stdout → stdin
- Handle pipeline errors
- **E2E Tests**: lux verifies data flows through chain

### 4.26 Event Bus
**Binary**: `./eventbus`
- Publish/subscribe message broker
- Multiple topics
- Unix socket API
- **E2E Tests**: lux spawns subscribers, publishes, verifies delivery

---

## Stage 5: Network Protocols (8 challenges)

### 4.27 DNS Resolver
**Binary**: `./resolve <hostname>`
- Parse DNS query format
- Send UDP query to DNS server
- Parse response, print IP addresses
- **E2E Tests**: lux runs mock DNS server, verifies query format

### 4.28 DHCP Client
**Binary**: `./dhcpclient <interface>`
- Discover, Request, Acknowledge flow
- Parse DHCP options
- Print assigned IP, gateway, DNS
- **E2E Tests**: lux runs DHCP server, verifies protocol

### 4.29 Redis Protocol Client
**Binary**: `./redis-cli <host> <port> <command>`
- Implement RESP protocol
- Send commands: GET, SET, PING
- Parse responses
- **E2E Tests**: lux runs Redis server, verifies protocol compliance

### 4.30 HTTP/1.1 Server
**Binary**: `./httpserver <port> <root-dir>`
- Parse HTTP requests
- Serve static files
- Handle: GET, HEAD, POST
- Response headers, status codes
- **E2E Tests**: lux sends various HTTP requests, verifies responses

### 4.31 WebSocket Server
**Binary**: `./wsserver <port>`
- HTTP upgrade to WebSocket
- Frame parsing/encoding
- Ping/pong, close handshake
- **E2E Tests**: lux connects as WS client, verifies handshake

### 4.32 SMTP Client
**Binary**: `./sendmail <to> <subject> <body>`
- Connect to SMTP server
- HELO, MAIL FROM, RCPT TO, DATA
- Send email
- **E2E Tests**: lux runs mock SMTP server, verifies protocol

### 4.33 FTP Client
**Binary**: `./ftp <host> <user> <pass>`
- Control and data connections
- LIST, RETR, STOR commands
- ASCII and binary modes
- **E2E Tests**: lux runs FTP server, verifies file transfer

### 4.34 Syslog Server
**Binary**: `./syslogd <port>`
- Receive UDP syslog messages
- Parse facility, severity, message
- Write to file with rotation
- **E2E Tests**: lux sends syslog messages, verifies parsing

---

## Stage 6: Concurrency Patterns (6 challenges)

### 4.35 Rate Limiter Service
**Binary**: `./ratelimit <requests-per-second>`
- HTTP server with rate limiting
- Token bucket algorithm
- Per-IP rate limits
- **E2E Tests**: lux floods requests, verifies throttling

### 4.36 Connection Pool
**Binary**: `./connpool <max-connections> <backend>`
- Proxy that pools backend connections
- Reuse connections across requests
- Health checks
- **E2E Tests**: lux sends concurrent requests, verifies pooling

### 4.37 Worker Queue
**Binary**: `./workers <count>`
- HTTP API to submit jobs
- Worker pool processes jobs
- Job status tracking
- **E2E Tests**: lux submits jobs, verifies parallel processing

### 4.38 Pub/Sub Broker
**Binary**: `./broker <port>`
- TCP server with pub/sub
- Topic-based routing
- Persistent connections
- **E2E Tests**: lux spawns publishers/subscribers

### 4.39 Load Balancer
**Binary**: `./lb <listen-port> <backend1> <backend2>...`
- Round-robin or least-connections
- Health checking
- Connection forwarding
- **E2E Tests**: lux sends requests, verifies distribution

### 4.40 Circuit Breaker Proxy
**Binary**: `./circuit <backend> <failure-threshold>`
- Proxy with circuit breaker
- Open/half-open/closed states
- Fail fast when open
- **E2E Tests**: lux simulates backend failures, verifies state transitions

---

## Stage 7: System Utilities (8 challenges)

### 4.41 Process Tree Viewer
**Binary**: `./pstree`
- Read /proc to build process tree
- Show parent-child relationships
- ASCII tree visualization
- **E2E Tests**: lux spawns process hierarchy, verifies tree

### 4.42 Network Interface Stats
**Binary**: `./ifstat [interface]`
- Read /proc/net/dev or similar
- Show packets, bytes, errors
- Continuous updates
- **E2E Tests**: lux verifies stat parsing

### 4.43 TCP Connection Monitor
**Binary**: `./tcpmon`
- List active TCP connections
- Show local/remote addresses, state
- Continuous monitoring
- **E2E Tests**: lux opens connections, verifies detection

### 4.44 System Logger
**Binary**: `./logger <message>`
- Write to system log
- Support different facilities
- Timestamp, hostname, process ID
- **E2E Tests**: lux verifies log entries

### 4.45 Cron Clone
**Binary**: `./cron <schedule> <command>`
- Parse cron expressions
- Execute commands on schedule
- Logging and error handling
- **E2E Tests**: lux verifies execution timing

### 4.46 Service Manager
**Binary**: `./service start|stop|restart <name>`
- Manage long-running processes
- Restart on failure
- Dependency ordering
- **E2E Tests**: lux verifies lifecycle management

### 4.47 Metrics Collector
**Binary**: `./metrics`
- Collect CPU, memory, disk, network stats
- Expose via HTTP endpoint
- Prometheus format
- **E2E Tests**: lux scrapes metrics, verifies format

### 4.48 Container Runtime (Mini Docker)
**Binary**: `./container run <image> <command>`
- Linux namespaces (PID, mount, network)
- cgroups for resource limits
- Rootfs setup
- **E2E Tests**: lux verifies process isolation

---

## Stage 8: Advanced Projects (8 challenges)

### 4.49 Key-Value Store Server
**Binary**: `./kvstore <port>`
- TCP protocol: GET, SET, DELETE
- In-memory storage
- Optional persistence
- **E2E Tests**: lux performs operations, verifies consistency

### 4.50 Time-Series Database
**Binary**: `./tsdb <port>`
- Store timestamped metrics
- Query by time range
- Aggregation functions
- **E2E Tests**: lux inserts data, queries, verifies results

### 4.51 Distributed Cache
**Binary**: `./cache <port> <peers...>`
- Consistent hashing
- Peer discovery
- Replication
- **E2E Tests**: lux spawns cluster, verifies distribution

### 4.52 Log Aggregator
**Binary**: `./logagg <port>`
- Receive logs from multiple sources
- Parse, index, query
- Full-text search
- **E2E Tests**: lux ships logs, performs searches

### 4.53 Reverse Proxy
**Binary**: `./proxy <listen> <upstream>`
- HTTP reverse proxy
- Request/response modification
- Caching
- **E2E Tests**: lux sends requests through proxy

### 4.54 VPN Tunnel (TUN/TAP)
**Binary**: `./vpn <config>`
- Create TUN device
- Encrypt/decrypt packets
- Route traffic
- **E2E Tests**: lux verifies packet routing

### 4.55 Network Packet Sniffer
**Binary**: `./sniffer <interface>`
- Capture packets (raw sockets)
- Parse Ethernet, IP, TCP, UDP headers
- Filter expressions
- **E2E Tests**: lux generates traffic, verifies capture

### 4.56 Distributed Lock Service
**Binary**: `./lockd <port>`
- Raft consensus
- Lock acquisition/release
- Lease management
- **E2E Tests**: lux spawns cluster, verifies consistency

---

## Testing Approach Summary

Each challenge:
1. **User builds**: `go build -o <binary> main.go`
2. **lux executes**: `./<binary> <args>`
3. **lux verifies**:
   - Network behavior (sends/receives data)
   - File system changes
   - Process behavior
   - Signal handling
   - Output correctness
   - Resource cleanup
4. **Pass/fail**: Based on observable behavior

No unit tests - pure E2E behavioral testing, just like CodeCrafters!
