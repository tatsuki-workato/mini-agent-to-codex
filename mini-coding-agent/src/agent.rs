// main.rsで宣言したHTTP・ツールのモジュールを使う。
use crate::{http, tool};
use serde_json::{Value, json};
use tokio::sync::mpsc::UnboundedSender;

// 途中経過と処理完了を区別して、TUIへ通知する。
// Messageを受け取っても処理は続き、Finishedで次の入力を受け付ける。
pub enum AgentEvent {
    Message(String),
    Finished(Result<(), String>),
}

// 一つの依頼について、ツール結果を返しながら最終回答まで繰り返す。
pub async fn run_turn(
    prompt: String,
    sender: UnboundedSender<AgentEvent>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // この依頼の途中経過だけを残す履歴。次のユーザー入力には引き継がない。
    let mut history = vec![json!({"role": "user", "content": prompt})];

    // モデルへの依頼と、呼び出し可能なツールの仕様を定義する。
    let mut body = json!({
        "model": "qwen3.5:4b",
        "input": [],
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
            },
            {
                "type": "function",
                "name": "read_file",
                "description": "指定したファイルをテキストとして読み取る",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string"
                        }
                    },
                    "required": ["path"]
                }
            },
            {
                "type": "function",
                "name": "write_file",
                "description": "指定したファイルを作成するか、既存の内容を上書きする",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string"
                        },
                        "content": {
                            "type": "string"
                        }
                    },
                    "required": ["path", "content"]
                }
            },
            {
                "type": "function",
                "name": "run_command",
                "description": "プロジェクトのディレクトリでプログラムを実行し、終了コードと出力を取得する",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "program": {
                            "type": "string",
                            "description": "実行するプログラム名。例: python3"
                        },
                        "args": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "プログラムに渡す引数。例: [\"sample.py\"]"
                        }
                    },
                    "required": ["program", "args"]
                }
            }
        ]
    });

    let mut displayed_message = false;
    // 直前の結果を含む履歴を再送し、次の操作をモデルに判断してもらう。
    // 無限に繰り返さないよう、モデルへの問い合わせは10回までにする。
    for request_number in 1..=10 {
        // 次のモデル応答を待っていることも、その都度知らせる。
        sender.send(AgentEvent::Message(format!(
            "モデルに問い合わせています（{request_number}/10）"
        )))?;
        body["input"] = json!(history);
        let response = http::request(&body).await?;

        // ツールの要求や文章回答を履歴に追加する。
        // 要求とその実行結果の両方を、次の問い合わせでモデルに見せる。
        let outputs = match response["output"].as_array() {
            Some(outputs) => outputs,
            None => return Err("モデルの応答に output がありませんでした".into()),
        };
        history.extend(outputs.iter().cloned());
        let mut called_tool = false;

        // 各項目の種別と名前を照合し、文章の表示かツールの実行を選ぶ。
        for output in outputs {
            if output["type"] == "function_call" {
                displayed_message = true;
            }
            let tool_result = match (output["type"].as_str(), output["name"].as_str()) {
                (Some("function_call"), Some("list_files")) => {
                    // モデルが文字列で返した引数を JSON に変換し、対象パスを取得する。
                    let arguments = output["arguments"].as_str().unwrap_or("{}");
                    let args: Value = serde_json::from_str(arguments)?;
                    let path = args["path"].as_str().unwrap_or(".");

                    // ローカルでツールを実行し、呼び出しと結果を会話欄に表示する。
                    sender.send(AgentEvent::Message(format!("ツール呼び出し: list_files({path:?})")))?;
                    let files = tool::list_files(path)?;
                    let result = serde_json::to_string_pretty(&files)?;
                    sender.send(AgentEvent::Message(format!("ツール結果:\n{result}")))?;
                    Some(result)
                }
                (Some("function_call"), Some("read_file")) => {
                    let arguments = output["arguments"].as_str().unwrap_or("{}");
                    let args: Value = serde_json::from_str(arguments)?;
                    let path = args["path"].as_str().ok_or("path がありません")?;

                    sender.send(AgentEvent::Message(format!("ツール呼び出し: read_file({path:?})")))?;
                    let content = tool::read_file(path)?;
                    sender.send(AgentEvent::Message(format!("ツール結果:\n{content}")))?;
                    Some(content)
                }
                (Some("function_call"), Some("write_file")) => {
                    let arguments = output["arguments"].as_str().unwrap_or("{}");
                    let args: Value = serde_json::from_str(arguments)?;
                    let path = args["path"].as_str().ok_or("path がありません")?;
                    let content = args["content"].as_str().ok_or("content がありません")?;

                    sender.send(AgentEvent::Message(format!("ツール呼び出し: write_file({path:?})")))?;
                    tool::write_file(path, content)?;
                    sender.send(AgentEvent::Message("ツール結果: 書き込み完了".to_string()))?;
                    Some("書き込み完了".to_string())
                }
                (Some("function_call"), Some("run_command")) => {
                    let arguments = output["arguments"].as_str().unwrap_or("{}");
                    let args: Value = serde_json::from_str(arguments)?;
                    let program = args["program"].as_str().ok_or("program がありません")?;
                    let command_args: Vec<String> = serde_json::from_value(args["args"].clone())?;

                    // 時間のかかるコマンドでも、実行開始を先に表示する。
                    sender.send(AgentEvent::Message(format!(
                        "ツール呼び出し: run_command({program:?}, {command_args:?})"
                    )))?;
                    let result = tool::run_command(program, &command_args).await?;
                    sender.send(AgentEvent::Message(format!("ツール結果:\n{result}")))?;
                    Some(result)
                }
                (Some("message"), _) => {
                    let mut answer = String::new();
                    if let Some(parts) = output["content"].as_array() {
                        for part in parts {
                            if part["type"] == "output_text" {
                                if let Some(text) = part["text"].as_str() {
                                    if !answer.is_empty() {
                                        answer.push('\n');
                                    }
                                    answer.push_str(text);
                                }
                            }
                        }
                    }
                    if !answer.is_empty() {
                        displayed_message = true;
                        sender.send(AgentEvent::Message(format!("Ollama: {answer}")))?;
                    }
                    None
                }
                (Some("function_call"), _) => {
                    sender.send(AgentEvent::Message(format!("未対応のツール: {}", output["name"])))?;
                    Some(format!("未対応のツール: {}", output["name"]))
                }
                _ => None,
            };

            if let Some(result) = tool_result {
                called_tool = true;
                // call_id は、モデルの要求とその実行結果を対応付ける識別子。
                let call_id = output["call_id"].as_str().ok_or("call_id がありません")?;
                history.push(json!({
                    "type": "function_call_output",
                    "call_id": call_id,
                    "output": result
                }));
            }
        }

        // ツール要求がなければ、この依頼のLoopを終える。
        if !called_tool {
            return if !displayed_message {
                Err("モデルから表示できる応答がありませんでした".into())
            } else {
                Ok(())
            };
        }
    }

    sender.send(AgentEvent::Message("モデルへの問い合わせが10回に達したため停止しました".to_string()))?;
    Ok(())
}
