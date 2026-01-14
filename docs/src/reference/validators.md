# Validators

Validators are the test cases that verify your implementation. Each task has one or more validators.

## Validator Types

### can_compile

Checks if your project compiles successfully.

```
can_compile:bool(true)
```

Supports: Go, Rust, C (Makefile), Python, TypeScript

### tcp_listening

Checks if a TCP server is listening on a port.

```
tcp_listening:int(8080)
```

### http_response

Makes an HTTP request and validates the response.

```
http_response:method(GET),path(/),status(200)
http_response:method(POST),path(/api),status(201),body_contains(success)
```

### concurrent_requests

Tests handling of concurrent HTTP requests.

```
concurrent_requests:num(100),path(/),expected_status(200)
```

### graceful_shutdown

Tests that your server shuts down cleanly on SIGTERM.

```
graceful_shutdown:binary(./server),timeout_ms(5000)
```

### race_detector

Runs Go's race detector on your code (requires Docker).

```
race_detector:source_dir(.)
```

### file_exists

Checks if a file or directory exists.

```
file_exists:path(.git/objects)
```

## Docker-Based Validators

Some validators (like `race_detector`) run inside Docker containers. These require Docker to be installed and running.

Progress messages are shown during Docker operations:

```
  building with race detector (this may take a moment)...
  running race detection tests...
```
