import { useEffect, useId, useRef, useState } from 'react'
import { Check, ListChecks, RefreshCw, SkipForward, Undo2 } from 'lucide-react'
import { ArticleRowContent, formatArticlePublished } from './ArticleRowContent'
import type { Feed, Folder } from './LibrarySidebar'
import { catchupError, CatchupRequestError, catchupWrite, useCatchupQuery } from './catchupApi'
import type { AcknowledgementAction, CatchupInterests, CatchupItem, CatchupReason, CatchupResult, ReaderTimezone } from './catchupApi'
import './catchup.css'

export type CatchupPanelProps = {
  onOpen: (id: number, sequenceIds?: number[]) => void
  feeds: Feed[]
  folders: Folder[]
  timezone: ReaderTimezone
  refreshKey?: number
}

function reasonText(reason: CatchupReason, feeds: Feed[]): string {
  switch (reason.code) {
    case 'interest_term': return reason.value ? `関心語「${reason.value}」に一致` : '関心語に一致'
    case 'priority_feed': return `優先フィード${reason.value ? `: ${feeds.find(feed => feed.id === Number(reason.value))?.title ?? reason.value}` : ''}`
    case 'updated_since_visit': return '前回開いた後に内容更新'
    case 'recent_unread': return '期間内の未読記事'
    case 'source_diversity': return '同じ配信元に偏らないよう選定'
    default: return '対象期間内の候補'
  }
}

type Completed = { item: CatchupItem; action: AcknowledgementAction }

function InterestSettings({ feeds, onSaved }: { feeds: Feed[]; onSaved: () => void }) {
  const id = useId()
  const query = useCatchupQuery<CatchupInterests>('/api/catchup/interests')
  const [draft, setDraft] = useState<CatchupInterests | null>(null)
  const [terms, setTerms] = useState('')
  const [pending, setPending] = useState(false)
  const [error, setError] = useState('')
  const [conflict, setConflict] = useState(false)
  const [message, setMessage] = useState('')
  const busy = useRef(false)
  const alive = useRef(true)
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  useEffect(() => {
    if (query.data && !draft) { setDraft(query.data); setTerms(query.data.terms.join('\n')) }
  }, [query.data, draft])
  const parsed = terms.split(/\r?\n/).map(term => term.trim()).filter(Boolean)
  const invalid = parsed.length > 20 || parsed.some(term => Array.from(term).length > 80)

  async function save() {
    if (!draft || invalid || busy.current || conflict) return
    busy.current = true; setPending(true); setError(''); setMessage('')
    try {
      const saved = await catchupWrite<CatchupInterests>('/api/catchup/interests', 'PUT', { ...draft, terms: parsed })
      if (!alive.current) return
      if (saved) { setDraft(saved); setTerms(saved.terms.join('\n')) }
      query.reload(); onSaved(); setMessage('共有の関心設定を保存しました。')
    } catch (e) {
      if (!alive.current) return
      const stale = e instanceof CatchupRequestError && e.status === 409
      setConflict(stale)
      setError(stale ? '別の操作で共有設定が変更されました。入力は保持しています。最新設定を確認してから保存してください。' : catchupError(e))
    } finally { busy.current = false; if (alive.current) setPending(false) }
  }

  return <div className="catchup-interests">
    <p id={`${id}-help`} className="catchup-note">この Reader 全体で共有する設定です。閲覧履歴から関心を推測しません。タイトル・本文に文字通り一致する短い語句を、1行に1語句、20語句まで入力できます（各80文字まで）。</p>
    {query.loading && <p role="status">共有設定を読み込み中…</p>}
    {query.error && <p className="catchup-error" role="alert">{query.error} <button type="button" onClick={query.reload}>設定を再読み込み</button></p>}
    {draft && <form onSubmit={event => { event.preventDefault(); void save() }}>
      <label className="catchup-field" htmlFor={`${id}-terms`}>関心のある語句<textarea id={`${id}-terms`} rows={4} value={terms} disabled={pending} aria-describedby={`${id}-help ${id}-validation`} aria-invalid={invalid} onChange={event => { setTerms(event.target.value); setMessage('') }} /></label>
      <p id={`${id}-validation`} className={invalid ? 'catchup-error' : 'catchup-note'}>{parsed.length}/20語句{invalid && '。20語句以内、各80文字以内にしてください。'}</p>
      <fieldset className="catchup-feed-picker" disabled={pending}>
        <legend>優先するフィード（{draft.priority_feed_ids.length}/20件）</legend>
        <div>{feeds.map(feed => <label key={feed.id}>
          <input type="checkbox" checked={draft.priority_feed_ids.includes(feed.id)} disabled={!draft.priority_feed_ids.includes(feed.id) && draft.priority_feed_ids.length >= 20} onChange={event => {
            setDraft({ ...draft, priority_feed_ids: event.target.checked ? [...draft.priority_feed_ids, feed.id] : draft.priority_feed_ids.filter(value => value !== feed.id) }); setMessage('')
          }} /><span>{feed.title}{feed.paused && '（収集停止中）'}</span>
        </label>)}</div>
        {feeds.length === 0 && <p className="catchup-note">登録されたフィードはありません。</p>}
        {draft.priority_feed_ids.some(feedId => !feeds.some(feed => feed.id === feedId)) && <p className="catchup-note">現在一覧にないフィードが選択されています。最新設定を読み込んで確認してください。</p>}
      </fieldset>
      {error && <p className="catchup-error" role="alert">{error}</p>}
      {conflict && query.data && <div className="catchup-conflict">
        <button type="button" disabled={pending || query.loading} onClick={query.reload}>最新設定を読み込み</button>
        {query.data.revision !== draft.revision && <>
          <p className="catchup-note">最新の共有語句: {query.data.terms.join('、') || 'なし'}。優先フィード: {query.data.priority_feed_ids.map(feedId => feeds.find(feed => feed.id === feedId)?.title ?? `#${feedId}`).join('、') || 'なし'}。</p>
          <button type="button" disabled={query.loading} onClick={() => { setDraft({ ...draft, revision: query.data!.revision }); setConflict(false); setError('') }}>入力を保持して最新の版で保存し直す</button>
        </>}
      </div>}
      <button type="submit" disabled={pending || query.loading || invalid || conflict}>{pending ? '保存中…' : '共有設定を保存'}</button>
    </form>}
    {message && <p className="catchup-note" role="status">{message}</p>}
  </div>
}

