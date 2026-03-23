use std::{
    io::{BufRead, BufReader},
    net::SocketAddr,
    process::{Child, Command, Stdio},
    thread::{self, JoinHandle},
};

pub struct Setup {
    container_id: String,
    pub base_url: String,
    logs_process: Child,
    logs_stdout: Option<JoinHandle<()>>,
    logs_stderr: Option<JoinHandle<()>>,
}

impl Drop for Setup {
    fn drop(&mut self) {
        // Cleanup
        Command::new("docker")
            .args(["rm", "-f", &self.container_id])
            .status()
            .unwrap();

        let _ = self.logs_process.kill();
        let _ = self.logs_process.wait();

        if let Some(handle) = self.logs_stdout.take() {
            handle.join().unwrap()
        }

        if let Some(handle) = self.logs_stderr.take() {
            handle.join().unwrap()
        }
    }
}

pub fn setup() -> Setup {
    // Run container
    let output = Command::new("docker")
        .args(["run", "-d", "-p", "0:8080", "galactic-exchange:test"])
        .output()
        .expect("failed to execute docker cmd");

    assert!(output.status.success(), "run failed");

    let container_id = String::from_utf8(output.stdout).unwrap().trim().to_string();

    let output = Command::new("docker")
        .args(["port", &container_id, "8080"])
        .output()
        .unwrap();

    let output_string = String::from_utf8(output.stdout).unwrap().trim().to_string();
    let first = output_string.lines().next().unwrap();

    let addr: SocketAddr = first.parse().unwrap();
    let port = addr.port();

    // Watch for logs
    let mut logs = Command::new("docker")
        .args(["logs", "-f", &container_id])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start docker logs");

    // Forward stdout in a thread
    let stdout = logs.stdout.take().unwrap();
    let stdout_thread = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let line = line.unwrap();
            println!("{}", line); // goes through Cargo, buffered
        }
    });

    // Forward stderr in a thread
    let stderr = logs.stderr.take().unwrap();
    let stderr_thread = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            let line = line.unwrap();
            println!("{}", line); // goes through Cargo, buffered
        }
    });

    Setup {
        container_id,
        base_url: format!("http://localhost:{port}"),
        logs_process: logs,
        logs_stdout: Some(stdout_thread),
        logs_stderr: Some(stderr_thread),
    }
}
