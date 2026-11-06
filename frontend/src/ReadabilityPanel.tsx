import { useState } from 'react'

type Readability = { fontSize: number; lineHeight: number; maxWidth: number }

const STORAGE_KEY = 'feeder-readability'
const DEFAULTS: Readability = { fontSize: 16, lineHeight: 1.9, maxWidth: 70 }
const LIMITS = {
  fontSize: { min: 13, max: 22 },
  lineHeight: { min: 1.5, max: 2.4 },
  maxWidth: { min: 40, max: 90 },
} as const

function bounded(value: unknown, min: number, max: number, fallback: number): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) return fallback
  return Math.min(max, Math.max(min, value))
}

function parseReadability(value: string | null): Readability {
  if (!value) return DEFAULTS
  try {
    const parsed: unknown = JSON.parse(value)
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return DEFAULTS
    const stored = parsed as Record<string, unknown>
    return {
      fontSize: bounded(stored.fontSize, LIMITS.fontSize.min, LIMITS.fontSize.max, DEFAULTS.fontSize),
      lineHeight: bounded(stored.lineHeight, LIMITS.lineHeight.min, LIMITS.lineHeight.max, DEFAULTS.lineHeight),
      maxWidth: bounded(stored.maxWidth, LIMITS.maxWidth.min, LIMITS.maxWidth.max, DEFAULTS.maxWidth),
    }
  } catch {
    return DEFAULTS
  }
}

function readReadability(): { value: Readability; available: boolean } {
  if (typeof window === 'undefined') return { value: DEFAULTS, available: true }
  try {
    return { value: parseReadability(window.localStorage.getItem(STORAGE_KEY)), available: true }
  } catch {
    return { value: DEFAULTS, available: false }
  }
}

function applyValues(value: Readability): void {
  if (typeof document === 'undefined') return
  const root = document.documentElement
  root.style.setProperty('--readability-font-size', `${value.fontSize}px`)
  root.style.setProperty('--readability-line-height', String(value.lineHeight))
  root.style.setProperty('--readability-max-width', `${value.maxWidth}ch`)
}

/** Apply browser-local readability settings before the parent renders the reader. */
export function applyReadability(): void {
  const { value } = readReadability()
  applyValues(value)
}

export default function ReadabilityPanel() {
  const [initial] = useState(readReadability)
  const [value, setValue] = useState(initial.value)
  const [status, setStatus] = useState(initial.available ? '' : 'このブラウザでは設定を保存できません。変更はこのページを開いている間だけ有効です。')

  function update(next: Readability): void {
    setValue(next)
    applyValues(next)
    try {
      if (typeof window === 'undefined') throw new Error('Browser storage is unavailable')
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(next))
      setStatus('読みやすさの設定をこのブラウザに保存しました。')
    } catch {
      setStatus('設定を適用しましたが、このブラウザには保存できません。変更はこのページを開いている間だけ有効です。')
    }
  }

  function reset(): void {
    update(DEFAULTS)
  }

  return <section className="readability-panel" aria-labelledby="readability-heading">
    <h2 id="readability-heading" tabIndex={-1}>記事の読みやすさ</h2>
    <p>設定はこのブラウザに保存されます。</p>
    <label className="readability-control" htmlFor="readability-font-size">
      <span>文字サイズ</span>
      <output htmlFor="readability-font-size">{value.fontSize} px</output>
      <input id="readability-font-size" type="range" min={LIMITS.fontSize.min} max={LIMITS.fontSize.max} step="1" value={value.fontSize}
        aria-valuetext={`${value.fontSize} ピクセル`} onChange={event => update({ ...value, fontSize: bounded(Number(event.target.value), LIMITS.fontSize.min, LIMITS.fontSize.max, DEFAULTS.fontSize) })} />
    </label>
    <label className="readability-control" htmlFor="readability-line-height">
      <span>行間</span>
      <output htmlFor="readability-line-height">{value.lineHeight.toFixed(1)}</output>
      <input id="readability-line-height" type="range" min={LIMITS.lineHeight.min} max={LIMITS.lineHeight.max} step="0.1" value={value.lineHeight}
        aria-valuetext={`${value.lineHeight.toFixed(1)} 倍`} onChange={event => update({ ...value, lineHeight: bounded(Number(event.target.value), LIMITS.lineHeight.min, LIMITS.lineHeight.max, DEFAULTS.lineHeight) })} />
    </label>
    <label className="readability-control" htmlFor="readability-max-width">
      <span>本文の最大幅</span>
      <output htmlFor="readability-max-width">{value.maxWidth} ch</output>
      <input id="readability-max-width" type="range" min={LIMITS.maxWidth.min} max={LIMITS.maxWidth.max} step="1" value={value.maxWidth}
        aria-valuetext={`${value.maxWidth} 文字分`} onChange={event => update({ ...value, maxWidth: bounded(Number(event.target.value), LIMITS.maxWidth.min, LIMITS.maxWidth.max, DEFAULTS.maxWidth) })} />
    </label>
    <button className="readability-reset" type="button" onClick={reset}>初期値に戻す</button>
    <p className="readability-status" role="status" aria-live="polite">{status}</p>
  </section>
}
