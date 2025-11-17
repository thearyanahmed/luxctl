# lighthouse CLI - Technical Specification

## Overview
A Rust-based CLI tool that validates local systems programming exercises without requiring cloud infrastructure.

## Why Rust
- Cross-platform single binary distribution
- Strong async support (tokio) for network testing
- Excellent process management
- Fast validation = better UX
- Dogfooding (teaching Rust by using Rust)

## Core Commands

```bash
lux auth --token <token>                       # Authenticate with Project Lighthouse
lux init <task-id> [--lang rust|go]            # Download starter code for a task
lux validate --task-id <task-uuid>             # Run validation tests for a task
lux hints --task-id <task-uuid> [--progressive] # Show progressive hints
lux doctor                                      # Check local environment
lux submit --task-id <task-uuid>               # (future) Record completion
```

### Command Details

**`lux auth --token <token>`**
- Stores authentication token locally (e.g., in `~/.config/lux/auth.json`)
- Token used for fetching tasks and submitting results
- Required before running other commands

**`lux validate --task-id <task-uuid>`**
- Looks up task in local task registry (`tasks.json`)
- Locates user's implementation in `exercises/<task-uuid>/<lang>/`
- Runs task-specific validator
- Reports test results

## Project Structure

```
lighthouse/
├── src/
│   ├── main.rs              # CLI entry point (clap)
│   ├── commands/
│   │   ├── init.rs          # Download starter code
│   │   ├── test.rs          # Run validators
│   │   ├── hints.rs         # Show hints
│   │   └── doctor.rs        # Check setup
│   ├── exercises/
│   │   ├── mod.rs
│   │   ├── registry.rs      # Exercise definitions
│   │   └── validators/
│   │       ├── tcp_echo.rs
│   │       ├── http_parser.rs
│   │       └── ...
│   ├── runner.rs            # Spawn/manage user binaries
│   └── output.rs            # Pretty test results
└── exercises/               # Starter code templates
    ├── tcp-echo-server/
    │   ├── rust/
    │   │   ├── Cargo.toml
    │   │   └── src/main.rs  # TODOs
    │   └── go/
    │       ├── go.mod
    │       └── main.go
    └── ...
```

## Key Dependencies
- `clap` - CLI argument parsing
- `tokio` - Async runtime for validators
- `colored` - Terminal output coloring
- `indicatif` - Progress bars
- `serde` / `serde_json` - Configuration
- `reqwest` - Fetch templates (optional)

## Task & Validator Architecture

### Core Design: Static Dispatch with Enums

All validators use static dispatch via enums (no `Box<dyn Trait>`). Each task UUID maps to a `Task` struct containing metadata and validators.

### Task Structure

```rust
pub struct Task {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub hints: &'static [&'static str],  // Compile-time strings
    pub validators: Vec<Validator>,
}

impl Task {
    pub fn description(&self) -> &'static str {
        self.description
    }

    pub fn hints(&self) -> &'static [&'static str] {
        self.hints
    }

    pub async fn validate(&self, context: &ValidationContext) -> Result<TestResults> {
        let mut tests = Vec::new();
        for validator in &self.validators {
            tests.push(validator.validate(context).await?);
        }
        Ok(TestResults { tests })
    }
}
```

### Validator Enum (Static Dispatch)

```rust
pub enum Validator {
    // Runtime validators (test running processes)
    Port(PortValidator),
    Endpoint(EndpointValidator),
    JsonResponse(JsonResponseValidator),
    StatusCode(StatusCodeValidator),
    TcpEcho(TcpEchoValidator),

    // Code validators (parse source files)
    CodeStructure(CodeValidator),
    FunctionExists(FunctionValidator),

    // Infrastructure validators (future)
    Docker(DockerValidator),
    Kubernetes(K8sValidator),
}

impl Validator {
    pub async fn validate(&self, context: &ValidationContext) -> Result<TestCase> {
        match self {
            Validator::Port(v) => v.validate(context).await,
            Validator::Endpoint(v) => v.validate(context).await,
            Validator::JsonResponse(v) => v.validate(context).await,
            Validator::StatusCode(v) => v.validate(context).await,
            Validator::TcpEcho(v) => v.validate(context).await,
            Validator::CodeStructure(v) => v.validate(context).await,
            Validator::FunctionExists(v) => v.validate(context).await,
            Validator::Docker(v) => v.validate(context).await,
            Validator::Kubernetes(v) => v.validate(context).await,
        }
    }
}
```

### Validation Context & Results

