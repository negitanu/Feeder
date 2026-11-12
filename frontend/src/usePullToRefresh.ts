import { useEffect, useState } from 'react'

const THRESHOLD = 80
const START_DISTANCE = 12

type PullState = 'idle' | 'pulling' | 'ready' | 'refreshing'

function standalone() {
  return window.matchMedia('(display-mode: standalone)').matches ||
    (navigator as Navigator & { standalone?: boolean }).standalone === true
}

function blockedTarget(target: EventTarget | null) {
  if (!(target instanceof Element)) return true
  if (target.closest('input, textarea, select, [contenteditable="true"], .sidebar, .article-detail, .modal-backdrop, .feed-menu, [role="dialog"], [data-order-handle]')) return true
  for (let element: Element | null = target; element && element !== document.body; element = element.parentElement) {
    const style = window.getComputedStyle(element)
    if (/(auto|scroll)/.test(style.overflowY) && element.scrollHeight > element.clientHeight + 1) return true
  }
  return false
}

export function usePullToRefresh(enabled: boolean): PullState {
  const [state, setState] = useState<PullState>('idle')
  useEffect(() => {
    if (!enabled || !standalone()) { setState('idle'); return }
    let start: { x: number; y: number; active: boolean; ready: boolean } | null = null
    let refreshing = false
    const cancel = () => { start = null; if (!refreshing) setState('idle') }
    const onStart = (event: TouchEvent) => {
      if (refreshing || event.touches.length !== 1 || window.scrollY > 0 || blockedTarget(event.target)) { cancel(); return }
      const touch = event.touches[0]
      start = { x: touch.clientX, y: touch.clientY, active: false, ready: false }
    }
    const onMove = (event: TouchEvent) => {
      if (!start || event.touches.length !== 1 || window.scrollY > 0) { cancel(); return }
      const touch = event.touches[0], dx = touch.clientX - start.x, dy = touch.clientY - start.y
      if (!start.active && (dy < 0 || Math.abs(dx) > START_DISTANCE && Math.abs(dx) >= dy)) { cancel(); return }
      if (dy <= START_DISTANCE) { if (start.active) { start.ready = false; setState('idle') } return }
      if (!start.active && dy <= Math.abs(dx) * 1.5) return
      start.active = true
      start.ready = dy >= THRESHOLD
      if (event.cancelable) event.preventDefault()
      setState(start.ready ? 'ready' : 'pulling')
    }
    const onEnd = (event: TouchEvent) => {
      if (event.touches.length === 0 && start?.active && start.ready) {
        refreshing = true; start = null; setState('refreshing'); window.location.reload(); return
      }
      cancel()
    }
    document.addEventListener('touchstart', onStart, { passive: true })
    document.addEventListener('touchmove', onMove, { passive: false })
    document.addEventListener('touchend', onEnd, { passive: true })
    document.addEventListener('touchcancel', cancel, { passive: true })
    return () => {
      document.removeEventListener('touchstart', onStart)
      document.removeEventListener('touchmove', onMove)
      document.removeEventListener('touchend', onEnd)
      document.removeEventListener('touchcancel', cancel)
    }
  }, [enabled])
  return state
}
