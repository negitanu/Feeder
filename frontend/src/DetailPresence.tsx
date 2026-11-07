import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import type { ReactNode } from 'react'

export default function DetailPresence({ open, children }: { open: boolean; children: ReactNode }) {
  const [present, setPresent] = useState(open)
  const retained = useRef<ReactNode>(null)
  useLayoutEffect(() => { if (open) retained.current = children }, [open, children])
  useEffect(() => {
    if (open) { setPresent(true); return }
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) { setPresent(false); return }
    const timer = window.setTimeout(() => setPresent(false), 160)
    return () => window.clearTimeout(timer)
  }, [open])
  if (!open && !present) return null
  return <div className={`detail-presence ${open ? 'is-open' : 'is-leaving'}`} inert={!open} aria-hidden={!open || undefined}>{open ? children : retained.current}</div>
}
