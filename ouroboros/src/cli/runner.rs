use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Model {
    Opus,
    Sonnet,
    Haiku,
}

impl Model {
    pub fn as_str(&self) -> &'static str {
        match self {
            Model::Opus => "opus",
            Model::Sonnet => "sonnet",
            Model::Haiku => "haiku",
        }
    }
}

impl Default for Model {
    fn default() -> Self {
        Model::Sonnet
    }
}

#[derive(Debug, Clone, Default)]
pub struct CliOptions {
    pub model: Model,
    pub print: bool,
    pub resume: Option<String>,
    pub system_prompt: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    /// Skip permission prompts (dangerous but needed for automation)
    pub skip_permissions: bool,
}

#[derive(Debug, Clone)]
pub struct CliOutput {
    pub session_id: Option<String>,
    pub response: String,
    pub exit_code: i32,
}

#[derive(Clone)]
pub struct CliRunner {
    claude_path: PathBuf,
    working_dir: PathBuf,
}

impl CliRunner {
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            claude_path: PathBuf::from("claude"),
            working_dir,
        }
    }

    pub fn with_claude_path(mut self, path: PathBuf) -> Self {
        self.claude_path = path;
        self
    }

    /// Run claude CLI with given prompt and options
    pub async fn run(&self, prompt: &str, options: CliOptions) -> Result<CliOutput> {
        let mut cmd = Command::new(&self.claude_path);
        cmd.current_dir(&self.working_dir);

        // Add options
        if options.print {
            cmd.arg("--print");
        }

        // Always skip permissions for automation
        if options.skip_permissions {
            cmd.arg("--dangerously-skip-permissions");
        }

        cmd.arg("--model").arg(options.model.as_str());

        if let Some(ref session_id) = options.resume {
            cmd.arg("--resume").arg(session_id);
        }

        if let Some(ref system_prompt) = options.system_prompt {
            cmd.arg("--system-prompt").arg(system_prompt);
        }

        if let Some(ref tools) = options.allowed_tools {
            for tool in tools {
                cmd.arg("--allowedTools").arg(tool);
            }
        }

        // Add prompt as positional argument (must be last)
        cmd.arg(prompt);

        // Capture output and close stdin to prevent blocking
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        tracing::debug!("Running claude CLI: {:?}", cmd);

        let output = cmd
            .output()
            .await
            .context("Failed to execute claude CLI")?;

        let response = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !stderr.is_empty() {
            tracing::warn!("claude stderr: {}", stderr);
        }

        // Try to extract session ID from output (if available)
        let session_id = self.extract_session_id(&response);

        Ok(CliOutput {
            session_id,
            response,
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    /// Resume a previous session
    pub async fn resume(&self, session_id: &str, prompt: &str) -> Result<CliOutput> {
        let options = CliOptions {
            print: true,
            resume: Some(session_id.to_string()),
            ..Default::default()
        };
        self.run(prompt, options).await
    }

    /// Fork a session (create new branch from existing session)
    pub async fn fork(&self, session_id: &str, prompt: &str) -> Result<CliOutput> {
        self.fork_with_options(session_id, prompt, CliOptions::default()).await
    }

    /// Fork with custom options
    /// Note: Fork uses -p for non-interactive mode, NOT --print
    /// --print interferes with --fork-session
    pub async fn fork_with_options(
        &self,
        session_id: &str,
        prompt: &str,
        base_options: CliOptions,
    ) -> Result<CliOutput> {
        let mut cmd = Command::new(&self.claude_path);
        cmd.current_dir(&self.working_dir);

        // Fork requires --resume + --fork-session
        cmd.arg("--resume").arg(session_id);
        cmd.arg("--fork-session");

        // Don't use --print with fork, just -p for non-interactive
        cmd.arg("--model").arg(base_options.model.as_str());

        if let Some(ref system_prompt) = base_options.system_prompt {
            cmd.arg("--system-prompt").arg(system_prompt);
        }

        // -p flag makes it non-interactive for fork
        cmd.arg("-p").arg(prompt);

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        tracing::debug!("Forking session {}: {:?}", session_id, cmd);

        let output = cmd
            .output()
            .await
            .context("Failed to fork claude session")?;

        let response = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !stderr.is_empty() {
            tracing::warn!("claude fork stderr: {}", stderr);
        }

        let new_session_id = self.extract_session_id(&response);

        Ok(CliOutput {
            session_id: new_session_id,
            response,
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    /// Fork with task tag (for DAG workflow)
    pub async fn fork_for_task(
        &self,
        session_id: &str,
        task_id: &str,
        task_name: &str,
    ) -> Result<CliOutput> {
        let prompt = format!("[TASK:{}] {}", &task_id[..8.min(task_id.len())], task_name);
        self.fork_with_options(session_id, &prompt, CliOptions::default()).await
    }

    /// Run with specific model for validation
    pub async fn validate(&self, prompt: &str) -> Result<CliOutput> {
        let options = CliOptions {
            model: Model::Opus,
            print: true,
            ..Default::default()
        };
        self.run(prompt, options).await
    }

    /// Run with Haiku for extraction tasks
    pub async fn extract(&self, prompt: &str) -> Result<CliOutput> {
        let options = CliOptions {
            model: Model::Haiku,
            print: true,
            ..Default::default()
        };
        self.run(prompt, options).await
    }

    fn extract_session_id(&self, _output: &str) -> Option<String> {
        // TODO: Parse session ID from claude output
        // The format depends on how claude CLI reports session info
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_as_str() {
        assert_eq!(Model::Opus.as_str(), "opus");
        assert_eq!(Model::Sonnet.as_str(), "sonnet");
        assert_eq!(Model::Haiku.as_str(), "haiku");
    }
}
