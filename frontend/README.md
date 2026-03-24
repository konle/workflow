# Workflow Engine Frontend

工作流引擎前端，基于 Vue 3 + TypeScript + Arco Design Vue 构建。

## 开发

```bash
# 安装依赖
npm install

# 启动开发服务器 (localhost:5173，自动代理 /api → localhost:3000)
npm run dev

# 类型检查
npx vue-tsc --noEmit

# 生产构建
npm run build

# 预览构建产物
npm run preview
```

## 技术栈

| 依赖 | 说明 |
|------|------|
| Vue 3 + Composition API | UI 框架 |
| TypeScript | 类型安全 |
| Vite | 构建工具 |
| Arco Design Vue | UI 组件库 |
| Vue Router 4 | 路由 |
| Pinia | 状态管理 |
| Axios | HTTP 客户端 |
| Vue Flow | DAG 可视化画布 |
| dagre | 自动布局算法 |

详细设计参见 [前端架构文档](../docs/frontend-architecture.md)。
