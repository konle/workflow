use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use tracing::{debug, warn, error};

use crate::shared::workflow::TaskType;
use crate::task::entity::{HttpMethod, TaskInstanceEntity, TaskTemplate};
use crate::task::http_template_resolve::effective_http_request;
use crate::task::interface::{TaskExecutionResult, TaskExecutor};
use crate::workflow::entity::workflow_definition::NodeExecutionStatus;

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
            other => {
                error!(task_instance_id = %task_instance.task_instance_id, template = ?other, "expected Http config");
                return Err(anyhow::anyhow!("Expected Http config but got other"));
            }
        };

        let empty_ctx = json!({});
        let (input_snapshot, url, method, headers_obj, body_json) =
            effective_http_request(task_instance, config, &empty_ctx);

        if url.is_empty() {
            return Err(anyhow::anyhow!("HTTP task has empty url after resolution"));
        }

        let mut last_error: Option<String> = None;
        let attempts = config.retry_count + 1;

        for attempt in 0..attempts {
            let mut request = match method {
                HttpMethod::Get => self.client.get(&url),
                HttpMethod::Post => self.client.post(&url),
                HttpMethod::Put => self.client.put(&url),
                HttpMethod::Delete => self.client.delete(&url),
                HttpMethod::Head => self.client.head(&url),
            };

            for (hk, hv) in &headers_obj {
                let s = match hv {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Null => continue,
                    other => other.to_string(),
                };
                request = request.header(hk.as_str(), s.as_str());
            }

            if let Some(ref bj) = body_json {
                if !bj.is_null() && bj != &serde_json::Value::Object(serde_json::Map::new()) {
                    request = request.json(bj);
                }
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

                    if (200..300).contains(&status_code) {
                        debug!(
                            task_instance_id = %task_instance.task_instance_id,
                            url = %url,
                            status_code = status_code,
                            duration_ms = duration,
                            "HTTP request succeeded"
                        );
                        return Ok(TaskExecutionResult {
                            status: NodeExecutionStatus::Success,
                            input: Some(input_snapshot),
                            output: Some(output_data),
                            error_message: None,
                        });
                    } else {
                        warn!(
                            task_instance_id = %task_instance.task_instance_id,
                            url = %url,
                            status_code = status_code,
                            attempt = attempt + 1,
                            "HTTP task returned non-2xx status"
                        );
                        last_error = Some(format!("HTTP {}: {}", status_code, resp_body));
                    }
                }
                Err(e) => {
                    warn!(
                        task_instance_id = %task_instance.task_instance_id,
                        url = %url,
                        attempt = attempt + 1,
                        error = %e,
                        "HTTP request failed"
                    );
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
        error!(
            task_instance_id = %task_instance.task_instance_id,
            url = %url,
            attempts = attempts,
            error = %error_msg,
            "HTTP task failed after all retries"
        );

        Ok(TaskExecutionResult {
            status: NodeExecutionStatus::Failed,
            input: Some(input_snapshot),
            output: None,
            error_message: Some(error_msg),
        })
    }

    fn task_type(&self) -> TaskType {
        TaskType::Http
    }
}
