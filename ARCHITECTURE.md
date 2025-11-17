# 项目架构

本文档描述 Polymarket Scanner 的整体架构和设计。

## 架构概览

```
┌─────────────────────────────────────────────────┐
│                   Main Entry                     │
│                  (main.rs)                       │
└───────────────┬─────────────────────────────────┘
                │
                ├─── 初始化日志
                ├─── 创建 API 客户端
                └─── 启动扫描器
                        │
        ┌───────────────┴───────────────┐
        │                               │
        ▼                               ▼
┌──────────────────┐          ┌──────────────────┐
│  API 客户端层     │          │   扫描器层        │
│   (api.rs)       │◄─────────│  (scanner.rs)    │
└──────────────────┘          └──────────────────┘
        │                               │
        ├─ get_markets()               ├─ scan_markets()
        ├─ get_market()                ├─ handle_event()
        ├─ get_price_history()         └─ print_market_info()
        └─ get_market_stats()
        │
        ▼
┌──────────────────┐
│  Polymarket API  │
│  (HTTPS REST)    │
└──────────────────┘
```

## 核心模块

### 1. main.rs - 程序入口

**职责:**
- 初始化应用程序环境
- 设置日志系统
- 协调各模块工作

**流程:**
1. 初始化 `env_logger`
2. 创建 `PolymarketClient` 实例
3. 创建 `MarketScanner` 实例
4. 启动扫描循环

### 2. api.rs - API 客户端层

**职责:**
- 封装与 Polymarket API 的所有交互
- 处理 HTTP 请求和响应
- 数据序列化/反序列化

**核心方法:**

```rust
impl PolymarketClient {
    pub fn new() -> Result<Self>
    pub async fn get_markets(&self, limit: Option<u32>) -> Result<Vec<Market>>
    pub async fn get_market(&self, condition_id: &str) -> Result<Market>
    pub async fn get_price_history(&self, ...) -> Result<Vec<PriceHistory>>
    pub async fn get_market_stats(&self, condition_id: &str) -> Result<Value>
}
```

**使用的 API 端点:**
- `GET /markets` - 获取市场列表
- `GET /markets/{id}` - 获取市场详情
- `GET /markets/{id}/stats` - 获取市场统计

### 3. scanner.rs - 市场扫描器

**职责:**
- 持续监控市场变化
- 检测并分类市场事件
- 格式化和显示市场信息

**核心逻辑:**

```rust
impl MarketScanner {
    pub fn new(client: PolymarketClient) -> Self
    
    // 主扫描循环
    pub async fn start_scanning(&self, interval: Duration) -> Result<()>
    
    // 扫描并检测变化
    async fn scan_markets(&self, tracked: &mut HashMap<...>) -> Result<Vec<MarketEvent>>
    
    // 处理事件
    fn handle_event(&self, event: MarketEvent)
}
```

**事件检测机制:**
1. **新市场**: 首次出现的 condition_id
2. **价格变化**: outcome_prices 数组变化
3. **成交量更新**: volume 字段变化
4. **市场关闭**: closed 字段从 false 变为 true

### 4. types.rs - 数据类型定义

**职责:**
- 定义领域模型
- 提供数据结构
- 支持 JSON 序列化

**核心类型:**

```rust
// 市场数据
pub struct Market { ... }

// 市场事件
pub struct MarketEvent {
    pub market: Market,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
}

// 事件类型
pub enum EventType {
    NewMarket,
    PriceChange,
    VolumeUpdate,
    MarketClosed,
}
```

### 5. error.rs - 错误处理

**职责:**
- 统一错误类型定义
- 错误转换和传播
- 提供友好的错误信息

**错误类型:**

```rust
pub enum ScannerError {
    ApiError(reqwest::Error),
    JsonError(serde_json::Error),
    InvalidResponse(String),
    NetworkError(String),
    ConfigError(String),
}
```

## 数据流

