use std::fs;
use std::io;

// 指定したディレクトリの直下にあるファイル・ディレクトリ名を取得する。
pub fn list_files(path: &str) -> Result<Vec<String>, io::Error> {
    let mut files = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        files.push(entry.file_name().to_string_lossy().to_string());
    }

    Ok(files)
}

// テスト時だけ、このモジュールをコンパイルする。
#[cfg(test)]
mod tests {
    // 親モジュールの list_files 関数を使う。
    use super::list_files;
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
}
