import { useEffect, useRef, useState } from 'react'
import type { KeyboardEvent, PointerEvent } from 'react'

export const DEFAULT_READER_SPLIT = 52
const MIN_LIST = 280
const MIN_DETAIL = 320
const HANDLE = 8
const STORAGE_KEY = 'feeder-reader-split'

export function storedReaderSplit() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    const value = raw === null ? NaN : Number(raw)
    return Number.isFinite(value) && value >= 20 && value <= 80 ? value : DEFAULT_READER_SPLIT
  } catch { return DEFAULT_READER_SPLIT }
}

export default function DetailSplitter({ value, onChange }: { value: number; onChange: (value: number) => void }) {
  const ref = useRef<HTMLDivElement>(null)
  const start = useRef<number | null>(null)
  const [width, setWidth] = useState(0)
  useEffect(() => {
    const parent = ref.current?.parentElement
    if (!parent) return
    const observer = new ResizeObserver(entries => setWidth(entries[0].contentRect.width))
    observer.observe(parent)
    return () => observer.disconnect()
  }, [])
  useEffect(() => { try { localStorage.setItem(STORAGE_KEY, String(value)) } catch { /* Continue without persistence. */ } }, [value])
  const available = Math.max(1, width - HANDLE)
  const minimum = Math.min(40, MIN_LIST / available * 100)
  const maximum = Math.max(minimum, Math.min(80, 100 - Math.min(40, MIN_DETAIL / available * 100)))
  const visible = Math.max(minimum, Math.min(maximum, value))
  function set(next: number) { onChange(Math.round(Math.max(minimum, Math.min(maximum, next)) * 10) / 10) }
  function key(event: KeyboardEvent<HTMLDivElement>) {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
    event.preventDefault()
    set(event.key === 'Home' ? minimum : event.key === 'End' ? maximum : visible + (event.key === 'ArrowRight' ? 1 : -1) * (event.shiftKey ? 5 : 1))
  }
  function down(event: PointerEvent<HTMLDivElement>) {
    if (!event.isPrimary || event.button !== 0) return
    start.current = value
    event.currentTarget.setPointerCapture(event.pointerId)
    event.preventDefault()
  }
  function move(event: PointerEvent<HTMLDivElement>) {
    if (start.current === null) return
    const rect = ref.current?.parentElement?.getBoundingClientRect()
    if (rect) set((event.clientX - rect.left - HANDLE / 2) / Math.max(1, rect.width - HANDLE) * 100)
  }
  function end(event: PointerEvent<HTMLDivElement>, cancel: boolean) {
    if (cancel && start.current !== null) onChange(start.current)
    start.current = null
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId)
  }
  return <div ref={ref} className="detail-splitter" role="separator" aria-label="記事一覧と詳細の幅" aria-controls="article-list-pane" aria-orientation="vertical" aria-valuemin={Math.round(minimum)} aria-valuemax={Math.round(maximum)} aria-valuenow={Math.round(visible)} tabIndex={0} onKeyDown={key} onPointerDown={down} onPointerMove={move} onPointerUp={event => end(event, false)} onPointerCancel={event => end(event, true)} onLostPointerCapture={() => { start.current = null }} />
}
