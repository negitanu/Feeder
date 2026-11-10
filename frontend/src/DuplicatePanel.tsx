import { useEffect, useRef, useState } from 'react'
import { apiFetch } from './apiFetch'
import './DuplicatePanel.css'

type Article = { id: number; title: string; url: string; feed_id: number | null; feed_title: string | null; is_read: boolean; is_starred: boolean }
type Result = { groups: { canonical_url: string; articles: Article[]; match_count: number; members_truncated: boolean }[]; group_count: number; article_count: number; scanned: number; truncated: boolean; offset: number; limit: number; has_more: boolean; next_before_id: number | null }
type Props = { onOpen: (id: number) => void; onRead: (ids: number[]) => Promise<void> }

/** The toggle retains the normal reader outside this panel. */
export default function DuplicatePanel({ onOpen, onRead }: Props) {
  const [active, setActive] = useState(false)
  const [offset, setOffset] = useState(0)
  const [windows, setWindows] = useState<(number | null)[]>([null])
  const beforeId = windows[windows.length - 1]
  const [revision, setRevision] = useState(0)
  const [data, setData] = useState<Result | null>(null)
  const [error, setError] = useState('')
  const [pending, setPending] = useState<number | null>(null)
  const [loadingGroup, setLoadingGroup] = useState<string | null>(null)
  const generation = useRef(0)
  useEffect(() => {
    const current = ++generation.current
    setLoadingGroup(null)
    if (!active) return
    let cancelled = false
    setData(null); setError('')
    apiFetch(`/api/duplicates?offset=${offset}&limit=20${beforeId === null ? '' : `&before_id=${beforeId}`}`).then(async response => {
      if (!response.ok) throw new Error()
      const result = await response.json() as Result
      if (!cancelled) setData(result)
    }).catch(() => { if (!cancelled) setError('重複記事を取得できませんでした。') })
    return () => { cancelled = true; if (generation.current === current) generation.current++ }
  }, [active, offset, revision, beforeId])
  async function markRead(id: number) {
    setPending(id); setError('')
    try { await onRead([id]); setRevision(value => value + 1) }
    catch { setError('既読にできませんでした。') }
    finally { setPending(null) }
  }
  async function loadMembers(group: Result['groups'][number]) {
    if (loadingGroup || !group.articles.length) return
    const current = generation.current
    const anchor = group.articles[0].id
    const before = group.articles[group.articles.length - 1].id
    setLoadingGroup(group.canonical_url); setError('')
    try {
      const response = await apiFetch(`/api/duplicates/members?article_id=${anchor}&before_id=${before}`)
      if (!response.ok) throw new Error()
      const page = await response.json() as { articles: Article[]; next_before_id: number | null }
      if (generation.current !== current) return
      setData(previous => previous && { ...previous, groups: previous.groups.map(item => item.canonical_url !== group.canonical_url || item.articles[item.articles.length - 1]?.id !== before ? item : { ...item, articles: [...item.articles, ...page.articles], members_truncated: page.next_before_id !== null }) })
    } catch { if (generation.current === current) setError('続きの重複記事を取得できませんでした。もう一度お試しください。') }
    finally { if (generation.current === current) setLoadingGroup(null) }
  }
  return <section className="duplicate-panel" aria-label="重複記事">
    <button type="button" aria-expanded={active} onClick={() => { setActive(!active); setOffset(0); setWindows([null]) }}>{active ? '通常表示に戻る' : '重複記事を表示'}</button>
    {active && <div className="duplicate-content">
      <p>範囲内の記事と同じURLの記事をライブラリ全体から探します。同じグループが複数の範囲に現れることがあります。元記事と既読・スターはそれぞれ保持されます。</p>
      {error && <p role="alert">{error}</p>}
      {!data && !error && <p role="status">読み込み中…</p>}
      {data && <>
        <p>範囲 {windows.length}: {data.scanned} 件を調査、{data.group_count} グループ、{data.article_count} 件の元記事{data.truncated && '（さらに古い記事があります）'}</p>
        {data.group_count === 0 && <p>重複記事は見つかりませんでした。</p>}
        {data.groups.map(group => <section className="duplicate-group" key={group.canonical_url}>
          <h3>{group.articles[0].title} <span>（{group.match_count} 件{group.members_truncated ? `、${group.articles.length}件を表示` : ''}）</span></h3>
          <ul>{group.articles.map(article => <li key={article.id}>
            <div><button type="button" className="duplicate-open" onClick={() => onOpen(article.id)}>{article.title || '無題'}</button><p>{article.feed_title || '配信元なし'} · {article.is_read ? '既読' : '未読'}{article.is_starred ? ' · スター' : ''}</p><small>{article.url}</small></div>
            {!article.is_read && <button type="button" disabled={pending !== null} onClick={() => void markRead(article.id)}>この記事を既読</button>}
          </li>)}</ul>
          {group.members_truncated && <button type="button" disabled={loadingGroup !== null} onClick={() => void loadMembers(group)}>{loadingGroup === group.canonical_url ? '読み込み中…' : '続きを表示'}</button>}
        </section>)}
        <nav aria-label="重複グループのページ"><button type="button" disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - data.limit))}>前へ</button><button type="button" disabled={!data.has_more} onClick={() => setOffset(offset + data.limit)}>次へ</button></nav>
        <nav aria-label="調査する記事の範囲"><button type="button" disabled={windows.length === 1} onClick={() => { setWindows(previous => previous.slice(0, -1)); setOffset(0) }}>新しい範囲へ</button><button type="button" disabled={data.next_before_id === null} onClick={() => { if (data.next_before_id !== null) { setWindows(previous => [...previous, data.next_before_id]); setOffset(0) } }}>古い範囲へ</button></nav>
      </>}
    </div>}
  </section>
}
