use futures_util::StreamExt;
use ratatui::crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use reqwest::Client;
use serde_json::{Value, json};
use std::fs;
use std::io;
use tokio::sync::mpsc;

// 指定したディレクトリの直下にある項目名を、ツールの結果として文字列の一覧にする。
fn list_files(path: &str) -> Result<Vec<String>, std::io::Error> {
    let mut files = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        files.push(entry.file_name().to_string_lossy().to_string());
    }

    Ok(files)
}

// 入力した文章を Ollama に送り、文章回答またはツールの実行結果を取り出す。
async fn request_model(
    prompt: String,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();

    // モデルへの依頼と、呼び出し可能な list_files ツールの仕様を定義する。
    let body = json!({
        "model": "qwen3.5:9b",
        "input": prompt,
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

    // ローカルモデルの応答を JSON として受け取り、ツール呼び出しの有無を調べる。
    let response: Value = client
        .post("http://localhost:11434/v1/responses")
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    // output が配列でなければ、表示できる応答がないので終了する。
    let outputs = match response["output"].as_array() {
        Some(outputs) => outputs,
        None => return Err("モデルの応答に output がありませんでした".into()),
    };

    let mut messages = Vec::new();
    // 各項目の種別と名前を照合し、文章の表示か list_files の実行を選ぶ。
    for output in outputs {
        match (output["type"].as_str(), output["name"].as_str()) {
            (Some("function_call"), Some("list_files")) => {
                // モデルが文字列で返した引数を JSON に変換し、対象パスを取得する。
                let arguments = output["arguments"].as_str().unwrap_or("{}");
                let args: Value = serde_json::from_str(arguments)?;
                let path = args["path"].as_str().unwrap_or(".");

                // ローカルでツールを実行し、呼び出しと結果を会話欄に表示する。
                let files = list_files(path)?;
                messages.push(format!("ツール呼び出し: list_files({path:?})"));
                messages.push(format!(
                    "ツール結果:\n{}",
                    serde_json::to_string_pretty(&files)?
                ));
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
                    messages.push(format!("Ollama: {answer}"));
                }
            }
            (Some("function_call"), _) => {
                messages.push(format!("未対応のツール: {}", output["name"]));
            }
            _ => {}
        }
    }

    if messages.is_empty() {
        Err("モデルから表示できる応答がありませんでした".into())
    } else {
        Ok(messages)
    }
}

fn draw(frame: &mut Frame, input: &str, messages: &[String], busy: bool) {
    // 画面を履歴、入力欄、操作案内の3つに分ける。
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let history = if messages.is_empty() {
        "入力した内容がここに表示されます".to_string()
    } else {
        messages.join("\n\n")
    };
    let history_height = areas[0].height.saturating_sub(2) as usize;
    let history_width = areas[0].width.saturating_sub(2).max(1) as usize;
    let history_lines: usize = history
        .lines()
        .map(|line| Line::from(line).width().max(1).div_ceil(history_width))
        .sum();
    let history_scroll = history_lines
        .saturating_sub(history_height)
        .min(u16::MAX as usize) as u16;
    frame.render_widget(
        Paragraph::new(history)
            .block(Block::bordered().title("会話"))
            .wrap(Wrap { trim: false })
            .scroll((history_scroll, 0)),
        areas[0],
    );

    // 入力が横に長くなったら末尾が見える位置までスクロールする。
    let input_width = Line::from(input).width();
    let visible_width = areas[1].width.saturating_sub(2) as usize;
    let input_scroll = input_width.saturating_sub(visible_width.saturating_sub(1));
    frame.render_widget(
        Paragraph::new(input)
            .block(Block::bordered().title("入力"))
            .scroll((0, input_scroll.min(u16::MAX as usize) as u16)),
        areas[1],
    );

    // 文字を入力する場所にカーソルを表示する。
    if visible_width > 0 && areas[1].height > 2 {
        let cursor_offset = (input_width - input_scroll).min(visible_width - 1) as u16;
        frame.set_cursor_position((areas[1].x + 1 + cursor_offset, areas[1].y + 1));
    }

    let help = if busy {
        "Ollama の返答を待っています…  Ctrl+C: 終了"
    } else {
        "Enter: 送信  Backspace: 削除  Ctrl+C: 終了"
    };
    frame.render_widget(Paragraph::new(help), areas[2]);
}

async fn run_ui(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let (sender, mut receiver) = mpsc::unbounded_channel::<Result<Vec<String>, String>>();
    let mut events = EventStream::new();
    let mut input = String::new();
    let mut messages: Vec<String> = Vec::new();
    let mut busy = false;

    loop {
        terminal.draw(|frame| draw(frame, &input, &messages, busy))?;

        // キー入力と HTTP の完了を同時に待つ。
        tokio::select! {
            event = events.next() => match event {
                Some(Ok(Event::Key(key)))
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    match key.code {
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            return Ok(());
                        }
                        KeyCode::Enter if !busy && !input.trim().is_empty() => {
                            let prompt = std::mem::take(&mut input);
                            messages.push(format!("あなた: {prompt}"));
                            busy = true;

                            let sender = sender.clone();
                            tokio::spawn(async move {
                                let result = request_model(prompt).await.map_err(|error| error.to_string());
                                let _ = sender.send(result);
                            });
                        }
                        KeyCode::Backspace => {
                            input.pop();
                        }
                        KeyCode::Char(c)
                            if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                        {
                            input.push(c);
                        }
                        _ => {}
                    }
                }
                Some(Err(error)) => return Err(error),
                None => return Ok(()),
                _ => {}
            },
            result = receiver.recv(), if busy => {
                busy = false;
                match result {
                    Some(Ok(output)) => messages.extend(output),
                    Some(Err(error)) => messages.push(format!("エラー: {error}")),
                    None => messages.push("エラー: 応答を受け取れませんでした".to_string()),
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // 非同期の画面ループが終わった後も、必ずターミナルを復元する。
    let mut terminal = ratatui::init();
    let result = run_ui(&mut terminal).await;
    ratatui::restore();
    result
}
