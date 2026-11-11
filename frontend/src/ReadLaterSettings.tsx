import { useEffect, useState } from 'react'
import { apiFetch } from './apiFetch'

export default function ReadLaterSettings() {
  const [autoRemove, setAutoRemove] = useState<boolean | null>(null)
  const [pending, setPending] = useState(false)
  const [error, setError] = useState('')
  useEffect(() => {
    const controller = new AbortController()
    apiFetch('/api/read-later/settings', { signal: controller.signal })
      .then(async response => {
        if (!response.ok) throw new Error()
        const value: unknown = await response.json()
        if (!value || typeof value !== 'object' || !('auto_remove' in value) || typeof value.auto_remove !== 'boolean') throw new Error()
        if (!controller.signal.aborted) setAutoRemove(value.auto_remove)
      })
      .catch(() => { if (!controller.signal.aborted) setError('設定を読み込めませんでした') })
    return () => controller.abort()
  }, [])
  async function change(value: boolean) {
    setPending(true); setError('')
    try {
      const response = await apiFetch('/api/read-later/settings', { method: 'PATCH', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ auto_remove: value }) })
      if (!response.ok) throw new Error()
      const result = await response.json() as { auto_remove: boolean }
      setAutoRemove(result.auto_remove)
    } catch { setError('設定を保存できませんでした') } finally { setPending(false) }
  }
  return <section className="collection-section"><h2>あとで読む</h2><label><input type="checkbox" checked={autoRemove ?? false} disabled={autoRemove === null || pending} onChange={event => void change(event.target.checked)} /> 記事を開いたときにあとで読むから自動で削除する</label>{error && <p role="alert">{error}</p>}</section>
}
