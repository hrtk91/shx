# shx - backlog

## 概要
POSIX sh の superset 言語。制御構文をブレースベースのモダンな記法に置き換え、POSIX sh にトランスパイルする CLI ツール。

## 構文変換イメージ
```
# shx                          # POSIX sh
if [ "$x" -gt 0 ] {            if [ "$x" -gt 0 ]; then
  echo "yes"                      echo "yes"
} elif [ "$x" -eq 0 ] {        elif [ "$x" -eq 0 ]; then
  echo "zero"                    echo "zero"
} else {                        else
  echo "no"                      echo "no"
}                               fi

for i in 1 2 3 {               for i in 1 2 3; do
  echo $i                        echo $i
}                               done

while [ "$n" -lt 10 ] {        while [ "$n" -lt 10 ]; do
  n=$((n + 1))                   n=$((n + 1))
}                               done

match "$val" {                  case "$val" in
  "foo" => echo "foo"             "foo") echo "foo";;
  "bar" | "baz" => echo "both"   "bar"|"baz") echo "both";;
  _ => echo "other"               *) echo "other";;
}                               esac
```

## アーキテクチャ
```
src/
  main.rs    -- CLI エントリポイント (clap)
  lib.rs     -- public API: transpile(&str) -> Result<String>
  lexer.rs   -- トークナイザ
  parser.rs  -- トークン列 → AST
  ast.rs     -- AST 定義
  codegen.rs -- AST → POSIX sh 文字列
```

## Phase 1: MVP
- [x] プロジェクト構成 (lib/bin分離, 手動argparse)
- [x] lexer: shx トークナイザ
- [x] ast: AST 型定義
- [x] parser: トークン列 → AST
- [x] codegen: AST → POSIX sh
- [x] 対応構文
  - [x] if / elif / else ブレース
  - [x] for ブレース
  - [x] while ブレース
  - [x] match (case の置き換え)
- [x] パススルー: shx 拡張構文以外はそのまま通す
- [x] CLI: `shx input.shx` → stdout に POSIX sh 出力
- [x] CLI: `shx input.shx -o output.sh`
- [ ] E2E テスト: トランスパイル結果を dash で実行して期待出力と比較

## Phase 2: 実用
- [x] エラーメッセージ (行番号・カラム付き)
- [x] `shx --check` (構文チェックのみ)
- [x] `shx file.shx` でデフォルト実行 (`--emit` で出力のみ)
- [x] ネストした制御構文
- [x] ヒアドキュメント対応
- [x] コメント保持

## Phase 3: 発展
- [x] shebang対応 (`#!/usr/bin/env shx` → `#!/bin/sh` に変換)
- [ ] パイプライン内の制御構文
- [ ] シンタックスハイライト (tree-sitter grammar)
- [ ] POSIX sh → shx 逆変換
- [ ] 式 if/match (Rust風: `result = if [...] { "yes" } else { "no" }` → 各ブランチで変数代入に変換)

## 設計判断
- POSIX sh の superset = 既存の POSIX sh スクリプトはそのまま有効な shx
- トランスパイラ方式 = 実行は既存のシェルに委ねる (dash, bash, etc.)
- パーサーは自前実装 (sh の文法は正規文法ではないので既存パーサークレートに頼りにくい)
- **変えるのは制御構文だけ** — `${:-}` 等のパラメータ展開や変数展開はshそのまま。
  理由: 制御構文 (`fi`, `done`, `esac`, `;;`) は「読みにくいだけで学びがない」が、
  パラメータ展開は「覚える価値のあるshの語彙」。shxで隠すとshが読めない人を作るだけになる。
