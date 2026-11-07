import { useEffect, useState } from 'react'
import { apiFetch } from './apiFetch'
import './DuplicatePanel.css'

type Article = { id: number; title: string; url: string; feed_id: number | null; feed_title: string | null; is_read: boolean; is_starred: boolean }
type Result = { groups: { canonical_url: string; articles: Article[] }[]; group_count: number; article_count: number; scanned: number; truncated: boolean; offset: number; limit: number; has_more: boolean }
type Props = { onOpen: (id: number) => void; onRead: (ids: number[]) => Promise<void> }

/** The toggle retains the normal reader outside this panel. */
export default function DuplicatePanel({ onOpen, onRead }: Props) {
  const [active, setActive] = useState(false)
  const [offset, setOffset] = useState(0)
  const [revision, setRevision] = useState(0)
  const [data, setData] = useState<Result | null>(null)
  const [error, setError] = useState('')
  const [pending, setPending] = useState<number | null>(null)
  useEffect(() => {
    if (!active) return
    let cancelled = false
    setData(null); setError('')
    apiFetch(`/api/duplicates?offset=${offset}&limit=20`).then(async response => {
      if (!response.ok) throw new Error()
      const result = await response.json() as Result
      if (!cancelled) setData(result)
    }).catch(() => { if (!cancelled) setError('重複記事を取得できませんでした。') })
    return () => { cancelled = true }
  }, [active, offset, revision])
  async function markRead(id: number) {
    setPending(id); setError('')
    try { await onRead([id]); setRevision(value => value + 1) }
    catch { setError('既読にできませんでした。') }
    finally { setPending(null) }
  }
  return <section className="duplicate-panel" aria-label="重複記事">
    <button type="button" aria-expanded={active} onClick={() => { setActive(!active); setOffset(0) }}>{active ? '通常表示に戻る' : '重複記事を表示'}</button>
    {active && <div className="duplicate-content">
      <p>同じ URL の記事をまとめて表示します。元記事と既読・スターはそれぞれ保持されます。</p>
      {error && <p role="alert">{error}</p>}
      {!data && !error && <p role="status">読み込み中…</p>}
      {data && <>
        <p>{data.scanned} 件を調査: {data.group_count} グループ、{data.article_count} 件の元記事{data.truncated && '（最新 1,000 件のみ。古い記事は含まれません）'}</p>
        {data.group_count === 0 && <p>重複記事は見つかりませんでした。</p>}
        {data.groups.map(group => <section className="duplicate-group" key={group.canonical_url}>
          <h3>{group.articles[0].title} <span>（{group.articles.length} 件）</span></h3>
          <ul>{group.articles.map(article => <li key={article.id}>
            <div><button type="button" className="duplicate-open" onClick={() => onOpen(article.id)}>{article.title || '無題'}</button><p>{article.feed_title || '配信元なし'} · {article.is_read ? '既読' : '未読'}{article.is_starred ? ' · スター' : ''}</p><small>{article.url}</small></div>
            {!article.is_read && <button type="button" disabled={pending !== null} onClick={() => void markRead(article.id)}>この記事を既読</button>}
          </li>)}</ul>
        </section>)}
        <nav aria-label="重複グループのページ"><button type="button" disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - data.limit))}>前へ</button><button type="button" disabled={!data.has_more} onClick={() => setOffset(offset + data.limit)}>次へ</button></nav>
      </>}
    </div>}
  </section>
}
