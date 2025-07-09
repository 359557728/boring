use serde::{Deserialize, Serialize};
use worker::*;
use worker::kv::KvError;
use thiserror::Error;

const KV_BINDING: &str = "EMAR_BORING";
const KV_UPSTREAM_URL_KEY: &str = "dp_zombie_task";

#[derive(Debug, Deserialize, Serialize)]
pub struct GenericResponse {
    pub code: i32,
    pub message: Option<String>,
    pub data: Option<serde_json::Value>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Worker error: {0}")]
    Worker(worker::Error),
    #[error("Request error: {0}")]
    Reqwest(reqwest::Error),
    #[error("KV error: {0}")]
    KvError(KvError),
    #[error("KV key not found: {0}")]
    KvKeyNotFound(String),
    #[error("Upstream API error: {0} - {1}")]
    UpstreamApi(u16, String),
}

impl From<worker::Error> for Error {
    fn from(err: worker::Error) -> Self {
        Error::Worker(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Reqwest(err)
    }
}

impl From<KvError> for Error {
    fn from(err: KvError) -> Self {
        Error::KvError(err)
    }
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    let r = Router::new();
    r.get_async("/zombie/task", |_, ctx| async {
        match zombie_task(ctx).await {
            Ok(response) => Ok(response),
            Err(e) => {
                console_error!("Error in zombie_task: {:?}", e);
                Response::from_json(&GenericResponse {
                    code: 500,
                    message: Some("An internal server error occurred.".to_string()),
                    data: None,
                })
            }
        }
    })
    .run(req, env)
    .await
}

pub async fn zombie_task(ctx: RouteContext<()>) -> std::result::Result<Response, Error> {
    let kv = ctx.kv(KV_BINDING)?;
    let api = kv
        .get(KV_UPSTREAM_URL_KEY)
        .text()
        .await?
        .ok_or_else(|| Error::KvKeyNotFound(format!("KV key '{}' not found", KV_UPSTREAM_URL_KEY)))?;

    let rt = emar_get_api(&api).await?;
    Response::from_json(&rt).map_err(Error::from)
}

async fn emar_get_api(api: &str) -> std::result::Result<GenericResponse, Error> {
    let res = reqwest::get(api).await?;
    if res.status().is_success() {
        let rt = res.json::<GenericResponse>().await?;
        Ok(rt)
    } else {
        Err(Error::UpstreamApi(
            res.status().as_u16(),
            format!("Upstream API returned an error: {}", res.status()),
        ))
    }
}
