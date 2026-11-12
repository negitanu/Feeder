import { useEffect, useRef, useState } from 'react'
import type { CSSProperties, KeyboardEvent, PointerEvent, ReactNode } from 'react'
const MIN = 200, MAX = 480, DEFAULT = 220
function savedWidth() {
  try { const raw = localStorage.getItem('feeder-sidebar-width'); const n = raw === null ? NaN : Number(raw); return Number.isFinite(n) && n >= MIN && n <= MAX ? n : DEFAULT } catch { return DEFAULT }
}
export default function SidebarWidth({ children, onWidthChange, settings = false }: { children: ReactNode; onWidthChange: (width: number) => void; settings?: boolean }) {
  const [width, setWidth] = useState(savedWidth)
  const [available, setAvailable] = useState(MAX)
  const workspace = useRef<HTMLDivElement>(null)
  const initial = useRef<{ x: number; width: number; saved: number } | null>(null)
  useEffect(() => {
    const el = workspace.current
    if (!el) return
    const observer = new ResizeObserver(entries => setAvailable(Math.max(MIN,Math.min(MAX,entries[0].contentRect.width - 366))))
    observer.observe(el); return () => observer.disconnect()
  }, [])
  useEffect(() => { try { localStorage.setItem('feeder-sidebar-width', String(width)) } catch { /* Width remains usable in this session. */ } }, [width])
  useEffect(() => { onWidthChange(width) }, [width,onWidthChange])
  useEffect(() => {
    function imported(event: Event) {
      const value = (event as CustomEvent<unknown>).detail
      if (typeof value === 'number' && Number.isInteger(value) && value >= MIN && value <= MAX) setWidth(value)
    }
    window.addEventListener('feeder-import-sidebar-width', imported)
    return () => window.removeEventListener('feeder-import-sidebar-width', imported)
  }, [])
  const visible = Math.min(width,available)
  function change(value: number) { setWidth(Math.max(MIN,Math.min(available,value))) }
  function key(e: KeyboardEvent<HTMLDivElement>) {
    if (!['ArrowLeft','ArrowRight','Home','End'].includes(e.key)) return
    e.preventDefault(); change(e.key === 'Home' ? MIN : e.key === 'End' ? available : visible + (e.key === 'ArrowLeft' ? -1 : 1) * (e.shiftKey ? 50 : 10))
  }
  function down(e: PointerEvent<HTMLDivElement>) {
    if (!e.isPrimary || e.button !== 0) return
    initial.current = {x:e.clientX,width:visible,saved:width}; e.currentTarget.setPointerCapture(e.pointerId)
  }
  function end(e: PointerEvent<HTMLDivElement>, cancel: boolean) {
    if (cancel && initial.current) setWidth(initial.current.saved)
    initial.current = null
    if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId)
  }
  return <div className={`workspace${settings ? ' workspace-settings' : ''}`} ref={workspace} style={{ '--sidebar-width': `${visible}px` } as CSSProperties}>
    {children}
    <div className="sidebar-separator" role="separator" aria-label="サイドバーの幅" aria-orientation="vertical" aria-controls="reader-sidebar" aria-valuemin={MIN} aria-valuemax={available} aria-valuenow={visible} tabIndex={0} onKeyDown={key} onPointerDown={down} onPointerMove={e => { if (initial.current) change(initial.current.width + e.clientX - initial.current.x) }} onPointerUp={e => end(e,false)} onPointerCancel={e => end(e,true)} onLostPointerCapture={() => { initial.current = null }} />
  </div>
}
