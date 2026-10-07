use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};

fn main() -> std::io::Result<()> {
    // ターミナルを TUI 用の状態に切り替える。クロージャが終了すると元の状態に戻る。
    ratatui::run(|terminal| {
        loop {
            // 現在の画面全体に文字列を描画する。draw の失敗は ? で呼び出し元へ返す。
            terminal.draw(|frame| {
                frame.render_widget("Mini coding agent — q で終了", frame.area());
            })?;

            // 次の入力を待つ。キー以外のイベント（画面サイズの変更など）は読み飛ばす。
            if let Event::Key(key) = event::read()? {
                // キーを押した瞬間の q だけで終了する。離したときや長押しのイベントは対象外。
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    // ループとクロージャを抜け、ratatui::run にターミナルを復元させる。
                    return Ok(());
                }
            }
        }
    })
}
