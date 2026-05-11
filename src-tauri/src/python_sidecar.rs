use anyhow::Result;
use log::{debug, error, info};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

/// Manages the Python Qwen3-ASR sidecar subprocess.
///
/// Spawns `python_sidecar/venv/bin/python transcriber.py` and communicates
/// via stdin/stdout with a JSON-line protocol.
pub struct PythonSidecar {
    process: Mutex<Child>,
}

impl PythonSidecar {
    /// Locate the sidecar directory relative to the executable.
    fn sidecar_dir() -> String {
        let exe = std::env::current_exe().ok();
        if let Some(path) = exe {
            if let Some(project_root) = path
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
            {
                let sidecar = project_root.join("python_sidecar");
                if sidecar.exists() {
                    return sidecar.to_string_lossy().to_string();
                }
            }
        }
        "python_sidecar".to_string()
    }

    /// Spawn the Python sidecar process.
    pub fn spawn() -> Result<Self> {
        let sidecar_dir = Self::sidecar_dir();
        let python_bin = format!("{}/venv/bin/python", sidecar_dir);
        let script = format!("{}/transcriber.py", sidecar_dir);

        info!("Starting Python sidecar: {} {}", python_bin, script);

        let mut child = Command::new(&python_bin)
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .current_dir(&sidecar_dir)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn Python sidecar: {}", e))?;

        // Wait for the "ready" signal
        let stdout = child.stdout.take().ok_or_else(|| {
            anyhow::anyhow!("Failed to capture Python sidecar stdout")
        })?;
        let mut reader = BufReader::new(stdout);
        let mut ready_line = String::new();
        reader
            .read_line(&mut ready_line)
            .map_err(|e| anyhow::anyhow!("Failed to read ready signal: {}", e))?;

        // Put the reader back as stdout
        child.stdout = Some(reader.into_inner());

        info!("Python sidecar ready: {}", ready_line.trim());
        Ok(Self {
            process: Mutex::new(child),
        })
    }

    /// Send audio data and receive transcription text.
    pub fn transcribe(&self, audio: &[f32]) -> Result<String> {
        // Encode audio as base64
        let audio_bytes: &[u8] = bytemuck::cast_slice(audio);
        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            audio_bytes,
        );

        let request = serde_json::json!({
            "audio": b64,
            "sample_rate": 16000,
        });
        let request_line = serde_json::to_string(&request)?;

        // Scope borrows so stdin/stdout don't conflict
        let response_line = {
            let mut child = self.process.lock().unwrap();

            // Send request via stdin
            let stdin = child.stdin.as_mut().ok_or_else(|| {
                anyhow::anyhow!("Python sidecar stdin not available")
            })?;
            writeln!(stdin, "{}", request_line)?;
            stdin.flush()?;
            debug!("Sent audio to Python sidecar ({} bytes)", audio.len() * 4);

            // Read response via stdout
            let stdout = child.stdout.as_mut().ok_or_else(|| {
                anyhow::anyhow!("Python sidecar stdout not available")
            })?;
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            reader.read_line(&mut line)?;
            line
        };

        // Now stdout is released, parse the response
        let response_line = response_line.trim().to_string();
        if response_line.is_empty() {
            return Err(anyhow::anyhow!("Python sidecar returned empty response"));
        }

        let response: serde_json::Value = serde_json::from_str(&response_line)?;

        if let Some(error_msg) = response.get("error") {
            let msg = error_msg.as_str().unwrap_or("unknown error");
            error!("Python sidecar error: {}", msg);
            return Err(anyhow::anyhow!("Python sidecar: {}", msg));
        }

        let text = response["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'text' in sidecar response"))?
            .to_string();

        Ok(text)
    }
}

impl Drop for PythonSidecar {
    fn drop(&mut self) {
        if let Ok(mut child) = self.process.lock() {
            let _ = child.kill();
            let _ = child.wait();
            debug!("Python sidecar process terminated");
        }
    }
}
