use reqwest::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    // モデルへの依頼と、モデルが呼び出せるツールの仕様を組み立てる。
    let body = json!({
        "model": "qwen3.5:9b",
        "input": "カレントディレクトリに何があるか確認して",
        "think": false,
        "tools": [
            {
                "type": "function",
                "name": "list_files",
                "description": "指定したディレクトリ内のファイルとディレクトリ一覧を取得する",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "確認するディレクトリのパス"
                        }
                    },
                    "required": ["path"]
                }
            }
        ]
    });

    // ローカルのモデルにリクエストを送る。
    let response = client
        .post("http://localhost:11434/v1/responses")
        .json(&body)
        .send()
        .await?;

    // 応答を JSON として読み取り、内容を確認できる形で表示する。
    let json: serde_json::Value = response.json().await?;

    println!("{}", serde_json::to_string_pretty(&json)?);

    Ok(())
}
