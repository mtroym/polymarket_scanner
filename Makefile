.PHONY: help build run release test clean fmt check lint examples

help:
	@echo "Polymarket Scanner - 可用命令:"
	@echo ""
	@echo "  make build      - 构建调试版本"
	@echo "  make release    - 构建发布版本"
	@echo "  make run        - 运行扫描器（调试模式）"
	@echo "  make test       - 运行测试"
	@echo "  make check      - 检查代码（不生成二进制）"
	@echo "  make fmt        - 格式化代码"
	@echo "  make lint       - 运行 clippy 检查"
	@echo "  make clean      - 清理构建文件"
	@echo "  make examples   - 运行示例"
	@echo ""

build:
	cargo build

release:
	cargo build --release

run:
	RUST_LOG=info cargo run

run-debug:
	RUST_LOG=debug cargo run

test:
	cargo test

check:
	cargo check

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

clean:
	cargo clean

# 运行示例
example-fetch:
	RUST_LOG=info cargo run --example fetch_markets

example-scan:
	RUST_LOG=info cargo run --example basic_scan

example-db-scan:
	RUST_LOG=info cargo run --example scan_with_database

example-scan-all:
	RUST_LOG=info cargo run --example scan_all_markets

example-query:
	RUST_LOG=info cargo run --example query_database

example-export:
	RUST_LOG=info cargo run --example export_markets

examples: example-fetch

# 数据库相关
db-init:
	@echo "数据库将在首次运行时自动初始化"

db-query:
	sqlite3 polymarket.db "SELECT COUNT(*) as market_count FROM markets; SELECT COUNT(*) as event_count FROM market_events;"

db-clean:
	rm -f polymarket.db
	@echo "数据库已清理"

# 开发相关
dev: fmt lint check

# 完整检查
all: fmt lint test build

