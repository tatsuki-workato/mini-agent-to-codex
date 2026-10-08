use std::fs;
use std::io;

// 指定したディレクトリの直下にある項目名を、ツールの結果として文字列の一覧にする。
pub fn list_files(path: &str) -> Result<Vec<String>, io::Error> {
    let mut files = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        files.push(entry.file_name().to_string_lossy().to_string());
    }

    Ok(files)
}
