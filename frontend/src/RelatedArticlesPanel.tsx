import { useEffect, useRef, useState } from 'react'
import { catchupError, catchupWrite, useCatchupQuery } from './catchupApi'
import type { RelatedArticles } from './catchupApi'
import './catchup.css'

function Related({ articleId, onOpen }: { articleId: number; onOpen: (id: number) => void }) {
  const query = useCatchupQuery<RelatedArticles>(`/api/articles/${articleId}/related?limit=20`)
  const [pending, setPending] = useState(false)
  const [error, setError] = useState('')
  const alive = useRef(true)
  const busy = useRef(false)
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  async function exclude() {
    if (!query.data || busy.current) return
    busy.current = true; setPending(true); setError('')
    try {
      await catchupWrite(`/api/articles/${articleId}/related-exclusion`, 'PUT', { excluded: !query.data.excluded })
      if (alive.current) query.reload()
    } catch (e) { if (alive.current) setError(catchupError(e)) }
    finally { busy.current = false; if (alive.current) setPending(false) }
  }
  return <div className="related-articles">
    <p className="catchup-note">URL、本文、タイトル、発行日時の一致からまとめています。元の記事と既読状態はそれぞれ保持します。</p>
    {query.loading && <p role="status">関連記事を読み込み中…</p>}
    {query.error && <p role="alert" className="catchup-error">{query.error} <button type="button" onClick={query.reload}>再試行</button></p>}
    {error && <p role="alert" className="catchup-error">{error}</p>}
    {!query.loading && !query.error && query.data && <>
      <p>{query.data.excluded ? 'この記事は、まとめずに個別で表示します。' : `関連記事 ${query.data.total}件${query.data.truncated ? '以上（判定・表示した範囲）' : ''}`}</p>
      <ul className="related-list">{query.data.articles.map(({ article, reason }) => <li key={article.id}>
        <button type="button" onClick={() => onOpen(article.id)}>{article.title || '無題'}</button>
        <p className="catchup-note">{article.feed_title || '記事'} · {reason === 'canonical_url' ? '同じ記事URL' : '本文・タイトル・発行時期が一致'}</p>
      </li>)}</ul>
      <button type="button" disabled={pending} onClick={() => void exclude()}>{pending ? '保存中…' : query.data.excluded ? 'この記事を再びまとめる' : 'この記事をまとめずに表示'}</button>
      {query.data.truncated && <p className="catchup-note">判定または表示の上限に達したため、一部の関連記事を表示しています。</p>}
    </>}
  </div>
}

export default function RelatedArticlesPanel(props: { articleId: number; onOpen: (id: number) => void }) {
  const [open, setOpen] = useState(false)
  return <details className="detail-related catchup-surface" onToggle={event => setOpen(event.currentTarget.open)}>
    <summary>関連記事</summary>
    {open && <Related {...props} />}
  </details>
}
