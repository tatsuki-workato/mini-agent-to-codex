# Coding Agentを作ってCodexを読む

## 目的

Coding Agentは、こちらの指示を受けてファイルを読み、コードを書き、実行までしてくれます。

しかし、その裏でLLMとプログラムがどう役割分担しているのかは、使っているだけではなかなかわかりません。

そこで、まずは小さいCoding Agentを自分で作りながら、その仕組みを理解していきます。

最初はLLMをローカル環境で動かして、HTTPリクエストを送ると何が返ってくるのかを見ます。

次にTool Callingの挙動を確認し、Coding AgentがLLMを使ってファイル操作などの実際の処理を行う仕組みを確かめます。

さらにRustで最小限のCoding Agentを開発していきます。

これによって、LLM APIへのHTTPリクエストとツールの実行を組み合わせると、なぜエンジニアのように振る舞えるのかを見ていきます。

この開発経験をもとにCodexのコードや開発プロセスを読み解き、最新のAIアプリケーションがどのように作られているのかを理解することを目指します。

## LLMの動作を確かめる

### OllamaでローカルLLMを動かす

LLMをローカル環境で動かすために、Ollamaを使います。
  
>Ollamaは、LLM（大規模言語モデル）を自分のPC上で手軽に動かすためのツールです。
>
>Llama、Qwen、Gemmaなどをダウンロードして、コマンド一つでローカル実行できます。
>
>クラウドAPIなしで使えるのが大きな特徴です。

環境構築はClaude CodeでもCodexでも使って以下のように聞いてください。

```sh
Ollamaをつかえるようにしてください
```

Ollamaが利用できるようになったら、Qwenを今回の教材用LLMモデルとして利用します。


> Qwenは、Alibabaが開発している生成AIモデル（LLM）のシリーズです。
>
> 文章生成、要約、翻訳、コーディング、画像理解などに対応する複数のモデルがあります


今回は`Qwen3.5 4B`を利用します。Ollamaでのモデル名は`qwen3.5:4b`です。

https://ollama.com/library/qwen3.5:4b

大体3.3GB - 4.0GBのストレージを消費するのでご注意ください。

>「3.5」はモデルの世代・バージョンを表します。
>
>「4B」のBはBillion（10億）の意味で、約40億個のパラメータを持つモデルということです。
>
>一般にパラメータ数が多いほど、複雑な知識やパターンを保持でき、推論・文章生成などの能力が高くなる傾向があります。
>
>しかし、大きいモデルほど必要なメモリや計算量も増えます。
>
>今回はなるべく多くの環境で試せるよう、比較的小さい4Bを選定しています。
>
>スペックに余裕がある人は9Bや他のモデルも試してみてください。


ここからはターミナルを使った作業が多くなります。ターミナルを開いてください。

モデルを取得し、対話ができる以下のコマンドをターミナルに貼り付けてください。

```sh
ollama run qwen3.5:4b
```
モデルを取得するのに少し時間がかかりますが、完了したらLLMと会話ができるようになっていると思うので遊んでみてください。

対話モードでの遊びが終わったら、ターミナルで以下を実行してみましょう。

なぜHTTPで試すかというと、Coding AgentもLLMサーバーにHTTPリクエストを送り、応答を受け取る形で動くことが多いからです。

```sh
curl http://localhost:11434/v1/responses \
 -H 'Content-Type: application/json' \
 -d '{
   "model": "qwen3.5:4b",
   "input": "123と456を足して",
   "think": false
 }' | jq
```

末尾の`| jq` はcurl の出力をパイプ | で jq に渡し、返ってきたJSONを整形・抽出して見やすくするコマンドです。

jqが環境に入っていなければCoding Agentに以下のように頼んでください。

```
jqを使えるようにしてください
```


### Tool Callingを試す

QwenはTool Callingに対応しています。Coding AgentはTool Callingを使ってファイルの読み込みや編集などの操作を行います。

>Tool Callingとは、LLMが必要に応じて外部ツールの呼び出しを要求する仕組みです。この教材では、ファイルの読み書きやコマンド実行をツールとして使います。
>
>モデル自身が「どのツールを、どんな引数で使うか」を決めます。実際にツールを実行し、その結果をモデルに返すのはプログラム側です。
>
>QwenでTool Callingできるのは、ツール定義を理解し、決められた形式で呼び出し指示を出すよう学習・調整されているためです。
>
>つまり単に文章を生成するだけでなく、「この場面では外部ツールを使うべき」と判断できるよう設計されています。

