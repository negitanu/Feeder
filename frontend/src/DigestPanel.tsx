import { useEffect, useState } from 'react'
import { apiFetch } from './apiFetch'
import { formatArticlePublished, type ArticleRowContentProps } from './ArticleRowContent'
import type { Feed, Folder } from './LibrarySidebar'
import './DigestPanel.css'

type Item = { id: number; title: string; published: string; url: string }
type Source = { feed_id: number; source_title: string; count: number; shown_count: number; items: Item[] }
type Digest = { hours: 24 | 168; total: number; shown_count: number; source_count: number; truncated: boolean; sources: Source[] }
type Props = { feeds: Feed[]; folders: Folder[]; onOpen: (id: number) => void; timezone?: ArticleRowContentProps['timezone'] }

export default function DigestPanel({ feeds, folders, onOpen, timezone = 'UTC' }: Props) {
  const [hours, setHours] = useState<24 | 168>(24)
  const [source, setSource] = useState('all')
  const [digest, setDigest] = useState<Digest | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [refresh, setRefresh] = useState(0)

  useEffect(() => {
    const controller = new AbortController()
    const params = new URLSearchParams({ hours: String(hours) })
    if (source.startsWith('feed:')) params.set('feed_id', source.slice(5))
    if (source.startsWith('folder:')) params.set('folder_id', source.slice(7))
    setLoading(true); setError('')
    apiFetch(`/api/digest?${params}`, { signal: controller.signal })
      .then(async response => {
        if (!response.ok) throw new Error('ダイジェストを読み込めませんでした。')
        return await response.json() as Digest
      })
      .then(value => { if (!controller.signal.aborted) setDigest(value) })
      .catch(reason => { if (!controller.signal.aborted) setError((reason as Error).message) })
      .finally(() => { if (!controller.signal.aborted) setLoading(false) })
    return () => controller.abort()
  }, [hours, source, refresh])

  return <section className="digest-panel" aria-labelledby="digest-title">
    <header className="digest-heading">
      <div><h2 id="digest-title">新着ダイジェスト</h2>
        <p>保存済み記事から期間内の新着をまとめています。</p></div>
      <div className="digest-controls">
        <button type="button" onClick={() => setRefresh(value => value + 1)}>更新</button>
        <label>期間<select value={hours} onChange={e => setHours(Number(e.target.value) as 24 | 168)}>
          <option value={24}>過去24時間</option><option value={168}>過去7日間</option>
        </select></label>
        <label>配信元<select value={source} onChange={e => setSource(e.target.value)}>
          <option value="all">すべて</option>
          {folders.map(folder => <option key={`folder-${folder.id}`} value={`folder:${folder.id}`}>フォルダー: {folder.name}</option>)}
          {feeds.map(feed => <option key={`feed-${feed.id}`} value={`feed:${feed.id}`}>フィード: {feed.title}</option>)}
        </select></label>
      </div>
    </header>
    {loading ? <p className="digest-status" role="status">読み込み中…</p> : error ? <p className="digest-error" role="alert">{error}</p> : digest?.total === 0 ? <p className="digest-status">この期間の新着記事はありません。</p> : <>
      <p className="digest-total">期間内の記事 <strong>{digest?.total ?? 0}</strong> 件（{digest?.source_count ?? 0} 配信元）、表示 {digest?.shown_count ?? 0} 件{digest?.truncated && '（新着順、全体100件・配信元ごと10件まで）'}</p>
      <div className="digest-sources">{digest?.sources.map(group => <section className="digest-source" key={group.feed_id}>
        <header><h3>{group.source_title}</h3><span>{group.count} 件中 {group.shown_count} 件を表示</span></header>
        <ul>{group.items.map(item => <li key={item.id}>
          <button type="button" onClick={() => onOpen(item.id)}>{item.title}</button>
          <time dateTime={item.published}>{formatArticlePublished(item.published, timezone)}</time>
        </li>)}</ul>
      </section>)}</div>
    </>}
  </section>
}
