#!/bin/sh
# shx installer — GitHub Releases からバイナリをダウンロードしてインストール
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/hrtk91/shx/master/install.sh | sh
#
# Options (環境変数):
#   SHX_INSTALL_DIR  インストール先 (default: /usr/local/bin)
#   SHX_VERSION      バージョン指定 (default: latest)

set -eu

REPO="hrtk91/shx"
INSTALL_DIR="${SHX_INSTALL_DIR:-/usr/local/bin}"

# --- OS / Arch 検出 ---
detect_target() {
  os=$(uname -s)
  arch=$(uname -m)

  case "$os" in
    Linux)
      # musl 版を優先（静的リンクで互換性が高い）
      case "$arch" in
        x86_64)  echo "x86_64-unknown-linux-musl" ;;
        aarch64) echo "aarch64-unknown-linux-gnu" ;;
        *)       echo "unsupported: $os/$arch" >&2; exit 1 ;;
      esac
      ;;
    Darwin)
      case "$arch" in
        x86_64)  echo "x86_64-apple-darwin" ;;
        arm64)   echo "aarch64-apple-darwin" ;;
        *)       echo "unsupported: $os/$arch" >&2; exit 1 ;;
      esac
      ;;
    *)
      echo "unsupported OS: $os" >&2
      exit 1
      ;;
  esac
}

# --- バージョン取得 ---
get_version() {
  if [ -n "${SHX_VERSION:-}" ]; then
    echo "$SHX_VERSION"
    return
  fi
  # GitHub API で最新タグを取得
  curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' \
    | head -1 \
    | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
}

main() {
  target=$(detect_target)
  version=$(get_version)
  archive="shx-${version}-${target}.tar.gz"
  url="https://github.com/$REPO/releases/download/${version}/${archive}"

  echo "shx ${version} (${target}) をインストールします..."

  tmpdir=$(mktemp -d)
  trap "rm -rf $tmpdir" EXIT

  echo "ダウンロード中: $url"
  curl -fsSL "$url" -o "$tmpdir/$archive"

  echo "展開中..."
  tar xzf "$tmpdir/$archive" -C "$tmpdir"

  echo "インストール先: $INSTALL_DIR"
  install -m 755 "$tmpdir/shx-${version}-${target}/shx" "$INSTALL_DIR/shx"
  install -m 755 "$tmpdir/shx-${version}-${target}/bashx" "$INSTALL_DIR/bashx"

  echo ""
  echo "インストール完了!"
  echo "  shx:   $(command -v shx || echo "$INSTALL_DIR/shx")"
  echo "  bashx: $(command -v bashx || echo "$INSTALL_DIR/bashx")"
  echo ""
  "$INSTALL_DIR/shx" --version
}

main