QwenがTool Callingに対応しているか見てみましょう。

以下のコマンドをターミナルで実行すると、`Capabilities`の中に`tools`という項目があり、対応していることがわかると思います。

```sh
ollama show qwen3.5:4b
```


では、ツールの定義を渡してリクエストを送ってみましょう。

ここで定義している`list_files`というツールは、現時点では実際には存在しません。

Qwenに対して、「path という引数を受け取り、指定したディレクトリ内のファイルやディレクトリ一覧を取得できる処理を、こちら側が持っている」と教えているだけです。

Qwenは、このツール定義と`input`の内容を見て、

ツールを使うべきか

使う場合、どのツールを選ぶか

どのような引数を渡すか

を判断して回答します。

```sh
curl http://localhost:11434/v1/responses \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "qwen3.5:4b",
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
              "type": "string",
              "description": "確認するディレクトリのパス"
            }
          },
          "required": ["path"]
        }
      }
    ]
  }' | jq

```

うまくいけばこんなレスポンスが返ってくるはずです。

ここでは output フィールドに注目します。
`"type": "function_call"` となっているので、Qwenは通常の文章回答ではなく、ツールを呼び出すべきだと判断したことが分かります。

さらに、

`"name": "list_files"` → `list_files`ツールを使う

`"arguments": "{\"path\":\".\"}"` → path に `.`、つまりカレントディレクトリを渡す

という指示を返しています。
```response.json
{
  "id": "resp_843953",
  "object": "response",
  "created_at": 1791347652,
  "completed_at": 1791347652,
  "status": "completed",
  "incomplete_details": null,
  "model": "qwen3.5:4b",
  "previous_response_id": null,
  "instructions": null,
  "output": [
    {
      "id": "fc_resp_843953_0",
      "type": "function_call",
      "status": "completed",
      "call_id": "call_het6jin4",
      "name": "list_files",
      "arguments": "{\"path\":\".\"}"
    }
  ],
  "error": null,
  "tools": [
    {
      "type": "function",
      "name": "list_files",
      "description": "指定したディレクトリ内のファイルとディレクトリ一覧を取得する",
      "parameters": {
        "properties": {
          "path": {
            "description": "確認するディレクトリのパス",
            "type": "string"
          }
        },
        "required": [
          "path"
        ],
        "type": "object"
      }
    }
  ],
// 長いので省略
```

では`list_files`をツールとして渡したものの、関係がなさそうなinputの場合はどうなるでしょうか？

この場合は`function_call`以外の`type`で返答が返ってくるはずです。

`input`の内容を変えていろいろ試してみてください。

```sh
curl http://localhost:11434/v1/responses \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "qwen3.5:4b",
    "input": "httpサーバー起動",                        
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
              "type": "string",
              "description": "確認するディレクトリのパス"
            }
          },
          "required": ["path"]
        }
      }
    ]
  }' | jq

```

## 小さいCoding Agentを作る

### RustからLLMを呼ぶ

次に、この呼び出しをプログラムから行えるようにします。

今回はRustという言語を使います。Rustを使うのは、Codex CLIの主要部分がRustで書かれているからです。

簡単なプログラムからRustに慣れていきましょう。

> Rustは、Mozilla発祥のシステムプログラミング向け言語です。
>
> 所有権や借用といった独自の仕組みを持ち、OS周辺、CLI、サーバー、組み込みなど幅広い用途で使われています。

Rustの環境構築はこれまで通り、Coding Agentに以下のように頼んでください。

```sh
Rustを使えるようにしてください
```

Rustが使えるようになったら、以下のコマンドでRustのプロジェクトを作りましょう。

```sh
cargo new mini-coding-agent
cd mini-coding-agent
```

プロジェクトディレクトリに移動したら、最初のプログラムを動かしましょう。

```sh
cargo run
```

```sh
Hello, world!
```

が表示されます。これは`src/main.rs`に書かれた内容が動いたわけです。

```rust
fn main() {
    println!("Hello, world!");
}
```

