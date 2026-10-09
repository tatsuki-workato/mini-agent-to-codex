use reqwest::Client;
use serde_json::Value;

// リクエストのJSONをOllamaに送り、応答のJSONを返す。
// ツールの選択や実行、会話履歴の管理は呼び出し元が担当する。
pub async fn request(body: &Value) -> Result<Value, reqwest::Error> {
    let client = Client::new();

    let response = client
        .post("http://localhost:11434/v1/responses")
        .json(body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(response)
}
