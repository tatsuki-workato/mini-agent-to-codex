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

// テスト時だけ、このモジュールをコンパイルする。
#[cfg(test)]
mod tests {
    // 親モジュールのツール関数を、LLMを経由せず直接使う。
    use super::{list_files, read_file, run_command, write_file};
    use std::fs;

    // この関数をテストとして実行する。
    #[test]
    fn list_files_returns_created_entries() -> std::io::Result<()> {
        // 一時ディレクトリは、テストが終わると中のファイルごと自動的に削除される。
        // ? は、処理に失敗した場合にエラーを返すための記述。
        let directory = tempfile::tempdir()?;
        // テストに必要なファイル・ディレクトリを用意する。
        fs::write(directory.path().join("hello.txt"), "Hello!")?;
        fs::create_dir(directory.path().join("notes"))?;
        fs::write(directory.path().join("notes").join("nested.txt"), "Nested")?;

        let path = directory.path().to_str().expect("パスがUTF-8ではありません");
        // 用意した一時ディレクトリを対象に、関数を直接実行する。
        let mut files = list_files(path)?;

        println!("list_files の実行結果: {files:#?}");

        // ファイル一覧の順序は保証されないため、比較する前に並べ替える。
        files.sort();
        // 直下の2項目だけが返り、notes 内の nested.txt は含まれないことを確認する。
        assert_eq!(files, vec!["hello.txt", "notes"]);

        Ok(())
    }

    #[test]
    fn read_file_returns_created_content() -> std::io::Result<()> {
        // 読み取り対象はテスト自身が用意する。
        let directory = tempfile::tempdir()?;
        let file = directory.path().join("hello.txt");
        fs::write(&file, "你好！\n")?;
        let path = file.to_str().expect("パスがUTF-8ではありません");

        let content = read_file(path)?;
        println!("read_file の実行結果: {content:?}");
        assert_eq!(content, "你好！\n");

        Ok(())
    }

    #[test]
    fn write_file_creates_and_overwrites_content() -> std::io::Result<()> {
        let directory = tempfile::tempdir()?;
        let file = directory.path().join("hello.txt");
        let path = file.to_str().expect("パスがUTF-8ではありません");

        // 存在しないファイルを作成できることを、標準ライブラリで読み戻して確認する。
        write_file(path, "Hello!")?;
        let created = fs::read_to_string(&file)?;
        println!("write_file で作成した内容: {created:?}");
        assert_eq!(created, "Hello!");

        // 同じパスに書くと、追記ではなく内容の上書きになる。
        write_file(path, "你好！")?;
        let overwritten = fs::read_to_string(&file)?;
        println!("write_file で上書きした内容: {overwritten:?}");
        assert_eq!(overwritten, "你好！");

        Ok(())
    }

    // 非同期の run_command は、tokio のテスト用ランタイムで実行する。
    #[tokio::test]
    async fn run_command_returns_output() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // sh の組み込みコマンドだけを使い、既存のファイルに依存せず出力を用意する。
        let args = vec![
            "-c".to_string(),
            "printf '%s' 'Hello!'; printf '%s' 'Notice' >&2".to_string(),
        ];
        let result = run_command("sh", &args).await?;
        println!("run_command の実行結果:\n{result}");

        // 終了コード・標準出力・標準エラーがそれぞれ返ることを確認する。
        assert_eq!(
            result,
            "終了コード: 0\n標準出力:\nHello!\n標準エラー出力:\nNotice"
        );

        Ok(())
    }

    #[tokio::test]
    async fn run_command_returns_nonzero_exit() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let args = vec![
            "-c".to_string(),
            "printf '%s' 'Failed' >&2; exit 7".to_string(),
        ];

        // コマンドの非ゼロ終了は、ツールの呼び出し自体のエラーとは区別される。
        // .await? が成功し、終了コード7を含む結果が返ることを確認する。
        let result = run_command("sh", &args).await?;
        println!("run_command の非ゼロ終了の結果:\n{result}");
        assert_eq!(
            result,
            "終了コード: 7\n標準出力:\n\n標準エラー出力:\nFailed"
        );

        Ok(())
    }
}
