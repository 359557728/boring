use serde::{Deserialize, Serialize};
use worker::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct GenericResponse {
    pub code: i32,
    pub message: Option<String>,
    pub data: Option<serde_json::Value>,
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    let r = Router::new();
    r.get_async("/zombie/task", zombie_task).run(req, env).await
}

pub async fn zombie_task(_: Request, ctx: RouteContext<()>) -> worker::Result<Response> {
    let kv = ctx.kv("EMAR_BORING").unwrap();
    let api = kv.get("dp_zombie_task").text().await.unwrap().unwrap();
    let rt = emar_get_api(&api).await;
    Response::from_json(&rt)
}

async fn emar_get_api(api: &str) -> GenericResponse {
    let rt = reqwest::get(api).await;
    if let Ok(res) = rt {
        if res.status() == reqwest::StatusCode::OK {
            let rt = res.json::<GenericResponse>().await;
            if let Ok(rt) = rt {
                return rt;
            }
        }
    }
    return GenericResponse {
        code: -1,
        message: Some(String::from("接口暂不可用")),
        data: None,
    };
}
