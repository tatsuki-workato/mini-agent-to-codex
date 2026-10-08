use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};

fn main() -> std::io::Result<()> {
    // TUI を起動し、終了時にはターミナルを元の状態に戻す。
    ratatui::run(|terminal| {
        let mut input = String::new();
        let mut messages: Vec<String> = Vec::new();

        loop {
            terminal.draw(|frame| {
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
                    messages.join("\n")
                };
                let history_height = areas[0].height.saturating_sub(2) as usize;
                let history_scroll = messages.len().saturating_sub(history_height) as u16;
                frame.render_widget(
                    Paragraph::new(history)
                        .block(Block::bordered().title("会話"))
                        .scroll((history_scroll, 0)),
                    areas[0],
                );

                // 入力が横に長くなったら末尾が見える位置までスクロールする。
                let input_width = Line::from(input.as_str()).width();
                let visible_width = areas[1].width.saturating_sub(2) as usize;
                let input_scroll = input_width.saturating_sub(visible_width.saturating_sub(1));
                frame.render_widget(
                    Paragraph::new(input.as_str())
                        .block(Block::bordered().title("入力"))
                        .scroll((0, input_scroll as u16)),
                    areas[1],
                );

                // 文字を入力する場所にカーソルを表示する。
                if visible_width > 0 && areas[1].height > 2 {
                    let cursor_offset = (input_width - input_scroll).min(visible_width - 1) as u16;
                    frame.set_cursor_position((areas[1].x + 1 + cursor_offset, areas[1].y + 1));
                }

                frame.render_widget(
                    Paragraph::new("Enter: 追加  Backspace: 削除  Ctrl+C: 終了"),
                    areas[2],
                );
            })?;

            // キーを押したとき（または長押ししたとき）だけ入力を処理する。
            if let Event::Key(key) = event::read()? {
                if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                    continue;
                }

                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Enter => {
                        if !input.trim().is_empty() {
                            messages.push(format!("あなた: {input}"));
                        }
                        input.clear();
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(c)
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        input.push(c);
                    }
                    _ => {}
                }
            }
        }
    })
}
