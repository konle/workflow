<template>
  <a-result status="403" title="403" subtitle="抱歉，您没有权限访问此页面">
    <template #extra>
      <a-button type="primary" @click="$router.push('/')">返回首页</a-button>
    </template>
  </a-result>
</template>
