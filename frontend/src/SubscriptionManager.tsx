import { useRef, useState } from 'react'
import { apiFetch } from './apiFetch'
import './subscription-manager.css'
import type { Feed, Folder } from './LibrarySidebar'

type Props = { feeds: Feed[]; folders: Folder[]; onChanged: () => void; onEdit: (feed: Feed) => void }
const limit = 100

export default function SubscriptionManager({ feeds, folders, onChanged, onEdit }: Props) {
  const [query, setQuery] = useState('')
  const [selected, setSelected] = useState<number[]>([])
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  const lock = useRef(false)
  const folderName = (id: number | null) => id === null ? 'フォルダーなし' : folders.find(f => f.id === id)?.name ?? '不明なフォルダー'
  const visible = feeds.filter(f => [f.title, f.url, folderName(f.folder_id)].some(s => s.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase())))
  const activeSelected = selected.filter(id => feeds.some(f => f.id === id))

  function toggle(id: number, checked: boolean) {
    if (lock.current) return
    setSelected(old => {
      const current = old.filter(item => feeds.some(feed => feed.id === item))
      return checked ? current.length < limit ? [...current, id] : current : current.filter(item => item !== id)
    })
  }
  async function run(targets: Feed[], action: 'pause' | 'resume' | 'refresh') {
    if (lock.current || targets.length === 0 || targets.length > limit) return
    lock.current = true; setBusy(true); setMessage('処理しています…')
    const failures: string[] = []
    let success = 0
    try {
      for (const feed of targets) {
        try {
          const response = await apiFetch(`/api/feeds/${feed.id}${action === 'refresh' ? '/refresh' : ''}`, action === 'refresh'
            ? { method: 'POST' }
            : { method: 'PATCH', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ paused: action === 'pause' }) })
          if (!response.ok) throw new Error(`HTTP ${response.status}`)
          success++
        } catch (error) {
          failures.push(`${feed.title || feed.url}（${error instanceof TypeError ? '通信結果不明' : error instanceof Error ? error.message : 'エラー'}）`)
        }
      }
      setMessage(`${success}件成功、${failures.length}件失敗${failures.length ? `：${failures.join('、')}。通信エラーの場合は結果が反映された可能性があります。` : '。'}`)
    } finally {
      try { onChanged() } finally { lock.current = false; setBusy(false) }
    }
  }
  const selectedFeeds = feeds.filter(f => activeSelected.includes(f.id))
  return <section className="subscription-manager" aria-label="購読管理" style={{ minWidth: 0 }}>
    <h2>購読管理</h2>
    <label>購読を検索 <input type="search" value={query} onChange={e => setQuery(e.target.value)} placeholder="タイトル、URL、フォルダー" /></label>
    <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, alignItems: 'center', margin: '12px 0' }}>
      <span>選択: {activeSelected.length}/{limit}件</span>
      <button type="button" disabled={busy || !selectedFeeds.length} onClick={() => void run(selectedFeeds, 'pause')}>選択を一時停止</button>
      <button type="button" disabled={busy || !selectedFeeds.length} onClick={() => void run(selectedFeeds, 'resume')}>選択を再開</button>
      <button type="button" disabled={busy || !activeSelected.length} onClick={() => setSelected([])}>選択解除</button>
    </div>
    <p>検索で非表示になった購読も、選択を解除するまで一括操作の対象です。</p>
    <p role="status" aria-live="polite">{message}</p>
    {visible.length === 0 ? <p>該当する購読はありません。</p> : <ul style={{ listStyle: 'none', padding: 0, margin: 0, display: 'grid', gap: 12 }}>
      {visible.map(feed => <li key={feed.id} style={{ border: '1px solid currentColor', borderRadius: 6, padding: 12, minWidth: 0, overflowWrap: 'anywhere' }}>
        <div style={{ display: 'flex', gap: 8, alignItems: 'baseline' }}><input type="checkbox" aria-label={`${feed.title || feed.url}を選択`} checked={activeSelected.includes(feed.id)} disabled={busy || (!activeSelected.includes(feed.id) && activeSelected.length >= limit)} onChange={e => toggle(feed.id, e.target.checked)} /><strong>{feed.title || feed.url}</strong></div>
        <div>{feed.url}</div><div>{folderName(feed.folder_id)} · {feed.paused ? '停止中' : '稼働中'}</div>
        <div>最終確認: {feed.last_checked || '未確認'}</div><div>エラー: {feed.last_error || 'なし'}</div>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, marginTop: 8 }}>
          <button type="button" disabled={busy} onClick={() => { if (!lock.current) onEdit(feed) }}>編集</button>
          <button type="button" disabled={busy} onClick={() => void run([feed], feed.paused ? 'resume' : 'pause')}>{feed.paused ? '再開' : '一時停止'}</button>
          <button type="button" disabled={busy || feed.paused} onClick={() => void run([feed], 'refresh')}>今すぐ更新</button>
        </div>
      </li>)}
    </ul>}
  </section>
}
