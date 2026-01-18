//! Docker-based validators
//!
//! Dockerfiles are fetched from GitHub at runtime, allowing updates
//! without releasing a new CLI version.

mod executor;
mod validator;

pub use executor::{is_docker_available, DockerExecutor, ExecutorResult};
pub use validator::{DockerValidator, Expectation};
