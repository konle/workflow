use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;

use crate::shared::workflow::TaskType;
use crate::task::entity::{HttpMethod, TaskInstanceEntity, TaskTemplate};
use crate::task::interface::{TaskExecutionResult, TaskExecutor};
use crate::workflow::entity::{NodeExecutionStatus, NodeOutput};

pub struct HttpTaskExecutor {
    client: Client,
}

impl HttpTaskExecutor {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl TaskExecutor for HttpTaskExecutor {
    async fn execute_task(
        &self,
        task_instance: &TaskInstanceEntity,
    ) -> anyhow::Result<TaskExecutionResult> {
        let config = match &task_instance.task_template {
            TaskTemplate::Http(c) => c,
            _ => return Err(anyhow::anyhow!("Expected Http config but got other")),
        };

        let mut last_error: Option<String> = None;
        let attempts = config.retry_count + 1;

        for attempt in 0..attempts {
            let mut request = match config.method {
                HttpMethod::Get => self.client.get(&config.url),
                HttpMethod::Post => self.client.post(&config.url),
                HttpMethod::Put => self.client.put(&config.url),
                HttpMethod::Delete => self.client.delete(&config.url),
                HttpMethod::Head => self.client.head(&config.url),
            };

            for (k, v) in &config.headers {
                request = request.header(k.as_str(), v.as_str());
            }

            if let Some(ref form) = config.body {
                request = request.json(form);
            }

            if config.timeout > 0 {
                request = request.timeout(std::time::Duration::from_secs(config.timeout as u64));
            }

            let start = Utc::now();
            match request.send().await {
                Ok(resp) => {
                    let status_code = resp.status().as_u16();
                    let resp_body = resp.text().await.unwrap_or_default();
                    let duration = (Utc::now() - start).num_milliseconds().max(0) as u64;

                    let output_data = json!({
                        "status_code": status_code,
                        "body": resp_body,
                        "duration_ms": duration,
                        "attempt": attempt + 1,
                    });

                    let output = Some(NodeOutput { data: output_data });

                    if (200..300).contains(&status_code) {
                        return Ok(TaskExecutionResult {
                            status: NodeExecutionStatus::Success,
                            output,
                            error_message: None,
                        });
                    } else {
                        last_error = Some(format!("HTTP {}: {}", status_code, resp_body));
                    }
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                }
            }

            if attempt < config.retry_count {
                let delay = config.retry_delay as u64;
                if delay > 0 {
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                }
            }
        }

        let error_msg = last_error.unwrap_or_else(|| "Unknown error".to_string());
        
        Ok(TaskExecutionResult {
            status: NodeExecutionStatus::Failed,
            output: None,
            error_message: Some(error_msg),
        })
    }

    fn task_type(&self) -> TaskType {
        TaskType::Http
    }
}