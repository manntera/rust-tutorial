.PHONY: check test clippy coverage all

# すべてのチェックを一括実行
all: test clippy coverage
	@echo "すべてのチェックが完了しました！"

# テスト実行
test:
	@echo "=== テスト実行 ==="
	cargo test

# Clippy実行
clippy:
	@echo "=== Clippy実行 ==="
	cargo clippy -- -D warnings

# テストカバレッジ（cargo-tarpaulinが必要）
coverage:
	@echo "=== テストカバレッジ測定 ==="
	@if ! command -v cargo-tarpaulin >/dev/null 2>&1; then \
		echo "cargo-tarpaulinをインストールしています..."; \
		cargo install cargo-tarpaulin; \
	fi
	cargo tarpaulin --out Html

# 個別実行用
check: clippy test