export default function CatchupPanel({ onOpen, feeds, folders, timezone, refreshKey = 0 }: CatchupPanelProps) {
  const id = useId()
  const [hours, setHours] = useState<24 | 168>(24)
  const [limit, setLimit] = useState<10 | 20>(10)
  const [source, setSource] = useState('all')
  const params = new URLSearchParams({ hours: String(hours), limit: String(limit) })
  if (source.startsWith('feed:')) params.set('feed_id', source.slice(5))
  if (source.startsWith('folder:')) params.set('folder_id', source.slice(7))
  const query = useCatchupQuery<CatchupResult>(`/api/catchup?${params}`, refreshKey)
  const trends = useCatchupQuery<{ window_start: string; window_end: string; previous_start: string; scanned: number; truncated: boolean; collection_incomplete: boolean; topics: { term: string; current_count: number; previous_count: number; new_feed_ids: number[] }[] }>(`/api/catchup/topics?${params}`, refreshKey)
  const [completed, setCompleted] = useState<Completed[]>([])
  const [pending, setPending] = useState<string | null>(null)
  const [error, setError] = useState('')
  const [message, setMessage] = useState('')
  const busy = useRef(false)
  const alive = useRef(true)
  const undoButtons = useRef(new Map<string, HTMLButtonElement>())
  const refreshButton = useRef<HTMLButtonElement>(null)
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  const items = (query.data?.items ?? []).filter(item => !item.article.is_muted && !completed.some(done => done.item.event_key === item.event_key))

  async function acknowledge(item: CatchupItem, action: AcknowledgementAction, undo = false) {
    if (busy.current) return
    busy.current = true; setPending(item.event_key); setError(''); setMessage('')
    try {
      await catchupWrite('/api/catchup/acknowledgements', undo ? 'DELETE' : 'PUT', {
        article_id: item.article.id, revision: item.revision, fingerprint: item.fingerprint, ...(!undo ? { action } : {}),
      })
      if (!alive.current) return
      setCompleted(previous => undo ? previous.filter(done => done.item.event_key !== item.event_key) : [...previous.filter(done => done.item.event_key !== item.event_key), { item, action }].slice(-100))
      setMessage(undo ? 'この版の操作を取り消しました。最新の候補を読み込みます。' : `${item.article.title || '無題'}: ${action === 'confirmed' ? '現在版を確認しました' : 'この版を見送りました'}。既読状態は変更していません。`)
      if (undo) query.reload()
      requestAnimationFrame(() => { if (alive.current) (undo ? refreshButton.current : undoButtons.current.get(item.event_key))?.focus({ preventScroll: true }) })
    } catch (e) {
      if (alive.current) setError(e instanceof CatchupRequestError && e.status === 409 ? '記事の版が変わりました。操作は保存されていません。「再読み込み」で最新の版を確認してください。' : catchupError(e))
    } finally { busy.current = false; if (alive.current) setPending(null) }
  }

  return <section className="catchup-panel catchup-surface" aria-labelledby={`${id}-title`}>
    <header className="catchup-heading"><div><h2 id={`${id}-title`}><ListChecks size={20} aria-hidden="true" />キャッチアップ</h2>
      <p className="catchup-note">未読と読んだ後の更新から、いま読む記事を選びます。</p></div>
      <button ref={refreshButton} type="button" disabled={pending !== null} onClick={() => { query.reload(); trends.reload() }}><RefreshCw size={16} aria-hidden="true" />再読み込み</button>
    </header>
    <div className="catchup-controls">
      <label className="catchup-field">期間<select value={hours} onChange={event => setHours(Number(event.target.value) as 24 | 168)}><option value={24}>過去24時間</option><option value={168}>過去7日間</option></select></label>
      <label className="catchup-field">件数<select value={limit} onChange={event => setLimit(Number(event.target.value) as 10 | 20)}><option value={10}>10件まで</option><option value={20}>20件まで</option></select></label>
      <label className="catchup-field catchup-source">対象<select value={source} onChange={event => setSource(event.target.value)}><option value="all">すべての配信元</option>
        {folders.map(folder => <option key={`folder:${folder.id}`} value={`folder:${folder.id}`}>フォルダー: {folder.name}</option>)}
        {feeds.map(feed => <option key={`feed:${feed.id}`} value={`feed:${feed.id}`}>フィード: {feed.title}</option>)}
      </select></label>
    </div>
    <details className="catchup-settings"><summary>共有の関心設定</summary><InterestSettings feeds={feeds} onSaved={() => { query.reload(); trends.reload() }} /></details>
    <section className="catchup-coverage" aria-label="関心語の動向">
      <h3>関心語の動向</h3>
      <p className="catchup-note">上で選んだ期間・対象の保存記事を、直前の同じ長さの期間と比較します。タイトル・本文の文字列一致で、既読・確認済みも含み、ミュート記事は除きます。関連記事は統合しません。</p>
      {trends.loading && <p role="status">動向を読み込み中…</p>}
      {trends.error && <p className="catchup-error" role="alert">動向: {trends.error} <button type="button" onClick={trends.reload}>再読み込み</button></p>}
      {trends.data && <div aria-busy={trends.loading}>
        <p className="catchup-note">今回: {formatArticlePublished(trends.data.window_start, timezone)} 〜 {formatArticlePublished(trends.data.window_end, timezone)}。前回: {formatArticlePublished(trends.data.previous_start, timezone)} 〜 {formatArticlePublished(trends.data.window_start, timezone)}。評価 {trends.data.scanned}記事。</p>
        {trends.data.truncated && <p className="catchup-warning">評価上限に達したため、件数と新配信元の比較は不完全です。</p>}
        {trends.data.collection_incomplete && <p className="catchup-warning">収集停止・待機・取得失敗などにより、取得できていない記事がある可能性があります。</p>}
        {trends.data.topics.length === 0 ? <p>共有の関心設定に語句を追加すると動向を表示します。</p> : <ul>{trends.data.topics.map(topic => <li key={topic.term}>
          <strong>{topic.term}</strong>: {topic.current_count}件（前期間 {topic.previous_count}件、{topic.current_count - topic.previous_count > 0 ? '+' : ''}{topic.current_count - topic.previous_count}件）
          <br /><span>新しい配信元: {topic.new_feed_ids.length ? topic.new_feed_ids.map(feedId => feeds.find(feed => feed.id === feedId)?.title ?? `#${feedId}`).join('、') : 'なし'}</span>
        </li>)}</ul>}
      </div>}
    </section>
    <p className="catchup-note">確認・見送りはこの Reader で共有され、現在の記事の版に適用されます。既読やスターとは別に保存します。</p>
    {query.loading && <p role="status">候補を読み込み中…</p>}
    {query.error && <p className="catchup-error" role="alert">{query.error} 表示が残っている場合は前回の結果です。</p>}
    {error && <p className="catchup-error" role="alert">{error}</p>}
    {message && <p className="catchup-note" role="status">{message}</p>}
    {query.data && <div aria-busy={query.loading}>
      <div className="catchup-coverage">
        <p>今回読む記事は最大{query.data.limit}件、残り{items.length}件。</p>
        <details><summary>選定範囲について</summary><p className="catchup-note">対象: {formatArticlePublished(query.data.window_start, timezone)} 〜 {formatArticlePublished(query.data.window_end, timezone)}<br />選定時刻: {formatArticlePublished(query.data.generated_at, timezone)}</p>
        <p className="catchup-note">通常の記事一覧の検索条件は適用せず、この画面の期間と対象から選びます。同じ配信ホストは最大2件。独立した取材源の数や記事の正しさを示すものではありません。</p><p className="catchup-note">評価{query.data.scanned}件・まとめた候補{query.data.eligible_count}件。関連判定は最大100回。件数に満たない場合もあります。</p></details>
        {query.data.truncated && <p className="catchup-warning">候補の評価または関連判定の上限に達したため、一部の記事から選んでいます。</p>}
        {query.data.collection_incomplete && <p className="catchup-warning">収集停止・待機・取得失敗などにより、まだ取得できていない記事がある可能性があります。</p>}
      </div>
      {items.length === 0 ? <p className="catchup-empty">今回の候補はここまでです。再読み込みで次の候補を確認できます。</p> : <ol className="catchup-list">{items.map(item => <li key={item.event_key}>
        <button type="button" className="catchup-open" data-article-id={item.article.id} disabled={query.loading || !!query.error} onClick={() => onOpen(item.article.id, items.map(entry => entry.article.id))}>
          <ArticleRowContent article={item.article} timezone={timezone} />
        </button>
        <div className="catchup-item-meta"><span>{item.kind === 'updated' ? '内容更新' : '新着'} · 版{item.revision}</span><span>関連記事 {item.related_count}件</span></div>
        <ul className="catchup-reasons" aria-label="選定理由">{item.reasons.map((reason, index) => <li key={`${reason.code}:${index}`}>{reasonText(reason, feeds)}</li>)}</ul>
        <div className="catchup-actions">
          <button type="button" disabled={pending !== null || query.loading || !!query.error} onClick={() => void acknowledge(item, 'confirmed')}><Check size={16} aria-hidden="true" />現在版を確認</button>
          <button type="button" disabled={pending !== null || query.loading || !!query.error} onClick={() => void acknowledge(item, 'skipped')}><SkipForward size={16} aria-hidden="true" />この版を見送り</button>
          {pending === item.event_key && <span role="status">保存中…</span>}
        </div>
      </li>)}</ol>}
    </div>}
    {completed.length > 0 && <section className="catchup-completed" aria-label="今回の確認と見送り"><h3>確認・見送り済み</h3><ul>{completed.map(done => <li key={done.item.event_key}>
      <span>{done.item.article.title || '無題'} <small>版{done.item.revision} · {done.action === 'confirmed' ? '確認済み' : '見送り'}</small></span>
      <button type="button" ref={element => { if (element) undoButtons.current.set(done.item.event_key, element); else undoButtons.current.delete(done.item.event_key) }} disabled={pending !== null} onClick={() => void acknowledge(done.item, done.action, true)} aria-label={`「${done.item.article.title || '無題'}」の版${done.item.revision}の操作を取り消す`}><Undo2 size={16} aria-hidden="true" />取り消す</button>
    </li>)}</ul></section>}
  </section>
}
