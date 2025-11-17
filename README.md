# Polymarket Scanner

一个基于 Rust 的 Polymarket 市场扫描器，用于实时监控预测市场的变化。

## 功能特性

- 🔍 **实时监控** - 持续扫描 Polymarket 活跃市场
- 📊 **市场发现** - 自动发现新上线的市场
- 💹 **价格追踪** - 监控市场价格变化
- 📈 **成交量监控** - 追踪市场成交量变化
- 🔔 **事件通知** - 实时显示市场事件
- ⚡ **高性能** - 基于 Tokio 异步运行时
- 💾 **数据库存储** - SQLite 数据库持久化历史数据
- 📄 **分页扫描** - 支持分页获取所有市场数据

## 安装

确保你已经安装了 Rust 工具链（推荐使用 rustup）：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

克隆并构建项目：

```bash
git clone <repository-url>
cd polymarket_scanner
cargo build --release
```

## 使用方法

### 基本使用

直接运行扫描器：

```bash
cargo run
```

或使用编译后的二进制文件：

```bash
./target/release/polymarket_scanner
```

### 配置日志级别

设置环境变量来控制日志输出：

```bash
# 详细日志
RUST_LOG=debug cargo run

# 仅显示重要信息
RUST_LOG=info cargo run

# 仅显示错误
RUST_LOG=error cargo run
```

### 环境变量配置

复制 `.env.example` 到 `.env` 并根据需要修改配置：

```bash
cp .env.example .env
```

可配置的环境变量：
- `RUST_LOG` - 日志级别（trace, debug, info, warn, error）
- `SCAN_INTERVAL` - 扫描间隔（秒）
- `API_TIMEOUT` - API 请求超时时间（秒）
- `MAX_MARKETS` - 最大获取市场数量

## 项目结构

```
polymarket_scanner/
├── src/
│   ├── main.rs          # 程序入口
│   ├── lib.rs           # 库入口
│   ├── api.rs           # Polymarket API 客户端
│   ├── scanner.rs       # 市场扫描器逻辑
│   ├── types.rs         # 数据类型定义
│   ├── error.rs         # 错误处理
│   └── database.rs      # 数据库模块
├── examples/            # 示例程序
│   ├── fetch_markets.rs          # 获取市场列表
│   ├── basic_scan.rs             # 基础扫描
│   ├── scan_with_database.rs    # 带数据库的扫描
│   ├── scan_all_markets.rs      # 扫描所有市场
│   ├── query_database.rs        # 查询数据库
│   └── export_markets.rs        # 导出数据
├── Cargo.toml           # 项目依赖配置
├── .env.example         # 环境变量示例
├── .gitignore          # Git 忽略文件
├── README.md           # 项目文档
└── DATABASE_GUIDE.md   # 数据库使用指南
```

## 核心模块

### API 客户端 (PolymarketClient)

提供与 Polymarket API 交互的方法：

- `get_markets()` - 获取活跃市场列表
- `get_markets_paginated()` - 分页获取市场
- `get_all_markets()` - 自动分页获取所有市场
- `get_market()` - 获取单个市场详情
- `get_price_history()` - 获取价格历史数据
- `get_market_stats()` - 获取市场统计信息

### 数据库模块 (Database)

提供数据持久化功能：

- `new()` - 创建数据库连接
- `init()` - 初始化表结构
- `save_market()` - 保存市场数据
- `save_event()` - 保存市场事件
- `save_price_history()` - 保存价格历史
- `get_market()` - 查询市场数据
- `get_market_events()` - 查询事件历史
- `get_price_history()` - 查询价格历史
- `count_markets()` - 统计市场数量

### 扫描器模块 (MarketScanner)

市场监控核心逻辑：

- `new()` - 创建扫描器
- `with_database()` - 创建带数据库的扫描器
- `start_scanning()` - 开始持续扫描
- `scan_all_markets()` - 一次性扫描所有市场

### 示例

```rust
use polymarket_scanner::api::PolymarketClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PolymarketClient::new()?;
    
    // 获取前 10 个活跃市场
    let markets = client.get_markets(Some(10)).await?;
    
    for market in markets {
        println!("市场: {}", market.question);
        println!("价格: {:?}", market.outcome_prices);
    }
    
    Ok(())
}
```

## 扫描器功能

扫描器会自动检测以下事件：

1. **新市场** - 新上线的预测市场
2. **价格变化** - 市场价格波动
3. **成交量更新** - 交易量变化
4. **市场关闭** - 市场结束或关闭

## 数据类型

### Market

```rust
pub struct Market {
    pub condition_id: String,
    pub question: String,
    pub description: Option<String>,
    pub outcomes: Vec<String>,
    pub outcome_prices: Vec<String>,
    pub volume: Option<String>,
    pub liquidity: Option<String>,
    pub end_date: Option<String>,
    pub active: Option<bool>,
    pub closed: Option<bool>,
}
```

## 技术栈

- **Rust** - 系统编程语言
- **Tokio** - 异步运行时
- **Reqwest** - HTTP 客户端
- **Serde** - 序列化/反序列化
- **Chrono** - 时间处理
- **Log/env_logger** - 日志系统

## 开发

### 运行测试

```bash
cargo test
```

### 代码格式化

```bash
cargo fmt
```

### 代码检查

```bash
cargo clippy
```

## 注意事项

- 请遵守 Polymarket API 的使用限制和条款
- 建议设置合理的扫描间隔，避免过于频繁的请求
- 生产环境使用时建议添加错误重试机制

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

MIT License

## 相关链接

- [Polymarket 官网](https://polymarket.com)
- [Polymarket API 文档](https://docs.polymarket.com)
- [Rust 官网](https://www.rust-lang.org)

## 作者

be1uga

---

**免责声明**: 此工具仅供学习和研究使用，不构成投资建议。使用者需自行承担使用本工具的所有风险。

