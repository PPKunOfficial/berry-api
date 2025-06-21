# 更新日志

本文档记录了 Berry API 项目的所有重要变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)，
并且本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [未发布]

### 新增
- 预编译 Docker 构建流程，提升构建性能
- GitHub Actions 自动发布 Release 功能
- 二进制文件自动打包和校验和生成
- 本地测试脚本 `scripts/test-release-build.sh`

### 改进
- 优化 Docker 构建流程，减少构建时间
- 更新文档，添加详细的构建说明
- 改进 CI/CD 流水线，支持多种构建方式

### 修复
- 修复 Docker 构建中的依赖问题

## [0.1.0] - 2024-01-XX

### 新增
- 初始版本发布
- 基础的 AI API 负载均衡功能
- 支持多种 AI 服务提供商
- 健康检查和故障转移机制
- 配置热重载功能
- 监控和可观测性支持
- Docker 容器化部署
- 完整的 API 文档

### 功能特性
- **负载均衡策略**
  - 加权轮询
  - 智能 AI 模式
  - 故障转移
  
- **健康检查**
  - 主动健康检查
  - 被动健康验证
  - 渐进式恢复

- **监控支持**
  - Prometheus 指标
  - 请求追踪
  - 性能监控

- **部署方式**
  - Docker 容器
  - 二进制文件
  - 源码编译

---

## 版本说明

### 版本号格式
- **主版本号**：不兼容的 API 修改
- **次版本号**：向下兼容的功能性新增
- **修订号**：向下兼容的问题修正

### 变更类型
- **新增**：新功能
- **改进**：对现有功能的改进
- **修复**：问题修复
- **移除**：移除的功能
- **安全**：安全相关的修复

### 发布流程
1. 更新 CHANGELOG.md
2. 创建版本标签：`git tag v1.0.0`
3. 推送标签：`git push origin v1.0.0`
4. GitHub Actions 自动构建和发布

---

## 贡献指南

如果你想为本项目贡献代码，请：

1. Fork 本仓库
2. 创建特性分支：`git checkout -b feature/amazing-feature`
3. 提交更改：`git commit -m 'Add some amazing feature'`
4. 推送到分支：`git push origin feature/amazing-feature`
5. 创建 Pull Request

### 提交信息格式
```
type(scope): description

[optional body]

[optional footer]
```

**类型（type）：**
- `feat`: 新功能
- `fix`: 修复
- `docs`: 文档
- `style`: 格式
- `refactor`: 重构
- `test`: 测试
- `chore`: 构建过程或辅助工具的变动