```rust
pub struct ValidationContext {
    pub task_id: String,
    pub language: String,
    pub project_path: PathBuf,  // e.g., exercises/<task-uuid>/rust/
}

pub struct TestResults {
    pub tests: Vec<TestCase>,
}

pub struct TestCase {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}
```

### Task Trait (Clean Interface)

Each task is a separate struct implementing the `Task` trait:

```rust
pub trait Task {
    fn new() -> Self where Self: Sized;
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn hints(&self) -> &'static [&'static str];
    fn validators(&self) -> &[ValidatorStep];

    // Default implementations
    async fn validate(&self, context: &ValidationContext) -> Result<TestResults, String> {
        let mut tests = Vec::new();
        for step in self.validators() {
            tests.push(step.validator.validate(context).await?);
        }
        Ok(TestResults { tests })
    }

    async fn validate_step(&self, step_id: &str, context: &ValidationContext) -> Result<TestCase, String> {
        let step = self.validators()
            .iter()
            .find(|s| s.id == step_id)
            .ok_or_else(|| format!("Step '{}' not found", step_id))?;

        step.validator.validate(context).await
    }
}
```

### ValidatorStep Structure

Each validator within a task has its own ID, name, and hints:

```rust
pub struct ValidatorStep {
    pub id: &'static str,
    pub name: &'static str,
    pub hints: &'static [&'static str],
    pub validator: Validator,
}
```

### Task Registry (Static Array)

Tasks are stored in a static array (no HashMap, no dynamic growth):

```rust
use once_cell::sync::Lazy;

static TASKS: Lazy<Vec<Box<dyn Task>>> = Lazy::new(|| {
    vec![
        Box::new(HttpServerTask::new()),
        Box::new(TcpEchoTask::new()),
    ]
});

pub fn get_task(id: &str) -> Option<&'static dyn Task> {
    TASKS.iter()
        .find(|t| t.id() == id)
        .map(|b| &**b as &dyn Task)
}
```

### Example Task Implementation

```rust
// src/tasks/http_server.rs
pub struct HttpServerTask {
    validators: Vec<ValidatorStep>,
}

impl Task for HttpServerTask {
    fn new() -> Self {
        Self {
            validators: vec![
                ValidatorStep {
                    id: "port-check",
                    name: "Server binds to port 8000",
                    hints: &["Use TcpListener::bind", "Bind to 127.0.0.1:8000"],
                    validator: Validator::Port(PortValidator::new(8000)),
                },
                ValidatorStep {
                    id: "hello-endpoint",
                    name: "Implements /api/v1/hello endpoint",
                    hints: &["Parse the request path", "Return JSON response"],
                    validator: Validator::Endpoint(EndpointValidator::new("/api/v1/hello")),
                },
            ],
        }
    }

    fn id(&self) -> &'static str {
        "550e8400-e29b-41d4-a716-446655440000"
    }

    fn name(&self) -> &'static str {
        "HTTP Server"
    }

    fn description(&self) -> &'static str {
        "Build an HTTP server that handles JSON API requests on port 8000"
    }

    fn hints(&self) -> &'static [&'static str] {
        &[
            "Start by binding a TCP listener to port 8000",
            "Implement the /api/v1/hello endpoint first",
            "Return proper JSON Content-Type headers",
        ]
    }

    fn validators(&self) -> &[ValidatorStep] {
        &self.validators
    }
}
```

### Validator Implementation Examples

**PortValidator** - Checks if server is listening on a port:

```rust
pub struct PortValidator {
    port: u16,
}

impl PortValidator {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub async fn validate(&self, context: &ValidationContext) -> Result<TestCase, String> {
        use tokio::net::TcpStream;
        use tokio::time::{timeout, Duration};

        // Try to connect to the port
        let addr = format!("127.0.0.1:{}", self.port);
        let result = timeout(
            Duration::from_secs(2),
            TcpStream::connect(&addr)
        ).await;

        match result {
            Ok(Ok(_)) => Ok(TestCase {
                name: format!("Server listening on port {}", self.port),
                passed: true,
                error: None,
            }),
            Ok(Err(e)) => Ok(TestCase {
                name: format!("Server listening on port {}", self.port),
                passed: false,
                error: Some(format!("Connection failed: {}", e)),
            }),
            Err(_) => Ok(TestCase {
                name: format!("Server listening on port {}", self.port),
                passed: false,
                error: Some("Connection timeout".to_string()),
            }),
        }
    }
}
```

**EndpointValidator** - Tests if an HTTP endpoint exists:

