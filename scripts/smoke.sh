#!/bin/bash
set -euox pipefail

HERE=$(
    CDPATH=
    cd -- "$(dirname -- "$0")" && pwd -P
)
ROOT=$(cd -- "$HERE/.." && pwd -P)
(cd "$ROOT" && cargo install --path .)

tmp=""
cleanup() {
    if [[ -n "${tmp:-}" && -d "$tmp" ]]; then
        cd /
        rm -rf -- "$tmp"
    fi
    return 0
}
trap cleanup EXIT

expect_fail() {
    if "$@"; then
        echo "!! 意外成功: $*"
        exit 1
    fi
}

tmp=$(mktemp -d)
cd "$tmp"

dp task add --help | grep -q -- '--deps <IDS>' || {
    echo "陈旧二进制!先 cargo install --path ."
    exit 1
}

dp init
dp task add "test 1"
dp task add "test 2"
dp task add "test 3"
dp task mod 3 --deps 1,2
dp task show 3 # 多行 Depends on: #1/#2 (todo, 未满足)
dp task mod 3 --deps ""
dp task show 3 # 无 Depends on 段 = 清空成功
dp task add "smoke" --deps 1,2
dp task mod 3 --deps 1,2
dp task show 1 # Blocks: #3, #4  ← dependents_of 方向,黑盒测试没盖到
dp task list --format markdown

expect_fail dp task mod 3 --deps 3   # 自依赖
expect_fail dp task mod 3 --deps 999 # 不存在
expect_fail dp task mod 1 --deps 4   # 环:4 已依赖 1,再让 1 依赖 4 必拒

echo "=== fresh-db smoke OK ==="

# --- 0.19.0 plan/adr mod ---
dp plan add "window test" --start 2026-11-01 --due 2026-11-30 >/dev/null
dp plan add "second" >/dev/null
dp plan mod 1 --due 2026-12-15 -p urgent --title "window test v2"
dp plan show 1 | grep -q "2026-12-15"
dp plan mod 1 --status archived
dp plan list | head -4      # second(open) 应排在 #1(archived) 前 ← 排序修复
dp plan mod 1 --status open # 归途
dp adr add "t" --context c --decision d >/dev/null
dp adr mod 1 --consequence "revised" --title "t2"
dp adr show 1 | grep -q revised
dp plan mod 1                            # Nothing to modify: no fields given
dp plan mod 1 --due 2026-12-15           # Nothing to modify: values already as requested
expect_fail dp plan mod 1 --adr 999      # ADR #999 not found
expect_fail dp plan mod 1 --status bogus # usage error (exit 2)

echo "=== plan/adr mod smoke OK ==="
