/// TestResults aggregates all test cases for a task
#[derive(Debug)]
pub struct TestResults {
    pub tests: Vec<TestCase>,
}

impl TestResults {
    pub fn new() -> Self {
        Self { tests: Vec::new() }
    }

    pub fn add(&mut self, test: TestCase) {
        self.tests.push(test);
    }

    pub fn passed(&self) -> usize {
        self.tests.iter().filter(|t| t.passed()).count()
    }

    pub fn failed(&self) -> usize {
        self.tests.iter().filter(|t| !t.passed()).count()
    }

    pub fn total(&self) -> usize {
        self.tests.len()
    }

    pub fn all_passed(&self) -> bool {
        self.tests.iter().all(|t| t.passed())
    }
}

impl Default for TestResults {
    fn default() -> Self {
        Self::new()
    }
}

/// TestCase represents a single validation test result
#[derive(Debug)]
pub struct TestCase {
    pub name: String,
    pub result: Result<String, String>, // Ok(success_msg) or Err(error_msg)
}

impl TestCase {
    pub fn passed(&self) -> bool {
        self.result.is_ok()
    }

    pub fn message(&self) -> &str {
        match &self.result {
            Ok(msg) => msg,
            Err(msg) => msg,
        }
    }
}