```rust
pub struct EndpointValidator {
    endpoint: String,
    port: u16,
}

impl EndpointValidator {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            port: 8000, // Default port
        }
    }

    pub async fn validate(&self, context: &ValidationContext) -> Result<TestCase, String> {
        use tokio::net::TcpStream;
        use tokio::io::{AsyncWriteExt, AsyncReadExt};

        let addr = format!("127.0.0.1:{}", self.port);
        let mut stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| format!("Failed to connect: {}", e))?;

        // Send HTTP GET request
        let request = format!(
            "GET {} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
            self.endpoint
        );

        stream.write_all(request.as_bytes())
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        // Read response
        let mut response = Vec::new();
        stream.read_to_end(&mut response)
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let response_str = String::from_utf8_lossy(&response);

        // Check for 200 OK status
        if response_str.contains("HTTP/1.1 200") || response_str.contains("HTTP/1.0 200") {
            Ok(TestCase {
                name: format!("Endpoint {} returns 200 OK", self.endpoint),
                passed: true,
                error: None,
            })
        } else {
            Ok(TestCase {
                name: format!("Endpoint {} returns 200 OK", self.endpoint),
                passed: false,
                error: Some(format!("Expected 200 OK, got: {}",
                    response_str.lines().next().unwrap_or("no response"))),
            })
        }
    }
}
```

**JsonResponseValidator** - Validates JSON Content-Type header:

```rust
pub struct JsonResponseValidator {
    endpoint: String,
    port: u16,
}

impl JsonResponseValidator {
    pub fn new() -> Self {
        Self {
            endpoint: "/api/v1/hello".to_string(),
            port: 8000,
        }
    }

    pub async fn validate(&self, context: &ValidationContext) -> Result<TestCase, String> {
        use tokio::net::TcpStream;
        use tokio::io::{AsyncWriteExt, AsyncReadExt};

        let addr = format!("127.0.0.1:{}", self.port);
        let mut stream = TcpStream::connect(&addr).await
            .map_err(|e| format!("Failed to connect: {}", e))?;

        let request = format!(
            "GET {} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
            self.endpoint
        );

        stream.write_all(request.as_bytes()).await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let response_str = String::from_utf8_lossy(&response);

        // Check for JSON content type
        let has_json_header = response_str
            .lines()
            .any(|line| line.to_lowercase().contains("content-type") &&
                        line.to_lowercase().contains("application/json"));

        if has_json_header {
            Ok(TestCase {
                name: "Response has JSON Content-Type header".to_string(),
                passed: true,
                error: None,
            })
        } else {
            Ok(TestCase {
                name: "Response has JSON Content-Type header".to_string(),
                passed: false,
                error: Some("Missing or incorrect Content-Type header".to_string()),
            })
        }
    }
}
```

## Example: TCP Echo Validator

**What it tests:**
1. Server starts and binds to port
2. Accepts connections
3. Echoes input correctly
4. Handles concurrent connections
5. Graceful shutdown

**How it works:**
- Spawn user's binary as subprocess
- Connect via tokio TcpStream
- Send test payloads
- Validate responses
- Test concurrent connections
- Kill process and verify cleanup

## Exercise Definition

```rust
pub struct Exercise {
    pub id: String,
    pub name: String,
    pub description: String,
    pub validator: Box<dyn Validator>,
    pub templates: HashMap<String, String>, // lang -> source
    pub hints: Vec<String>,
}
```

## Output Format

```
Running tests...
✓ Server starts on port 8080
✓ Accepts connections
✓ Echoes input correctly
✗ Handles concurrent connections (expected: 10, got: 1)

3/4 tests passed

Hints: lighthouse hints --progressive
```

## Distribution
- Build for Linux (x86_64), macOS (x86_64, ARM64), Windows
- GitHub Releases with pre-built binaries
- Use `cargo-dist` for automated releases
- Single binary, no dependencies

## Development Timeline

**Week 1-2:** Core CLI + test runner framework
**Week 3:** First exercise validator (TCP echo server)
**Week 4:** Add 2-3 more exercises, beta release

## First Exercise: TCP Echo Server

**Why this one first:**
- Simple concept, actually useful
- Tests networking + concurrency fundamentals
- Easy to validate programmatically
- Good learning progression

**Starter code includes:**
- TODO comments marking implementation points
- Basic structure (main, connection handling)
- Error handling skeleton
- Tests they can run locally

## Future Considerations
- Cache validated exercises locally
- Submit results to projectlighthouse.io (track progress)
- Leaderboards / completion stats
- Exercise difficulty ratings
- Community-contributed exercises
