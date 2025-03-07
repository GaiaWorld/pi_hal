# Pi-HAL 图形渲染库

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.65%2B-blue)](https://www.rust-lang.org)

Pi-HAL 是一个高性能的图形渲染库，专注于基于有向距离场（SDF）的字体和矢量图形渲染，提供跨平台的硬件抽象层支持。

## 功能特性

- 🖌️ 基于SDF的字体渲染引擎
- 🌟 支持多重特效：
  - 文字阴影（可调半径和权重）
  - 外发光效果
  - 高斯模糊
- 🚀 GPU加速渲染管线
- 🌐 跨平台支持：
  - 原生平台（通过wgpu）
  - WebAssembly（WebGL 2.0）
- 📦 自动纹理打包管理
- 🧵 多线程异步任务处理
- 📐 精确的几何计算（基于parry2d）

## 核心组件

```rust
src/
├── font/              // 字体渲染核心模块
│   ├── sdf2_table.rs   // SDF字体处理主逻辑
│   ├── sdf_gpu.rs      // GPU加速实现
│   ├── blur.rs         // 模糊算法实现
│   └── text_pack.rs    // 纹理打包管理
│
├── hal/               // 硬件抽象层
│   ├── native/        // 原生平台实现
│   └── web/           // WebAssembly实现
│
└── svg/               // SVG矢量图形支持
```

## 快速开始

### 依赖安装

在Cargo.toml中添加：
```toml
[dependencies]
pi-hal = { git = "https://github.com/your-repo/pi-hal" }
```

### 基础用法
```rust
use pi_hal::{Sdf2Table, FontFaceId, FontInfo};

// 初始化渲染上下文
let mut sdf_table = Sdf2Table::new(1024, 1024, device, queue);

// 加载字体
sdf_table.add_font(FontFaceId(0), font_buffer);

// 创建文字样式
let font_info = FontInfo {
    font_size: 32.0,
    // ...其他参数
};

// 获取字形度量
let metrics = sdf_table.metrics(glyph_id, &font_info);
```

## 高级特性

### GPU加速配置
```rust
// 启用GPU加速（自动检测平台）
let gpu_state = GPUState::init(device, queue);
```

### 特效应用
```rust
// 添加文字阴影
sdf_table.add_font_shadow(
    glyph_id,
    &font_info,
    radius = 5,
    weight = NotNan::new(0.8).unwrap()
);

// 添加外发光
sdf_table.add_font_outer_glow(glyph_id, &font_info, 3);
```

## 性能优化

- 多级缓存系统（内存 + 持久化存储）
- 自动字形复用
- 批处理渲染
- 异步纹理上传

