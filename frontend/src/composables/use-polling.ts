import { ref, onUnmounted } from 'vue'

export function usePolling(fn: () => Promise<void>, intervalMs = 5000) {
  const active = ref(false)
  let timer: ReturnType<typeof setInterval> | null = null

  function start() {
    if (active.value) return
    active.value = true
    timer = setInterval(async () => {
      try { await fn() } catch { /* ignore */ }
    }, intervalMs)
  }

  function stop() {
    active.value = false
    if (timer) {
      clearInterval(timer)
      timer = null
    }
  }

  onUnmounted(stop)

  return { active, start, stop }
}
