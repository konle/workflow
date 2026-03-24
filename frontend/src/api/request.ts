import axios from 'axios'
import { getToken, clearToken } from '../utils/token'
import { Notification } from '@arco-design/web-vue'
import router from '../router'

const instance = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || '/api/v1',
  timeout: 30000,
})

instance.interceptors.request.use(config => {
  const token = getToken()
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

instance.interceptors.response.use(
  response => {
    const { code, message, data } = response.data
    if (code !== 0) {
      Notification.error({ title: '请求失败', content: message })
      return Promise.reject(new Error(message))
    }
    return { message, data } as any
  },
  error => {
    if (error.response?.status === 401) {
      clearToken()
      router.push('/login')
      Notification.error({ title: '登录过期', content: '请重新登录' })
    } else if (error.response?.status === 403) {
      Notification.error({ title: '无权限', content: '您没有执行此操作的权限' })
    } else if (error.response?.status === 404) {
      Notification.error({ title: '未找到', content: '请求的资源不存在' })
    } else {
      Notification.error({ title: '请求错误', content: error.message || '网络异常' })
    }
    return Promise.reject(error)
  }
)

export default instance
