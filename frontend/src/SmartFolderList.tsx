import { useCallback, useEffect, useState } from 'react'
import { apiFetch } from './apiFetch'
import type { SearchSnapshot } from './SavedSearches'
import './SmartFolderList.css'
type Item = { id: number; name: string; snapshot: SearchSnapshot; revision: number }
export default function SmartFolderList({ onApply, refreshKey }: { onApply: (snapshot: SearchSnapshot) => string | null; refreshKey?: number }) {
  const [items, setItems] = useState<Item[]>([])
  const [notice, setNotice] = useState('')
  const [busy, setBusy] = useState(false)
  const load = useCallback(async () => {
    try {
      const response = await apiFetch('/api/smart-folders')
      if (!response.ok) throw new Error()
      const value: unknown = await response.json()
      if (!Array.isArray(value) || !value.every(item => item && typeof item === 'object' && Number.isSafeInteger(item.id) && typeof item.name === 'string' && Number.isSafeInteger(item.revision) && item.snapshot && typeof item.snapshot === 'object')) throw new Error()
      setItems(value as Item[])
    }
    catch { setNotice('共有検索を読み込めませんでした。') }
  }, [])
  useEffect(() => { void load() }, [load, refreshKey])
  async function remove(item: Item) {
    setBusy(true)
    try {
      const response = await apiFetch(`/api/smart-folders/${item.id}`, { method: 'DELETE', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ revision: item.revision }) })
      if (!response.ok) { setNotice(response.status === 409 ? '他の画面で変更されました。一覧を更新しました。' : '削除できませんでした。'); await load(); return }
      setNotice('共有検索を削除しました。'); await load()
    } catch { setNotice('削除できませんでした。') } finally { setBusy(false) }
  }
  return <details className="smart-folders" aria-label="共有検索"><summary>共有検索（{items.length}）</summary><button type="button" onClick={() => void load()}>一覧を更新</button><ul>{items.map(item => <li key={item.id}><button type="button" onClick={() => setNotice(onApply(item.snapshot) || `「${item.name}」を適用しました。`)}>{item.name}</button><button type="button" disabled={busy} aria-label={`${item.name}を共有検索から削除`} onClick={() => void remove(item)}>削除</button></li>)}</ul>{items.length === 0 && <p>共有検索はありません。</p>}{notice && <p role="status">{notice}</p>}</details>
}
