#!/bin/bash
# scripts/release.sh — dp 的发布序列
# 用法: ./scripts/release.sh <version> <tag-message> [commit-message]
# 例:   ./scripts/release.sh 0.13.3 "dp 0.13.3: fix something" "Release 0.13.3"
set -euo pipefail

VERSION="${1:?用法: release.sh <version> <tag-message> [commit-message]}"
VERSION="${VERSION#v}"    # 剥掉误传的 v 前缀,在 TAG= 定义之前
TAG_MSG="${2:?缺 tag message}"
COMMIT_MSG="${3:-Release ${VERSION}}"
TAG="v${VERSION}"

cd "$(dirname "$0")/.."

echo "=== [1/7] 前置检查 ==="
# 替换原步骤 1 的工作区检查:
if ! git diff --quiet -- . ':!CHANGELOG.md' || ! git diff --cached --quiet -- . ':!CHANGELOG.md'; then
    echo "✗ 有 CHANGELOG 之外的未提交改动,先 commit"; exit 1
fi

echo "=== [2/7] 质量门 ==="
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run

echo "=== [3/7] 版本号 ==="
cargo set-version "${VERSION}"
git add Cargo.toml Cargo.lock

echo "=== [4/7] 发布物验证 ==="
git add -A
git diff --cached --quiet || git commit -m "${COMMIT_MSG}"
cargo publish --dry-run

echo "=== [5/7] 推送 commit ==="
git push

echo "=== [6/7] tag(describe 确认后才推) ==="
git tag -a "${TAG}" -m "${TAG_MSG}"
DESCRIBE=$(git describe --tags)
[[ "${DESCRIBE}" == "${TAG}" ]] || { echo "✗ describe=${DESCRIBE} ≠ ${TAG},tag 未落在 HEAD"; exit 1; }
git push origin "${TAG}"

echo "=== [7/7] 发布 + 本机同步 ==="
cargo publish
cargo install --force --path .
dp generate fish > ~/.config/fish/completions/dp.fish
cp man/dp.1 ~/.local/share/man/man1/ 2>/dev/null || true

echo ""
echo "✓ devplan ${VERSION} 发布完成"
dp --version