以降のリンクは各段階のコード全文です。試すときは、リンク先の内容を指定されたファイルにコピーしてください。


続いて、HTTPリクエストをRustで送受信するためのクレート（再利用できるRustのライブラリ）をプロジェクトに追加します。

```sh
cargo add reqwest --features json
cargo add tokio --features full
cargo add serde_json
```


`src/main.rs` のコードを以下のように変更しましょう。

[この段階の `main.rs` 全文](steps/http_request.rs)

これは、先ほどまでcurlで送っていた内容をRustで送るようにしたというだけです。

コードを変更したら`cargo run`してください。`curl`でやっていたのと同じようにレスポンスが返ってくるはずです。


都度、コードについてはCoding Agentに解説をしてもらうといいです。

```sh
mini-coding-agentプロジェクトのsrc/main.rsについてわかりやすく解説してください
```


### list_filesを実装する

では次に、ツールを実装し、モデルがその使用を要求したらプログラム側で実行できるようにしましょう。

ここでは、コードを `main.rs` と `tool.rs` の2つのファイルに分けます。

- `main.rs` は、LLMへのリクエストと、返ってきたツール呼び出しの処理を担当します。
- `tool.rs` は、実際にディレクトリ内の一覧を取得する `list_files` 関数を実装します。

まず、`src/tool.rs` を新しく作成し、以下のコードをコピーしてください。

[この段階の `tool.rs` 全文](steps/list_files/tool.rs)

`list_files` は、パスを受け取ってファイル・ディレクトリ名の一覧を返す普通のRust関数です。関数の中ではLLMを呼び出していません。`pub` は、別のモジュールからこの関数を呼び出せるようにする指定です。

次に、`src/main.rs` を以下のコードに置き換えてください。

[この段階の `main.rs` 全文](steps/list_files/main.rs)

ファイルの配置は以下のようになります。

```text
mini-coding-agent/
└── src/
    ├── main.rs
    └── tool.rs
```

`main.rs` の先頭にある `mod tool;` によって、`tool.rs` をモジュールとして読み込みます。LLMが `list_files` の呼び出しを要求したら、`main.rs` が引数を取り出して `tool::list_files(path)` を呼び出します。

リクエスト内のツール定義は、LLMに関数の名前・用途・引数を知らせるものです。実際にファイル一覧を取得するのは、`tool.rs` に書いたRustのプログラムです。

コードを変更したら`cargo run`してください。実行したディレクトリでlsしたのと同じように、ファイル一覧が出力されるはずです。

#### 寄り道：ツールをテストから直接動かす

ここで少し寄り道して、Rustのテストコードから `list_files` を直接呼び出してみましょう。Ollamaを起動しなくても試せます。

テストに必要なファイルは、一時ディレクトリにテスト自身が用意します。これによって、実行環境にどんなファイルがあるかに依存せず、同じ条件で動作を確認できます。

まず、`mini-coding-agent` ディレクトリで、一時ディレクトリを扱うクレートをテスト用の依存関係として追加してください。

```sh
cargo add tempfile --dev
```

`src/tool.rs` の中身を以下のように変更してください。`src/main.rs` の変更は不要です。

[テストを追加した `tool.rs` 全文](steps/list_files_test/tool.rs)

`mini-coding-agent` ディレクトリで、以下を実行してください。

```sh
cargo test list_files_returns_created_entries -- --nocapture
```

`cargo test` はテスト用のプログラムを実行します。このとき通常の `main()` は実行されないので、LLMへのHTTPリクエストも送られません。`--nocapture` を付けると、成功したテストでも `println!` の出力が表示されます。

ファイル一覧が表示され、次のような結果が出れば成功です。

