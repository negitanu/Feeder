import { useState } from 'react'
import { formatArticlePublished } from './ArticleRowContent'
import { originalArticleUrl, useCatchupQuery } from './catchupApi'
import type { ArticleRevisions, ReaderTimezone } from './catchupApi'
import './catchup.css'

function History({ articleId, timezone }: { articleId: number; timezone: ReaderTimezone }) {
  const [before, setBefore] = useState<number | null>(null)
  const query = useCatchupQuery<ArticleRevisions>(`/api/articles/${articleId}/revisions?limit=10${before ? `&before_revision=${before}` : ''}`)
  return <div className="revision-history">
    <p className="catchup-note">Feeder が収集時に観測した変更です。配信元の編集履歴全体ではありません。最新20版まで保存します。</p>
    {query.loading && <p role="status">履歴を読み込み中…</p>}
    {query.error && <p role="alert" className="catchup-error">{query.error} <button type="button" onClick={query.reload}>再試行</button></p>}
    {!query.loading && !query.error && query.data && <>
      {query.data.items.length === 0 && <p>まだ履歴を記録していません。次回の収集から記録します。</p>}
      {query.data.items.map((item, index) => {
        const previous = query.data!.items[index + 1]
        const changed = previous ? [item.title !== previous.title && 'タイトル', item.content !== previous.content && '本文', item.url !== previous.url && 'URL'].filter(Boolean).join('・') : ''
        const url = originalArticleUrl(item.url)
        return <details key={item.revision} className="revision-entry" open={index === 0 ? true : undefined}>
          <summary>版{item.revision} · {item.baseline ? '最初の記録' : changed ? `${changed}を更新` : '内容を更新'}</summary>
          <p className="catchup-note">観測: <time dateTime={item.observed_at}>{formatArticlePublished(item.observed_at, timezone)}</time></p>
          {item.source_updated_at && <p className="catchup-note">配信元の更新日時: {formatArticlePublished(item.source_updated_at, timezone)}</p>}
          <h3>{item.title || '無題'}</h3>
          {url && <a className="revision-url" href={url} target="_blank" rel="noopener noreferrer">{url}</a>}
          <div className="revision-content">{item.content || '本文なし'}</div>
          {previous && changed && <details className="revision-previous"><summary>変更前（版{previous.revision}）と比べる</summary>
            {item.title !== previous.title && <p><strong>以前のタイトル</strong><br />{previous.title}</p>}
            {item.url !== previous.url && <p className="revision-url"><strong>以前のURL</strong><br />{previous.url}</p>}
            {item.content !== previous.content && <div className="revision-content"><strong>以前の本文</strong><br />{previous.content || '本文なし'}</div>}
          </details>}
        </details>
      })}
      {query.data.history_start !== null && <p className="catchup-note">保存している最初の記録: 版{query.data.history_start}。これより前の履歴は表示できません。</p>}
      <div className="catchup-actions">
        {before !== null && <button type="button" onClick={() => setBefore(null)}>最新の履歴</button>}
        {query.data.has_more && query.data.next_before_revision !== null && <button type="button" onClick={() => setBefore(query.data!.next_before_revision)}>以前の履歴</button>}
      </div>
    </>}
  </div>
}

export default function ArticleRevisionsPanel(props: { articleId: number; timezone: ReaderTimezone }) {
  const [open, setOpen] = useState(false)
  return <details className="detail-history catchup-surface" onToggle={event => setOpen(event.currentTarget.open)}>
    <summary>記事の更新履歴</summary>
    {open && <History {...props} />}
  </details>
}
