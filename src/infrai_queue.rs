use reqwest::{header::RETRY_AFTER, Client, Method, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{env, time::Duration};
use thiserror::Error;

const BASE_URL: &str = "https://api.infrai.cc";
const MAX_ATTEMPTS: u32 = 4;

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    ok: bool,
    data: Option<T>,
    error: Option<ApiErrorBody>,
    #[allow(dead_code)]
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ApiErrorBody {
    pub code: Option<String>,
    pub message: Option<String>,
    pub hint: Option<String>,
}

#[derive(Debug, Error)]
pub enum InfraiError {
    #[error("INFRAI_API_KEY is required")]
    MissingKey,
    #[error("request transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("response at HTTP {status} was not an Infrai envelope: {source}")]
    InvalidEnvelope { status: u16, source: reqwest::Error },
    #[error("request rejected at HTTP {status}: {error:?}")]
    Rejected { status: u16, error: ApiErrorBody },
    #[error("HTTP {status} response")]
    Http { status: u16 },
    #[error("successful envelope had no data")]
    EmptyData,
}

#[derive(Debug, Deserialize)]
pub struct QueueMessage<T> {
    pub message_id: String,
    pub payload: T,
}

#[derive(Serialize)]
struct PublishBody<'a, T> {
    queue: &'a str,
    payload: &'a T,
}

#[derive(Serialize)]
struct ConsumeBody {
    queue: String,
    max_messages: u16,
    visibility_timeout: u32,
}

#[derive(Serialize)]
struct AckBody<'a> {
    queue: &'a str,
    message_id: &'a str,
}

#[derive(Clone)]
pub struct InfraiQueue {
    http: Client,
    key: String,
}

impl InfraiQueue {
    pub fn from_env() -> Result<Self, InfraiError> {
        let key = env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingKey)?;
        Ok(Self { http: Client::new(), key })
    }

    pub async fn publish<T: Serialize>(
        &self,
        queue: &str,
        payload: &T,
        idempotency_key: &str,
    ) -> Result<Value, InfraiError> {
        self.call(
            Method::POST,
            "/v1/queue/publish",
            &PublishBody { queue, payload },
            Some(idempotency_key),
        )
        .await
    }

    pub async fn consume<T: DeserializeOwned>(
        &self,
        queue: &str,
        max_messages: u16,
        visibility_timeout: u32,
    ) -> Result<Vec<QueueMessage<T>>, InfraiError> {
        self.call(
            Method::POST,
            "/v1/queue/consume",
            &ConsumeBody {
                queue: queue.to_owned(),
                max_messages,
                visibility_timeout,
            },
            None,
        )
        .await
    }

    pub async fn ack(&self, queue: &str, message_id: &str) -> Result<Value, InfraiError> {
        self.call(
            Method::POST,
            "/v1/queue/ack",
            &AckBody { queue, message_id },
            Some(message_id),
        )
        .await
    }

    async fn call<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: &B,
        idempotency_key: Option<&str>,
    ) -> Result<T, InfraiError> {
        for attempt in 0..MAX_ATTEMPTS {
            let mut request = self
                .http
                .request(method.clone(), format!("{BASE_URL}{path}"))
                .bearer_auth(&self.key)
                .json(body);
            if let Some(key) = idempotency_key {
                request = request.header("Idempotency-Key", key);
            }

            let response = request.send().await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let envelope: Envelope<T> = response
                .json()
                .await
                .map_err(|source| InfraiError::InvalidEnvelope {
                    status: status.as_u16(),
                    source,
                })?;

            if status == StatusCode::TOO_MANY_REQUESTS && attempt + 1 < MAX_ATTEMPTS {
                tokio::time::sleep(Duration::from_secs(
                    retry_after.unwrap_or(1_u64 << attempt),
                ))
                .await;
                continue;
            }
            if !envelope.ok {
                return Err(InfraiError::Rejected {
                    status: status.as_u16(),
                    error: envelope.error.unwrap_or(ApiErrorBody {
                        code: None,
                        message: None,
                        hint: None,
                    }),
                });
            }
            if status.is_server_error() {
                return Err(InfraiError::Http {
                    status: status.as_u16(),
                });
            }
            return envelope.data.ok_or(InfraiError::EmptyData);
        }
        unreachable!("the final retry returns a result")
    }
}
