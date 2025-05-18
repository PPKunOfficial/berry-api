use axum::extract::Json;
use axum_extra::TypedHeader;
use reqwest::Client; // 用于发起 HTTP 请求
use serde_json::{Value, json}; // 用于解析 JSON 数据

use crate::relay::openai::OPENAI_API_URL;

pub async fn handle_model(
    // 提取 Bearer Token 类型的 Authorization 头
    TypedHeader(authorization): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    TypedHeader(content_type): TypedHeader<headers::ContentType>,
) -> Json<Value> {
    let api_client = Client::new();
    let token = authorization.token();
    let auth_header_value = format!("Bearer {}", token);
    let request_result = api_client
        .get(format!("{}/models", OPENAI_API_URL))
        .header("Authorization", auth_header_value)
        .header("Content-Type", content_type.to_string())
        .send()
        .await;
    match request_result {
        Ok(resp) => {
            if resp.status().is_success() {
                let text = resp.text().await.unwrap();
                match serde_json::from_str(&text) {
                    Ok(val) => Json(val),
                    Err(_) => Json(
                        json!({"berry-api-error": text,"错误信息": "上游返回数据格式错误，解析失败"}),
                    ),
                }
            } else {
                let text = resp.text().await.unwrap();
                Json(json!({"berry-api-error": text,"错误信息": "请求上游失败"}))
            }
        }
        Err(e) => Json(json!({"berry-api-error": e.to_string(),"错误信息": "请求上游失败"})),
    }
}
