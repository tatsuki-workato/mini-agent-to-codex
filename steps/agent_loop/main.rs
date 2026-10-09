mod agent;
mod http;
mod tool;

use agent::AgentEvent;
use futures_util::StreamExt;
use ratatui::crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use std::io;
use tokio::sync::mpsc;

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
        "Agentが処理しています…  Ctrl+C: 終了"
    } else {
        "Enter: 送信  Backspace: 削除  Ctrl+C: 終了"
    };
    frame.render_widget(Paragraph::new(help), areas[2]);
}

async fn run_ui(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let (sender, mut receiver) = mpsc::unbounded_channel::<AgentEvent>();
    let mut events = EventStream::new();
    let mut input = String::new();
    let mut messages: Vec<String> = Vec::new();
    let mut busy = false;

    loop {
        terminal.draw(|frame| draw(frame, &input, &messages, busy))?;

        // キー入力とAgentからの途中経過・処理完了を同時に待つ。
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
                                let result = agent::run_turn(prompt, sender.clone()).await.map_err(|error| error.to_string());
                                let _ = sender.send(AgentEvent::Finished(result));
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
            event = receiver.recv(), if busy => {
                match event {
                    // 途中経過ではbusyを解除せず、次の描画でメッセージを表示する。
                    Some(AgentEvent::Message(message)) => messages.push(message),
                    Some(AgentEvent::Finished(Ok(()))) => {
                        busy = false;
                    }
                    Some(AgentEvent::Finished(Err(error))) => {
                        messages.push(format!("エラー: {error}"));
                        busy = false;
                    }
                    None => {
                        messages.push("エラー: 応答を受け取れませんでした".to_string());
                        busy = false;
                    }
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
