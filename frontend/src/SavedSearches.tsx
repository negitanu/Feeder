import { useState } from 'react'
import { apiFetch } from './apiFetch'
import type { AppliedFilters, Filter } from './App'
export type SearchSnapshot = { filter: Filter; query: string; source: { kind: 'feed' | 'folder'; id: number } | null; applied: AppliedFilters }
type Saved = { name: string; snapshot: SearchSnapshot }
const key = 'feeder-saved-searches-v1'
function valid(value: unknown): value is Saved {
  if (!value || typeof value !== 'object') return false
  const { name, snapshot: s } = value as Saved
  if (typeof name !== 'string' || !name.trim() || name.length > 80 || !s || typeof s !== 'object' || typeof s.query !== 'string' || s.query.length > 1000 || !['all','unread','starred','recent','later'].includes(s.filter)) return false
  if (s.source !== null && (!s.source || !['feed','folder'].includes(s.source.kind) || !Number.isSafeInteger(s.source.id) || s.source.id < 1)) return false
  const a = s.applied
  if (!a || typeof a !== 'object' || !['new','old','recent'].includes(a.order) || !['both','title','body'].includes(a.scope)) return false
  const allowed = ['order','scope','any','exclude','published_from','published_to','feed_ids']
  return Object.entries(a).every(([k,v]) => v === undefined || (['later','include_muted'].includes(k) ? typeof v === 'boolean' : allowed.includes(k) && typeof v === 'string' && v.length <= 1000))
}
function load(): Saved[] {
  try { const parsed = JSON.parse(localStorage.getItem(key) || 'null'); return parsed?.version === 1 && Array.isArray(parsed.items) ? parsed.items.filter(valid).slice(0,50) : [] } catch { return [] }
}
export default function SavedSearches({ snapshot, onApply, onShared }: { snapshot: SearchSnapshot; onApply: (value: SearchSnapshot) => string | null; onShared?: () => void }) {
  const [items, setItems] = useState<Saved[]>(load)
  const [name, setName] = useState('')
  const [notice, setNotice] = useState('')
  function persist(next: Saved[]) {
    setItems(next)
    try { localStorage.setItem(key, JSON.stringify({ version: 1, items: next })); setNotice('保存した検索を更新しました。') }
    catch { setNotice('このブラウザに保存できませんでした。この画面を開いている間だけ利用できます。') }
  }
  async function copy(item: Saved) {
    try {
      const response = await apiFetch('/api/smart-folders', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(item) })
      if (response.ok) onShared?.()
      setNotice(response.ok ? `「${item.name}」を共有検索にコピーしました。` : response.status === 409 ? '同じ名前があるか、共有検索が上限に達しています。' : '共有検索にコピーできませんでした。')
    } catch { setNotice('共有検索にコピーできませんでした。') }
  }
  return <details className="saved-searches"><summary>保存した検索（{items.length}）</summary><p>このブラウザに、適用済みの検索条件を保存します。日付は現在の表示タイムゾーンで検索します。</p><form onSubmit={e => { e.preventDefault(); const entry = { name: name.trim(), snapshot }; if (!valid(entry)) { setNotice('検索条件または名前を確認してください。'); return } if (items.some(item => item.name === entry.name)) { setNotice('同じ名前があります。別の名前を指定してください。'); return } if (items.length >= 50) return; persist([...items,entry]); setName('') }}><label>検索の名前<input aria-label="検索の名前" value={name} maxLength={80} required onChange={e => setName(e.target.value)} /></label><button disabled={!name.trim() || items.length >= 50}>現在の検索を保存</button></form><ul>{items.map((item,i) => <li key={item.name}><button className="saved-search-apply" onClick={() => { setNotice(onApply(item.snapshot) || `「${item.name}」を適用しました。`) }}>{item.name}</button><button type="button" aria-label={`${item.name}を共有検索にコピー`} onClick={() => void copy(item)}>共有にコピー</button><button aria-label={`${item.name}を削除`} onClick={() => persist(items.filter((_,index) => index !== i))}>削除</button></li>)}</ul>{items.length >= 50 && <p>保存できる検索は50件までです。</p>}{notice && <p role="status">{notice}</p>}</details>
}
