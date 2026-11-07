import { useEffect, useState } from 'react'
import { apiFetch } from './apiFetch'
import ReadLaterButton from './ReadLaterButton'
import './read-later.css'

type Item = { id: number; title: string; url: string; published: string; saved_at: string }
type Props = { onOpen: (id: number) => void; refreshKey?: number; onChanged?: (id: number, saved: boolean) => void }
export default function ReadLaterPanel({ onOpen, refreshKey = 0, onChanged }: Props) {
  const [items, setItems] = useState<Item[]>([])
  const [autoRemove, setAutoRemove] = useState(false)
  const [loading, setLoading] = useState(true)
  const [pending, setPending] = useState(false)
  const [error, setError] = useState('')
  useEffect(() => {
    const controller = new AbortController()
    apiFetch('/api/read-later', { signal: controller.signal })
      .then(async response => { if (!response.ok) throw new Error(); return response.json() as Promise<unknown> })
      .then(value => {
        if (!value || typeof value !== 'object' || !('items' in value) || !Array.isArray(value.items) || !value.items.every((item: unknown) => item && typeof item === 'object' && 'id' in item && Number.isSafeInteger(item.id) && 'title' in item && typeof item.title === 'string')) throw new Error()
        if (!controller.signal.aborted) { setItems(value.items as Item[]); setError('') }
      })
      .catch(() => { if (!controller.signal.aborted) setError('あとで読むを読み込めませんでした。') })
      .finally(() => { if (!controller.signal.aborted) setLoading(false) })
    return () => controller.abort()
  }, [refreshKey])
  useEffect(() => {
    const controller = new AbortController()
    apiFetch('/api/read-later/settings', { signal: controller.signal })
      .then(async response => { if (!response.ok) throw new Error(); return response.json() as Promise<unknown> })
      .then(settings => { if (!settings || typeof settings !== 'object' || !('auto_remove' in settings) || typeof settings.auto_remove !== 'boolean') throw new Error(); if (!controller.signal.aborted) setAutoRemove(settings.auto_remove) })
      .catch(() => { if (!controller.signal.aborted) setError('設定を読み込めませんでした。') })
    return () => controller.abort()
  }, [])
  async function changeSetting(value: boolean) {
    setPending(true); setError('')
    try {
      const response = await apiFetch('/api/read-later/settings', { method: 'PATCH', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ auto_remove: value }) })
      if (!response.ok) throw new Error()
      const result = await response.json() as { auto_remove: boolean }
      setAutoRemove(result.auto_remove)
    } catch { setError('設定を保存できませんでした。') } finally { setPending(false) }
  }
  return <section className="read-later-panel" aria-labelledby="read-later-title">
    <h2 id="read-later-title">あとで読む</h2>
    {error && <p className="read-later-error" role="alert">{error}</p>}
    {loading ? <p role="status">読み込み中…</p> : <>
      <label className="read-later-setting"><input type="checkbox" checked={autoRemove} disabled={pending} onChange={e => changeSetting(e.target.checked)} />記事を開いたときに自動で削除する</label>
      {items.length === 0 ? <p>保存した記事はありません。</p> : <><p>新しく保存した記事を最大100件表示します。</p><ul className="read-later-list">{items.map(item => <li key={item.id}><button type="button" className="read-later-open" onClick={() => onOpen(item.id)}>{item.title}</button><ReadLaterButton id={item.id} saved onChanged={saved => { if (!saved) setItems(current => current.filter(entry => entry.id !== item.id)); onChanged?.(item.id, saved) }} /></li>)}</ul></>}
    </>}
  </section>
}