### 扫描循环数据流

```
1. 定时触发
   │
   ▼
2. 调用 API 获取市场列表
   │
   ▼
3. 与缓存数据比对
   │
   ├─► 新市场 → NewMarket 事件
   ├─► 价格变化 → PriceChange 事件
   ├─► 成交量变化 → VolumeUpdate 事件
   └─► 市场关闭 → MarketClosed 事件
   │
   ▼
4. 更新本地缓存
   │
   ▼
5. 触发事件处理
   │
   ▼
6. 显示/记录事件信息
   │
   ▼
7. 等待下一个周期
```

## 异步架构

项目使用 Tokio 作为异步运行时：

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 异步主函数
}
```

**异步操作:**
- HTTP 请求 (reqwest)
- 定时器 (tokio::time::sleep)
- 未来可扩展: WebSocket 连接、数据库操作等

## 扩展点

### 1. 事件处理器

可以实现不同的事件处理器：

```rust
trait EventHandler {
    fn handle(&self, event: MarketEvent);
}

// 控制台输出
struct ConsoleHandler;

// 文件记录
struct FileHandler;

// 数据库存储
struct DatabaseHandler;

// Webhook 通知
struct WebhookHandler;
```

### 2. 过滤器

添加市场过滤逻辑：

```rust
trait MarketFilter {
    fn should_track(&self, market: &Market) -> bool;
}

// 按关键词过滤
struct KeywordFilter;

// 按成交量过滤
struct VolumeFilter;

// 按类别过滤
struct CategoryFilter;
```

### 3. 存储层

添加持久化支持：

```rust
trait Storage {
    async fn save_market(&self, market: &Market) -> Result<()>;
    async fn save_event(&self, event: &MarketEvent) -> Result<()>;
    async fn get_history(&self, condition_id: &str) -> Result<Vec<MarketEvent>>;
}
```

## 配置管理

当前配置通过环境变量：

```rust
// 未来可以使用配置文件
pub struct Config {
    pub scan_interval: Duration,
    pub api_timeout: Duration,
    pub max_markets: u32,
    pub log_level: String,
}
```

## 性能考虑

1. **并发请求**: 使用 Tokio 的异步特性，可以并行请求多个市场数据
2. **内存管理**: 使用 HashMap 缓存市场数据，避免重复请求
3. **错误恢复**: 单次扫描失败不会终止程序，继续下一轮扫描

## 安全考虑

1. **API 密钥**: 当前使用公开 API，未来如需认证可通过环境变量管理
2. **速率限制**: 建议扫描间隔不少于 5 秒
3. **数据验证**: API 响应经过 Serde 验证

## 测试策略

```
src/
├── api.rs
│   └── 单元测试: mock HTTP 响应
├── scanner.rs
│   └── 单元测试: 事件检测逻辑
├── types.rs
│   └── 单元测试: 序列化/反序列化
└── integration tests/
    └── 集成测试: 完整扫描流程
```

## 未来改进

1. **WebSocket 支持**: 实时价格更新
2. **数据库集成**: 历史数据存储
3. **图表展示**: Web UI 或 TUI
4. **智能告警**: 基于规则的通知系统
5. **策略回测**: 历史数据分析
6. **多市场支持**: 支持其他预测市场平台

## 依赖关系图

```
polymarket_scanner
├── tokio (异步运行时)
├── reqwest (HTTP 客户端)
│   └── tokio
├── serde + serde_json (序列化)
├── chrono (时间处理)
├── log + env_logger (日志)
├── anyhow (错误处理)
└── thiserror (错误定义)
```

## 最佳实践

1. **错误处理**: 使用 `Result<T>` 和 `?` 操作符
2. **异步编程**: 合理使用 `async/await`
3. **代码组织**: 模块化设计，职责清晰
4. **文档**: 代码注释和文档完善
5. **日志**: 分级日志记录

---

本文档随项目演进持续更新。

