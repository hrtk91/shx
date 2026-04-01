# shx for Visual Studio Code

[shx](https://github.com/hrtk91/shx) のシンタックスハイライト拡張。

## 機能

- `.shx` / `.bashx` ファイルのシンタックスハイライト
- 括弧・クォート・バッククォートの自動補完
- `{ }` ブロックの自動インデント
- shx 固有構文のハイライト: `match`, `=>`, 波括弧ブロック

## インストール（ローカル）

```sh
# リポジトリルートから
cd editors/vscode
code --install-extension .
```

またはシンボリックリンク:

```sh
ln -s "$(pwd)/editors/vscode" ~/.vscode/extensions/shx
```

## ライセンス

MIT
