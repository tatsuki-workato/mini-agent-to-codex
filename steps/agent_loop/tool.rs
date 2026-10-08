use std::fs;
use std::io;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

// 指定したディレクトリの直下にある項目名を、ツールの結果として文字列の一覧にする。
pub fn list_files(path: &str) -> Result<Vec<String>, io::Error> {
    let mut files = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        files.push(entry.file_name().to_string_lossy().to_string());
    }

    Ok(files)
}

// 指定したファイルをテキストとして読み取る。
pub fn read_file(path: &str) -> Result<String, io::Error> {
    fs::read_to_string(path)
}

// 指定したファイルを作成、または既存の内容を上書きする。
pub fn write_file(path: &str, content: &str) -> Result<(), io::Error> {
    fs::write(path, content)
}

// プロジェクトのディレクトリでコマンドを実行し、終了コードと出力を返す。
pub async fn run_command(
    program: &str,
    args: &[String],
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .kill_on_drop(true);

    let output = timeout(Duration::from_secs(30), command.output())
        .await
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                "コマンドが30秒でタイムアウトしました",
            )
        })??;
    let exit_code = output
        .status
        .code()
        .map_or_else(|| "シグナルで終了".to_string(), |code| code.to_string());
    let stdout: String = String::from_utf8_lossy(&output.stdout)
        .chars()
        .take(4000)
        .collect();
    let stderr: String = String::from_utf8_lossy(&output.stderr)
        .chars()
        .take(4000)
        .collect();

    Ok(format!(
        "終了コード: {exit_code}\n標準出力:\n{stdout}\n標準エラー出力:\n{stderr}"
    ))
}
