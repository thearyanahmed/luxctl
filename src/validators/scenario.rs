use crate::tasks::TestCase;
use super::http::http_request;
use serde_json::Value as JsonValue;
use tokio::time::{sleep, Duration};

const DEFAULT_PORT: u16 = 8080;

/// Helper to extract a field from JSON, supporting nested paths like "workers.total"
fn get_nested_field<'a>(json: &'a JsonValue, path: &str) -> Option<&'a JsonValue> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = json;
    for part in parts {
        current = current.get(part)?;
    }
    Some(current)
}

/// Helper to convert JSON value to string for comparison
fn json_value_to_string(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Null => "null".to_string(),
        _ => value.to_string(),
    }
}

/// Scenario: Submit a job and verify it was stored
/// 1. POST /jobs with payload
/// 2. Extract job_id from response
/// 3. GET /jobs/{id}
/// 4. Verify job exists with correct data
pub struct JobSubmissionVerified {
    pub port: u16,
    pub job_type: String,
    pub payload: String,
}

impl JobSubmissionVerified {
    pub fn new(job_type: &str, payload: &str) -> Self {
        Self {
            port: DEFAULT_PORT,
            job_type: job_type.to_string(),
            payload: payload.to_string(),
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        // step 1: POST job
        let body = format!(
            r#"{{"type":"{}","payload":"{}"}}"#,
            self.job_type, self.payload
        );
        let headers = [("Content-Type", "application/json")];
        let post_response = http_request(self.port, "POST", "/jobs", &headers, Some(&body)).await?;

        if post_response.status_code != 201 {
            return Ok(TestCase {
                name: "job submission verified".to_string(),
                result: Err(format!(
                    "POST /jobs expected 201, got {}",
                    post_response.status_code
                )),
            });
        }

        // step 2: extract job_id
        let json: JsonValue = serde_json::from_str(&post_response.body)
            .map_err(|e| format!("invalid JSON in POST response: {}", e))?;

        let job_id = json
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("POST response missing 'id' field")?;

        // step 3: GET /jobs/{id}
        let get_path = format!("/jobs/{}", job_id);
        let get_response = http_request(self.port, "GET", &get_path, &[], None).await?;

        if get_response.status_code != 200 {
            return Ok(TestCase {
                name: "job submission verified".to_string(),
                result: Err(format!(
                    "GET {} expected 200, got {} - job not stored",
                    get_path, get_response.status_code
                )),
            });
        }

        // step 4: verify job data
        let get_json: JsonValue = serde_json::from_str(&get_response.body)
            .map_err(|e| format!("invalid JSON in GET response: {}", e))?;

        let stored_id = get_json
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if stored_id != job_id {
            return Ok(TestCase {
                name: "job submission verified".to_string(),
                result: Err(format!(
                    "stored job id '{}' doesn't match submitted '{}'",
                    stored_id, job_id
                )),
            });
        }

        Ok(TestCase {
            name: "job submission verified".to_string(),
            result: Ok(format!(
                "job {} submitted and verified in storage",
                job_id
            )),
        })
    }
}

/// Scenario: Submit a job and verify it gets processed
/// 1. POST /jobs
/// 2. Wait for processing
/// 3. GET /jobs/{id}
/// 4. Verify status changed to expected value
pub struct JobProcessingVerified {
    pub port: u16,
    pub job_type: String,
    pub payload: String,
    pub wait_ms: u64,
    pub expected_status: String,
}

impl JobProcessingVerified {
    pub fn new(wait_ms: u64, expected_status: &str) -> Self {
        Self {
            port: DEFAULT_PORT,
            job_type: "test".to_string(),
            payload: "data".to_string(),
            wait_ms,
            expected_status: expected_status.to_string(),
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        // step 1: POST job
        let body = format!(
            r#"{{"type":"{}","payload":"{}"}}"#,
            self.job_type, self.payload
        );
        let headers = [("Content-Type", "application/json")];
        let post_response = http_request(self.port, "POST", "/jobs", &headers, Some(&body)).await?;

        if post_response.status_code != 201 {
            return Ok(TestCase {
                name: "job processing verified".to_string(),
                result: Err(format!(
                    "POST /jobs expected 201, got {}",
                    post_response.status_code
                )),
            });
        }

        let json: JsonValue = serde_json::from_str(&post_response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;

        let job_id = json
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("missing job id")?;

        // step 2: wait for processing
        sleep(Duration::from_millis(self.wait_ms)).await;

        // step 3: GET job status
        let get_path = format!("/jobs/{}", job_id);
        let get_response = http_request(self.port, "GET", &get_path, &[], None).await?;

        if get_response.status_code != 200 {
            return Ok(TestCase {
                name: "job processing verified".to_string(),
                result: Err(format!(
                    "GET {} returned {}",
                    get_path, get_response.status_code
                )),
            });
        }

        let get_json: JsonValue = serde_json::from_str(&get_response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;

        let status = get_json
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        // step 4: verify status
        let result = if status == self.expected_status {
            Ok(format!(
                "job {} processed, status: {}",
                job_id, status
            ))
        } else {
            Err(format!(
                "expected status '{}', got '{}'",
                self.expected_status, status
            ))
        };

        Ok(TestCase {
            name: format!("job processing → {}", self.expected_status),
            result,
        })
    }
}

/// Scenario: Verify multiple workers process jobs concurrently
/// 1. Submit N jobs simultaneously
/// 2. Immediately poll job statuses
/// 3. Verify multiple jobs are in "processing" state at same time
pub struct WorkerPoolConcurrent {
    pub port: u16,
    pub worker_count: u32,
    pub job_count: u32,
    pub job_duration_ms: u64,
    pub max_total_ms: u64,
}

impl WorkerPoolConcurrent {
    pub fn new(worker_count: u32, job_count: u32, max_total_ms: u64) -> Self {
        Self {
            port: DEFAULT_PORT,
            worker_count,
            job_count,
            job_duration_ms: 500, // jobs should take this long
            max_total_ms,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let start = std::time::Instant::now();
        let mut job_ids = Vec::new();

        // step 1: submit all jobs simultaneously
        let mut handles = Vec::new();
        for i in 0..self.job_count {
            let port = self.port;
            let duration = self.job_duration_ms;
            let handle = tokio::spawn(async move {
                let body = format!(
                    r#"{{"type":"sleep","payload":"{}","duration_ms":{}}}"#,
                    i, duration
                );
                let headers = [("Content-Type", "application/json")];
                http_request(port, "POST", "/jobs", &headers, Some(&body)).await
            });
            handles.push(handle);
        }

        // collect job IDs
        for handle in handles {
            if let Ok(Ok(response)) = handle.await {
                if let Ok(json) = serde_json::from_str::<JsonValue>(&response.body) {
                    if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
                        job_ids.push(id.to_string());
                    }
                }
            }
        }

        if job_ids.len() != self.job_count as usize {
            return Ok(TestCase {
                name: "worker pool concurrent".to_string(),
                result: Err(format!(
                    "only {} of {} jobs submitted successfully",
                    job_ids.len(),
                    self.job_count
                )),
            });
        }

        // step 2: immediately check how many are processing
        sleep(Duration::from_millis(50)).await; // tiny delay for jobs to start

        let mut processing_count = 0;
        for job_id in &job_ids {
            let get_path = format!("/jobs/{}", job_id);
            if let Ok(response) = http_request(self.port, "GET", &get_path, &[], None).await {
                if let Ok(json) = serde_json::from_str::<JsonValue>(&response.body) {
                    if let Some(status) = json.get("status").and_then(|v| v.as_str()) {
                        if status == "processing" {
                            processing_count += 1;
                        }
                    }
                }
            }
        }

        // step 3: wait for all to complete
        sleep(Duration::from_millis(self.job_duration_ms + 100)).await;

        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;

        // step 4: verify concurrency
        // if workers are concurrent, total time should be ~job_duration, not job_duration * job_count
        let result = if processing_count >= 2 {
            if elapsed_ms <= self.max_total_ms {
                Ok(format!(
                    "concurrent processing confirmed: {} jobs processing simultaneously, completed in {}ms",
                    processing_count, elapsed_ms
                ))
            } else {
                Err(format!(
                    "jobs processed but took {}ms (max allowed: {}ms) - workers may not be concurrent",
                    elapsed_ms, self.max_total_ms
                ))
            }
        } else {
            Err(format!(
                "only {} job(s) processing at same time - expected concurrent processing with {} workers",
                processing_count, self.worker_count
            ))
        };

        Ok(TestCase {
            name: format!("{} workers processing {} jobs", self.worker_count, self.job_count),
            result,
        })
    }
}

/// Scenario: Test job results are stored correctly
/// 1. POST job with specific type and payload
/// 2. Wait for processing
/// 3. GET job and verify result field
pub struct JobResultVerified {
    pub port: u16,
    pub job_type: String,
    pub payload: String,
    pub expected_result: String,
}

impl JobResultVerified {
    pub fn new(job_type: &str, payload: &str, expected_result: &str) -> Self {
        Self {
            port: DEFAULT_PORT,
            job_type: job_type.to_string(),
            payload: payload.to_string(),
            expected_result: expected_result.to_string(),
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        // step 1: POST job
        let body = format!(
            r#"{{"type":"{}","payload":"{}"}}"#,
            self.job_type, self.payload
        );
        let headers = [("Content-Type", "application/json")];
        let post_response = http_request(self.port, "POST", "/jobs", &headers, Some(&body)).await?;

        if post_response.status_code != 201 {
            return Ok(TestCase {
                name: format!("job result: {}", self.job_type),
                result: Err(format!("POST failed with {}", post_response.status_code)),
            });
        }

        let json: JsonValue = serde_json::from_str(&post_response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;
        let job_id = json.get("id").and_then(|v| v.as_str()).ok_or("missing id")?;

        // step 2: wait for processing
        sleep(Duration::from_millis(200)).await;

        // step 3: GET job and check result
        let get_path = format!("/jobs/{}", job_id);
        let get_response = http_request(self.port, "GET", &get_path, &[], None).await?;

        let get_json: JsonValue = serde_json::from_str(&get_response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;

        let result_value = get_json
            .get("result")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let result = if result_value == self.expected_result {
            Ok(format!(
                "job type '{}' with payload '{}' returned '{}'",
                self.job_type, self.payload, result_value
            ))
        } else {
            Err(format!(
                "expected result '{}', got '{}'",
                self.expected_result, result_value
            ))
        };

        Ok(TestCase {
            name: format!("job result: {} → {}", self.job_type, self.expected_result),
            result,
        })
    }
}

/// Scenario: Test priority queue ordering
/// 1. POST low priority job
/// 2. POST high priority job
/// 3. Verify high priority completes first
pub struct JobPriorityVerified {
    pub port: u16,
    pub high_priority: u32,
    pub low_priority: u32,
}

impl JobPriorityVerified {
    pub fn new(high_priority: u32, low_priority: u32) -> Self {
        Self {
            port: DEFAULT_PORT,
            high_priority,
            low_priority,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        // step 1: POST low priority job first
        let low_body = format!(
            r#"{{"type":"sleep","payload":"low","priority":{},"duration_ms":100}}"#,
            self.low_priority
        );
        let headers = [("Content-Type", "application/json")];
        let low_response = http_request(self.port, "POST", "/jobs", &headers, Some(&low_body)).await?;

        let low_json: JsonValue = serde_json::from_str(&low_response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;
        let low_id = low_json.get("id").and_then(|v| v.as_str()).ok_or("missing id")?;

        // small delay to ensure order
        sleep(Duration::from_millis(10)).await;

        // step 2: POST high priority job second
        let high_body = format!(
            r#"{{"type":"sleep","payload":"high","priority":{},"duration_ms":100}}"#,
            self.high_priority
        );
        let high_response = http_request(self.port, "POST", "/jobs", &headers, Some(&high_body)).await?;

        let high_json: JsonValue = serde_json::from_str(&high_response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;
        let high_id = high_json.get("id").and_then(|v| v.as_str()).ok_or("missing id")?;

        // step 3: wait for both to complete
        sleep(Duration::from_millis(500)).await;

        // step 4: check completion order via completed_at timestamps
        let low_path = format!("/jobs/{}", low_id);
        let high_path = format!("/jobs/{}", high_id);

        let low_get = http_request(self.port, "GET", &low_path, &[], None).await?;
        let high_get = http_request(self.port, "GET", &high_path, &[], None).await?;

        let low_data: JsonValue = serde_json::from_str(&low_get.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;
        let high_data: JsonValue = serde_json::from_str(&high_get.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;

        // compare completed_at timestamps or check processing order
        let low_completed = low_data.get("completed_at").and_then(|v| v.as_str());
        let high_completed = high_data.get("completed_at").and_then(|v| v.as_str());

        let result = match (low_completed, high_completed) {
            (Some(low_time), Some(high_time)) => {
                if high_time < low_time {
                    Ok(format!(
                        "priority {} job completed before priority {} job",
                        self.high_priority, self.low_priority
                    ))
                } else {
                    Err(format!(
                        "low priority job completed at {}, high at {} - priority not respected",
                        low_time, high_time
                    ))
                }
            }
            _ => Err("jobs missing completed_at timestamp".to_string()),
        };

        Ok(TestCase {
            name: format!("priority {} before {}", self.high_priority, self.low_priority),
            result,
        })
    }
}

/// Scenario: Test job timeout behavior
/// 1. POST a slow job
/// 2. Wait for timeout
/// 3. Verify job status is "failed" with reason "timeout"
pub struct JobTimeoutVerified {
    pub port: u16,
    pub job_duration_ms: u64,
    pub expected_status: String,
}

impl JobTimeoutVerified {
    pub fn new(job_duration_ms: u64, expected_status: &str) -> Self {
        Self {
            port: DEFAULT_PORT,
            job_duration_ms,
            expected_status: expected_status.to_string(),
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        // step 1: POST slow job
        let body = format!(
            r#"{{"type":"sleep","payload":"slow","duration_ms":{}}}"#,
            self.job_duration_ms
        );
        let headers = [("Content-Type", "application/json")];
        let post_response = http_request(self.port, "POST", "/jobs", &headers, Some(&body)).await?;

        let json: JsonValue = serde_json::from_str(&post_response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;
        let job_id = json.get("id").and_then(|v| v.as_str()).ok_or("missing id")?;

        // step 2: wait for timeout to occur (server timeout + buffer)
        sleep(Duration::from_millis(2000)).await;

        // step 3: GET job status
        let get_path = format!("/jobs/{}", job_id);
        let get_response = http_request(self.port, "GET", &get_path, &[], None).await?;

        let get_json: JsonValue = serde_json::from_str(&get_response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;

        let status = get_json.get("status").and_then(|v| v.as_str()).unwrap_or("");

        let result = if status == self.expected_status {
            Ok(format!("slow job timed out correctly, status: {}", status))
        } else {
            Err(format!(
                "expected status '{}', got '{}'",
                self.expected_status, status
            ))
        };

        Ok(TestCase {
            name: "job timeout".to_string(),
            result,
        })
    }
}

/// Scenario: Verify timeout reason
pub struct JobTimeoutReasonVerified {
    pub port: u16,
    pub expected_reason: String,
}

impl JobTimeoutReasonVerified {
    pub fn new(expected_reason: &str) -> Self {
        Self {
            port: DEFAULT_PORT,
            expected_reason: expected_reason.to_string(),
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        // POST slow job
        let body = r#"{"type":"sleep","payload":"slow","duration_ms":5000}"#;
        let headers = [("Content-Type", "application/json")];
        let post_response = http_request(self.port, "POST", "/jobs", &headers, Some(body)).await?;

        let json: JsonValue = serde_json::from_str(&post_response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;
        let job_id = json.get("id").and_then(|v| v.as_str()).ok_or("missing id")?;

        // wait for timeout
        sleep(Duration::from_millis(2000)).await;

        // GET job
        let get_path = format!("/jobs/{}", job_id);
        let get_response = http_request(self.port, "GET", &get_path, &[], None).await?;

        let get_json: JsonValue = serde_json::from_str(&get_response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;

        let reason = get_json
            .get("error")
            .or_else(|| get_json.get("failure_reason"))
            .or_else(|| get_json.get("reason"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let result = if reason.to_lowercase().contains(&self.expected_reason.to_lowercase()) {
            Ok(format!("timeout reason correctly set: {}", reason))
        } else {
            Err(format!(
                "expected reason containing '{}', got '{}'",
                self.expected_reason, reason
            ))
        };

        Ok(TestCase {
            name: "job timeout reason".to_string(),
            result,
        })
    }
}

/// Scenario: Test retry mechanism
pub struct JobRetryVerified {
    pub port: u16,
    pub job_type: String,
    pub max_retries: u32,
}

impl JobRetryVerified {
    pub fn new(job_type: &str, max_retries: u32) -> Self {
        Self {
            port: DEFAULT_PORT,
            job_type: job_type.to_string(),
            max_retries,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        // POST flaky job
        let body = format!(
            r#"{{"type":"{}","payload":"test","max_retries":{}}}"#,
            self.job_type, self.max_retries
        );
        let headers = [("Content-Type", "application/json")];
        let post_response = http_request(self.port, "POST", "/jobs", &headers, Some(&body)).await?;

        let json: JsonValue = serde_json::from_str(&post_response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;
        let job_id = json.get("id").and_then(|v| v.as_str()).ok_or("missing id")?;

        // wait for retries
        sleep(Duration::from_millis(5000)).await;

        // GET job
        let get_path = format!("/jobs/{}", job_id);
        let get_response = http_request(self.port, "GET", &get_path, &[], None).await?;

        let get_json: JsonValue = serde_json::from_str(&get_response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;

        let retries = get_json
            .get("retries")
            .or_else(|| get_json.get("retry_count"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let result = if retries > 0 {
            Ok(format!("job retry tracked: {} retries", retries))
        } else {
            Err("job retries not tracked - expected retries > 0".to_string())
        };

        Ok(TestCase {
            name: "job retry tracking".to_string(),
            result,
        })
    }
}

/// Scenario: Worker scale up under load
pub struct WorkerScaleUp {
    pub port: u16,
    pub initial_workers: u32,
    pub job_count: u32,
    pub expected_min_workers: u32,
}

impl WorkerScaleUp {
    pub fn new(initial_workers: u32, job_count: u32, expected_min_workers: u32) -> Self {
        Self {
            port: DEFAULT_PORT,
            initial_workers,
            job_count,
            expected_min_workers,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        // step 1: check initial worker count
        let initial_response = http_request(self.port, "GET", "/workers", &[], None).await?;
        let initial_json: JsonValue = serde_json::from_str(&initial_response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;

        let initial_count = initial_json
            .get("count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        // step 2: submit many jobs to trigger scale up
        for i in 0..self.job_count {
            let body = format!(r#"{{"type":"sleep","payload":"{}","duration_ms":2000}}"#, i);
            let headers = [("Content-Type", "application/json")];
            let _ = http_request(self.port, "POST", "/jobs", &headers, Some(&body)).await;
        }

        // step 3: wait for auto-scaling
        sleep(Duration::from_millis(1000)).await;

        // step 4: check worker count increased
        let final_response = http_request(self.port, "GET", "/workers", &[], None).await?;
        let final_json: JsonValue = serde_json::from_str(&final_response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;

        let final_count = final_json
            .get("count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let result = if final_count >= self.expected_min_workers {
            Ok(format!(
                "workers scaled from {} to {} (expected >= {})",
                initial_count, final_count, self.expected_min_workers
            ))
        } else {
            Err(format!(
                "workers at {} (expected >= {})",
                final_count, self.expected_min_workers
            ))
        };

        Ok(TestCase {
            name: "worker scale up".to_string(),
            result,
        })
    }
}

/// Scenario: Worker scale down when idle
pub struct WorkerScaleDown {
    pub port: u16,
    pub initial_workers: u32,
    pub expected_max_workers: u32,
}

impl WorkerScaleDown {
    pub fn new(initial_workers: u32, expected_max_workers: u32) -> Self {
        Self {
            port: DEFAULT_PORT,
            initial_workers,
            expected_max_workers,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        // step 1: manually scale to high worker count
        let scale_path = format!("/workers/scale?count={}", self.initial_workers);
        let _ = http_request(self.port, "POST", &scale_path, &[], None).await;

        // step 2: wait for scale down (no jobs)
        sleep(Duration::from_millis(3000)).await;

        // step 3: check worker count decreased
        let response = http_request(self.port, "GET", "/workers", &[], None).await?;
        let json: JsonValue = serde_json::from_str(&response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;

        let count = json.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        let result = if count <= self.expected_max_workers {
            Ok(format!(
                "workers scaled down to {} (expected <= {})",
                count, self.expected_max_workers
            ))
        } else {
            Err(format!(
                "workers still at {} (expected <= {})",
                count, self.expected_max_workers
            ))
        };

        Ok(TestCase {
            name: "worker scale down".to_string(),
            result,
        })
    }
}

/// HTTP request with body support (enhanced)
pub struct HttpRequestWithBody {
    pub port: u16,
    pub method: String,
    pub path: String,
    pub body: Option<String>,
    pub expected_status: u16,
}

impl HttpRequestWithBody {
    pub fn new(method: &str, path: &str, body: Option<&str>, expected_status: u16) -> Self {
        Self {
            port: DEFAULT_PORT,
            method: method.to_string(),
            path: path.to_string(),
            body: body.map(|s| s.to_string()),
            expected_status,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let headers = if self.body.is_some() {
            vec![("Content-Type", "application/json")]
        } else {
            vec![]
        };

        let response = http_request(
            self.port,
            &self.method,
            &self.path,
            &headers.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
            self.body.as_deref(),
        )
        .await?;

        let result = if response.status_code == self.expected_status {
            Ok(format!(
                "{} {} returned {}",
                self.method, self.path, self.expected_status
            ))
        } else {
            Err(format!(
                "expected {}, got {}",
                self.expected_status, response.status_code
            ))
        };

        Ok(TestCase {
            name: format!("{} {} → {}", self.method, self.path, self.expected_status),
            result,
        })
    }
}

/// Check nested JSON field exists
pub struct HttpJsonFieldNested {
    pub port: u16,
    pub path: String,
    pub field_path: String,
}

impl HttpJsonFieldNested {
    pub fn new(path: &str, field_path: &str) -> Self {
        Self {
            port: DEFAULT_PORT,
            path: path.to_string(),
            field_path: field_path.to_string(),
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let response = http_request(self.port, "GET", &self.path, &[], None).await?;

        let json: JsonValue = serde_json::from_str(&response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;

        let field = get_nested_field(&json, &self.field_path);

        let result = if field.is_some() {
            Ok(format!("field '{}' exists", self.field_path))
        } else {
            Err(format!("field '{}' not found", self.field_path))
        };

        Ok(TestCase {
            name: format!("JSON field: {}", self.field_path),
            result,
        })
    }
}

/// Simple HTTP health check - GET path and verify status + JSON field value
pub struct HttpHealthCheck {
    pub port: u16,
    pub path: String,
    pub expected_status: u16,
    pub expected_field: String,
    pub expected_value: String,
}

impl HttpHealthCheck {
    pub fn new(path: &str, expected_status: u16, field: &str, value: &str) -> Self {
        Self {
            port: DEFAULT_PORT,
            path: path.to_string(),
            expected_status,
            expected_field: field.to_string(),
            expected_value: value.to_string(),
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let response = http_request(self.port, "GET", &self.path, &[], None).await?;

        if response.status_code != self.expected_status {
            return Ok(TestCase {
                name: format!("GET {} → {}", self.path, self.expected_status),
                result: Err(format!(
                    "expected status {}, got {}",
                    self.expected_status, response.status_code
                )),
            });
        }

        let json: JsonValue = serde_json::from_str(&response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;

        let actual = json
            .get(&self.expected_field)
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let result = if actual == self.expected_value {
            Ok(format!(
                "GET {} returned {} with {}={}",
                self.path, self.expected_status, self.expected_field, self.expected_value
            ))
        } else {
            Err(format!(
                "expected {}='{}', got '{}'",
                self.expected_field, self.expected_value, actual
            ))
        };

        Ok(TestCase {
            name: format!("GET {} → {} ({}={})", self.path, self.expected_status, self.expected_field, self.expected_value),
            result,
        })
    }
}

/// HTTP GET with JSON field value check (any path, port 8080)
pub struct HttpJsonFieldValue {
    pub port: u16,
    pub path: String,
    pub field: String,
    pub expected_value: String,
}

impl HttpJsonFieldValue {
    pub fn new(path: &str, field: &str, expected_value: &str) -> Self {
        Self {
            port: DEFAULT_PORT,
            path: path.to_string(),
            field: field.to_string(),
            expected_value: expected_value.to_string(),
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let response = http_request(self.port, "GET", &self.path, &[], None).await?;

        let json: JsonValue = serde_json::from_str(&response.body)
            .map_err(|e| format!("invalid JSON: {}", e))?;

        let actual = get_nested_field(&json, &self.field);
        let actual_str = actual.map(json_value_to_string).unwrap_or_default();

        let result = if actual_str == self.expected_value {
            Ok(format!("field '{}' = '{}'", self.field, self.expected_value))
        } else {
            Err(format!(
                "field '{}' expected '{}', got '{}'",
                self.field, self.expected_value, actual_str
            ))
        };

        Ok(TestCase {
            name: format!("GET {} field '{}' = '{}'", self.path, self.field, self.expected_value),
            result,
        })
    }
}

/// HTTP GET and check status code
pub struct HttpStatusCheck {
    pub port: u16,
    pub path: String,
    pub expected_status: u16,
}

impl HttpStatusCheck {
    pub fn new(path: &str, expected_status: u16) -> Self {
        Self {
            port: DEFAULT_PORT,
            path: path.to_string(),
            expected_status,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let response = http_request(self.port, "GET", &self.path, &[], None).await?;

        let result = if response.status_code == self.expected_status {
            Ok(format!("GET {} returned {}", self.path, self.expected_status))
        } else {
            Err(format!(
                "expected status {}, got {}",
                self.expected_status, response.status_code
            ))
        };

        Ok(TestCase {
            name: format!("GET {} → {}", self.path, self.expected_status),
            result,
        })
    }
}
