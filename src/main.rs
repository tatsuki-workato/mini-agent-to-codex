use reqwest::Client;
use serde_json::{Value, json};
use std::fs;

// 指定したディレクトリの直下にある項目名を、モデルに返せる文字列の一覧にする。
fn list_files(path: &str) -> Result<Vec<String>, std::io::Error> {
    let mut files = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        files.push(entry.file_name().to_string_lossy().to_string());
    }

    Ok(files)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    // モデルへの依頼と、呼び出し可能な list_files ツールの仕様を定義する。
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
                            "type": "string"
                        }
                    },
                    "required": ["path"]
                }
            }
        ]
    });

    println!("=== 1. Request ===");
    println!("{}", serde_json::to_string_pretty(&body)?);

    // ローカルモデルの応答を JSON として受け取り、ツール呼び出しの有無を調べる。
    let response: Value = client
        .post("http://localhost:11434/v1/responses")
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    println!("\n=== 2. Model Response ===");
    println!("{}", serde_json::to_string_pretty(&response)?);

    // 応答の中から list_files の呼び出しだけを取り出す。
    if let Some(outputs) = response["output"].as_array() {
        for output in outputs {
            if output["type"] == "function_call" && output["name"] == "list_files" {
                println!("\n=== 3. Tool Call ===");
                println!("name: {}", output["name"]);
                println!("arguments: {}", output["arguments"]);

                // モデルが文字列で返した引数を JSON に変換し、対象パスを取得する。
                let arguments = output["arguments"].as_str().unwrap_or("{}");

                let args: Value = serde_json::from_str(arguments)?;
                let path = args["path"].as_str().unwrap_or(".");

                println!("\n=== 4. Execute Tool ===");
                println!("list_files({path:?})");

                // ローカルでツールを実行し、その結果を表示する。
                let files = list_files(path)?;

                println!("\n=== 5. Tool Result ===");
                println!("{}", serde_json::to_string_pretty(&files)?);
            }
        }
    }

    Ok(())
}
