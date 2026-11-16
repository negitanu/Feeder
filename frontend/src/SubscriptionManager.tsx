import { useEffect, useRef, useState } from 'react'
import { apiFetch } from './apiFetch'
import { absoluteTime, errorGuidance } from './CollectionHealthPanel'
import type { HealthFeed, HealthResponse } from './CollectionHealthPanel'
import './subscription-manager.css'
import type { Feed, Folder } from './LibrarySidebar'

type Props = { feeds: Feed[]; folders: Folder[]; onChanged: () => void; onEdit: (feed: Feed) => void; refreshKey?: number; timezone?: 'UTC' | 'Asia/Tokyo' }
type Status = 'all' | 'active' | 'paused' | 'error' | 'stale'
const limit = 100

export default function SubscriptionManager({ feeds, folders, onChanged, onEdit, refreshKey = 0, timezone = 'UTC' }: Props) {
  const [query, setQuery] = useState('')
  const [status, setStatus] = useState<Status>('all')
  const [selected, setSelected] = useState<number[]>([])
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  const [health, setHealth] = useState<HealthResponse | null>(null)
  const [healthError, setHealthError] = useState('')
  const [healthLoading, setHealthLoading] = useState(true)
  const [healthReload, setHealthReload] = useState(0)
  const lock = useRef(false)
  useEffect(() => {
    let active = true
    setHealthLoading(true); setHealthError(''); setHealth(null)
    apiFetch('/api/collection/health', { method: 'GET' }).then(async response => {
      if (!response.ok) throw new Error('収集状態を読み込めませんでした。')
      return response.json() as Promise<HealthResponse>
    }).then(value => { if (active) setHealth(value) }).catch(() => { if (active) setHealthError('収集状態を読み込めませんでした。購読の操作は引き続き利用できます。') }).finally(() => { if (active) setHealthLoading(false) })
    return () => { active = false }
  }, [refreshKey, healthReload])
  const healthById = new Map((health?.feeds ?? []).map(feed => [feed.id, feed]))
  const feedById = new Map(feeds.map(feed => [feed.id, feed]))
  const folderName = (id: number | null) => id === null ? 'フォルダーなし' : folders.find(f => f.id === id)?.name ?? '不明なフォルダー'
  const rows: { feed?: Feed; health?: HealthFeed; id: number }[] = [
    ...feeds.map(feed => ({ id: feed.id, feed, health: healthById.get(feed.id) })),
    ...(health?.feeds ?? []).filter(item => !feedById.has(item.id)).map(item => ({ id: item.id, health: item })),
  ]
  const visible = rows.filter(row => {
    const feed = row.feed, h = row.health
    const matches = status === 'all' || (status === 'active' && (feed ? !feed.paused : h?.paused === false)) || (status === 'paused' && (feed ? feed.paused : h?.paused === true)) || (status === 'error' && (!!h?.error || (h?.consecutive_failures ?? 0) > 0 || !!feed?.last_error)) || (status === 'stale' && !!h?.stale)
    const text = [feed?.title, feed?.url, feed ? folderName(feed.folder_id) : '', h?.name, h?.url].filter((x): x is string => !!x).join(' ').toLocaleLowerCase()
    return matches && text.includes(query.trim().toLocaleLowerCase())
  })
  const activeSelected = selected.filter(id => feedById.has(id))
  function toggle(id: number, checked: boolean) {
    if (lock.current) return
    setSelected(old => { const current = old.filter(item => feedById.has(item)); return checked ? current.length < limit ? [...current, id] : current : current.filter(item => item !== id) })
  }
  async function run(targets: Feed[], action: 'pause' | 'resume' | 'refresh') {
    if (lock.current || targets.length === 0 || targets.length > limit) return
    lock.current = true; setBusy(true); setMessage('処理しています…')
    const failures: string[] = []; let success = 0
    try {
      for (const feed of targets) {
        try {
          const response = await apiFetch(`/api/feeds/${feed.id}${action === 'refresh' ? '/refresh' : ''}`, action === 'refresh' ? { method: 'POST' } : { method: 'PATCH', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ paused: action === 'pause' }) })
          if (!response.ok) throw new Error(`HTTP ${response.status}`)
          success++
        } catch (error) { failures.push(`${feed.title || feed.url}（${error instanceof TypeError ? '通信結果不明' : error instanceof Error ? error.message : 'エラー'}）`) }
      }
      setMessage(`${success}件成功、${failures.length}件失敗${failures.length ? `：${failures.join('、')}。通信エラーの場合は結果が反映された可能性があります。` : '。'}`)
    } finally { try { onChanged() } finally { lock.current = false; setBusy(false) } }
  }
  const selectedFeeds = feeds.filter(f => activeSelected.includes(f.id))
  return <section className="subscription-manager" aria-labelledby="subscription-manager-title">
    <h2 id="subscription-manager-title">購読管理・収集状態</h2>
    {health && <p>自動収集 {health.global_enabled ? '有効' : '無効'} · 全体間隔 {health.global_interval_seconds} 秒</p>}
    {healthLoading && <p role="status">収集状態を読み込んでいます…</p>}
    {healthError && <p role="alert">{healthError} <button type="button" onClick={() => setHealthReload(v => v + 1)}>再読み込み</button></p>}
    {health?.truncated && <p role="status">収集状態は上限の 1000 件まで表示しています。残りの購読は状態不明として表示します。</p>}
    <search><label>購読を検索 <input type="search" value={query} onChange={e => setQuery(e.target.value)} placeholder="タイトル、URL、フォルダー" /></label>
      <fieldset><legend>購読の状態</legend>{([['all','すべて'], ['active','稼働中'], ['paused','停止中'], ['error','エラーあり'], ['stale','古い']] as const).map(([value,label]) => <label key={value}><input type="radio" name="subscription-status" checked={status === value} onChange={() => setStatus(value)} /> {label}</label>)}</fieldset>
    </search>
    {(status === 'error' || status === 'stale') && (!health || health.truncated) && <p>取得できていない収集状態は絞り込みに含まれません。</p>}
    <div className="subscription-actions"><span>選択: {activeSelected.length}/{limit}件</span><button type="button" disabled={busy || !selectedFeeds.length} onClick={() => void run(selectedFeeds, 'pause')}>選択を一時停止</button><button type="button" disabled={busy || !selectedFeeds.length} onClick={() => void run(selectedFeeds, 'resume')}>選択を再開</button><button type="button" disabled={busy || !activeSelected.length} onClick={() => setSelected([])}>選択解除</button></div>
    <p>検索や絞り込みで非表示になった購読も、選択を解除するまで一括操作の対象です。</p>
    <p role="status" aria-live="polite">{message}</p>
    {visible.length === 0 ? <p>該当する購読はありません。</p> : <ul className="subscription-list">{visible.map(({ feed, health: h, id }) => {
      const title = feed?.title || h?.name || feed?.url || h?.url || `フィード ${id}`
      const hasError = !!h?.error || (h?.consecutive_failures ?? 0) > 0 || !!feed?.last_error
      const waiting = h?.collection_state === 'waiting'
      const healthLabels = h ? [hasError && 'エラーあり', h.stale && '古い', waiting && '再試行待ち', h.collection_state === 'uncollected' && '未収集', h.collection_state === 'catching_up' && '収集中', h.more_pending && '続きを取得予定', h.gap_possible && '取得漏れの可能性'].filter(Boolean) : []
      const healthSummary = !h ? '収集状態不明' : healthLabels.length ? healthLabels.join(' · ') : h.paused ? '収集停止中' : '正常'
      return <li key={id} className="subscription-row">
        <div className="subscription-row-heading">{feed ? <label><input type="checkbox" aria-label={`${title}を選択`} checked={activeSelected.includes(id)} disabled={busy || (!activeSelected.includes(id) && activeSelected.length >= limit)} onChange={e => toggle(id, e.target.checked)} /><strong>{title}</strong></label> : <strong>{title}</strong>}</div>
        <div className="subscription-url">{feed?.url || h?.url}</div>
        <p>{feed ? folderName(feed.folder_id) : '購読一覧にないフィード'} · {(feed?.paused ?? h?.paused) ? '停止中' : '稼働中'} · {healthSummary}</p>
        <details><summary>収集状態の詳細</summary>{h ? <div className="subscription-health-detail">
          {(h.error || feed?.last_error) && <p>{errorGuidance((h.error || feed?.last_error)!).summary}。{errorGuidance((h.error || feed?.last_error)!).action}</p>}
          {h.consecutive_failures > 0 && <p>連続失敗 {h.consecutive_failures} 回</p>}{waiting && <p>再試行 {absoluteTime(h.next_retry, timezone)} 以降</p>}{h.more_pending && <p>続きを取得予定</p>}{h.gap_possible && <p>取得漏れの可能性があります</p>}
          <dl><div><dt>最終確認</dt><dd>{absoluteTime(h.last_checked, timezone)}</dd></div><div><dt>最終成功</dt><dd>{absoluteTime(h.last_success, timezone)}</dd></div><div><dt>次回予定</dt><dd>{absoluteTime(h.next_due, timezone)}</dd></div><div><dt>収集間隔</dt><dd>{h.individual_override ? `${h.poll_interval_seconds} 秒（個別）` : `${health?.global_interval_seconds} 秒`}</dd></div></dl>
        </div> : <p>このフィードの収集状態を取得できませんでした。</p>}</details>
        {feed ? <div className="subscription-actions"><button type="button" aria-label={`${title}を編集`} disabled={busy} onClick={() => { if (!lock.current) onEdit(feed) }}>編集</button><button type="button" aria-label={`${title}を${feed.paused ? '再開' : '一時停止'}`} disabled={busy} onClick={() => void run([feed], feed.paused ? 'resume' : 'pause')}>{feed.paused ? '再開' : '一時停止'}</button><button type="button" aria-label={`${title}を${hasError ? '再試行' : '今すぐ更新'}`} disabled={busy || feed.paused || waiting} onClick={() => void run([feed], 'refresh')}>{hasError ? '再試行' : '今すぐ更新'}</button></div> : <div><p>購読一覧を再読み込みすると編集と一括操作が利用できます。</p><button type="button" aria-label={`${title}を再試行`} disabled={busy || !!h?.paused || waiting} onClick={() => { if (h) void run([{ id: h.id, title, url: h.url, folder_id: null, paused: h.paused, poll_interval: h.poll_interval_seconds }], 'refresh') }}>再試行</button></div>}
      </li>
    })}</ul>}
  </section>
}