```text
test tool::tests::list_files_returns_created_entries ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

ここで実行したのは、`main.rs` がLLMの要求を受けて呼び出すものと同じ `list_files` 関数です。LLMは使うツールと引数を選び、実際の処理はRustの関数が行います。その関数は、このようにテストからも直接実行できます。


### TUIを作る

そろそろインタラクティブに入力したくなってきたと思います。

いったん今までのツール呼び出しやLLMのことは忘れて、Claude CodeやCodexのようなターミナルでの入力画面(TUI)を作りましょう。

いままで書いていた処理はあとで戻すので安心してください。

>TUI（Terminal User Interface）は、ターミナル上で文字や枠、キー操作を使って操作するユーザーインターフェースです。


まずは必要なクレートを入れましょう。

```sh
cargo add ratatui
cargo add crossterm --features event-stream
cargo add futures-util
```

`src/main.rs` のコードを以下のように変更しましょう。

[この段階の `main.rs` 全文](steps/tui_input.rs)

コードを変更したら`cargo run`してください。TUIが立ち上がって遊べるはずです。

### TUIからLLMを呼ぶ

続いて、先ほど消してしまったHTTPリクエストを復活します。これによって、TUIでQwenと会話ができるようになります。

`src/main.rs` のコードを以下のように変更しましょう。

[この段階の `main.rs` 全文](steps/tui_http.rs)

コードを変更したら`cargo run`してこんな質問をしてみてください。

```sh
カレントディレクトリにはどんなファイルがありますか？
```

おそらく、Qwenからは答えられませんと言われるでしょう。なぜならこのCoding Agentはまだツールと繋がっていないからです。

### TUIからツールを使う

続いて、ツールを呼び出す処理を復活させます。

`src/main.rs` のコードを以下のように変更しましょう。

[この段階の `main.rs` 全文](steps/tui_list_files.rs)

コードを変更したら`cargo run`して立ち上がったTUIにこんな質問をしてみてください。

```sh
カレントディレクトリにはどんなファイルがありますか？
```

うまくいっていれば、今いるディレクトリ配下のファイルが表示されるはずです。

しかし、今はまだこの一つしかツールを持っていないので、ファイルを表示する以上のことはできません。

### ツールを増やす

続いて、このCoding Agentにツールを追加します。

```
 list_files(path):  指定したディレクトリの直下にあるファイル・ディレクトリ名を一覧にする(既存)
 read_file(path):  指定したファイルをテキストとして読む 
 write_file(path, content):  ファイルを作成する。既存のファイルなら内容を上書きする 
 run_command(program, args):  プロジェクトのディレクトリでプログラムを実行し、終了コード・標準出力・標準エラーを返す。30秒でタイムアウトする 
```

`src/main.rs` のコードを以下のように変更しましょう。

[この段階の `main.rs` 全文](steps/file_and_command_tools.rs)

コードを変更したら`cargo run`して立ち上がったTUIにこんな質問をしてみてください。

```sh
カレントディレクトリにはどんなファイルがありますか？
カレントディレクトリにhello.mdというファイルを作成し、本文に、你好！と書いてください
カレントディレクトリのhello.mdの中にはなんと書いてありますか?
shで12345679 × 9を計算して出力
```

それぞれがツールを呼び出していることが動作からわかると思います。

例えば最後の計算を
```sh
ollama run qwen3.5:4b --think=false
```

で起動したQwenに直接聞いてみるとshを作るより非常に時間をかけて回答してくるのがわかります。
```sh
12345679 × 9
```

### Agent Loopを組み込む

さて、ここまででさまざまなツールを呼び出せるようになりましたが、実はまだ足りません。
今のところ、このCoding Agentはツールを使うことはできても、それぞれの会話ターンが独立しており、文脈を保持していません。

```
まず list_files でカレントディレクトリを確認し、次に Cargo.toml を read_file で読んで、パッケージ名を答えて
```
のように複数のツールが必要となる動作や、一つ前の会話を踏まえた動作をしません。

これを解決するためにAgent Loopを組み込みます。

`src/main.rs` のコードを以下のように変更しましょう。

[この段階の `main.rs` 全文](steps/agent_loop.rs)

コードを変更したら`cargo run`して立ち上がったTUIにこんな質問をしてみてください。

```sh
shで、引数に渡った数値の素数判定するプログラム書いておいて
# agentがコーディングするのをまつ
今作ったプログラムの中身みせて
今作ったプログラムに、67を渡して素数判定して
# 実行結果をまつ
```

ここまでで、LLMに質問するだけだったプログラムが、ファイルを読み書きし、コマンドを実行し、その結果をもとに次の操作を決められるようになりました。これが今回作りたかった小さいCoding Agentです。

もちろん、これでCodexと同じものができたわけではありません。では、Codexはこの仕組みをどう実装しているのでしょうか。ここからは自分たちで書いたコードを手がかりに、Codexの中身を読んでいきます。
