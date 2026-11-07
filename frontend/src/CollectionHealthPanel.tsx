import { useEffect, useState } from 'react'
import { apiFetch } from './apiFetch'
import './collection-health.css'

type HealthFeed = {
  id: number
  name: string
  url: string
  paused: boolean
  last_checked: string | null
  last_success: string | null
  error: string | null
  next_due: string | null
  consecutive_failures: number
  stale: boolean
  individual_override: boolean
  poll_interval_seconds: number | null
}
type HealthResponse = {
  global_enabled: boolean
  global_interval_seconds: number
  feeds: HealthFeed[]
  truncated: boolean
}
type Filter = 'all' | 'errors' | 'stale'

function absoluteTime(value: string | null, timezone: 'UTC' | 'Asia/Tokyo') {
  if (!value) return '—'
  const date = new Date(/(?:Z|[+-]\d\d:\d\d)$/.test(value) ? value : `${value.replace(' ', 'T')}Z`)
  if (Number.isNaN(date.getTime())) return value
  return new Intl.DateTimeFormat('ja-JP', {
    timeZone: timezone, year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false, timeZoneName: 'short',
  }).format(date)
}

export default function CollectionHealthPanel({ onRetry, refreshKey = 0, timezone = 'UTC' }: { onRetry: (id: number) => Promise<void>; refreshKey?: number; timezone?: 'UTC' | 'Asia/Tokyo' }) {
  const [data, setData] = useState<HealthResponse | null>(null)
  const [filter, setFilter] = useState<Filter>('all')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [retrying, setRetrying] = useState<number | null>(null)

  useEffect(() => {
    let active = true
    setLoading(true); setError('')
    apiFetch('/api/collection/health', { method: 'GET', credentials: 'same-origin' })
      .then(async response => {
        if (!response.ok) throw new Error('収集状態を読み込めませんでした。')
        return response.json() as Promise<HealthResponse>
      })
      .then(value => { if (active) setData(value) })
      .catch(reason => { if (active) setError((reason as Error).message) })
      .finally(() => { if (active) setLoading(false) })
    return () => { active = false }
  }, [refreshKey])

  const visible = (data?.feeds ?? []).filter(feed => filter === 'all' || (filter === 'errors' ? Boolean(feed.error) || feed.consecutive_failures > 0 : feed.stale))
  async function retry(id: number) {
    setRetrying(id); setError('')
    try { await onRetry(id) } catch (reason) { setError((reason as Error).message || '再試行に失敗しました。') }
    finally { setRetrying(null) }
  }

  return <section className="collection-health" aria-labelledby="collection-health-title">
    <header className="collection-health-heading">
      <div><h2 id="collection-health-title">収集状態</h2><p>{data ? `自動収集 ${data.global_enabled ? '有効' : '無効'} · 全体間隔 ${data.global_interval_seconds} 秒` : 'フィードの最新の収集状況'}</p></div>
      <div className="collection-health-filters" role="group" aria-label="表示するフィード">
        {([['all', 'すべて'], ['errors', 'エラー'], ['stale', '古い']] as const).map(([value, label]) => <button type="button" key={value} aria-pressed={filter === value} onClick={() => setFilter(value)}>{label}</button>)}
      </div>
    </header>
    {error && <p className="collection-health-error" role="alert">{error}</p>}
    {loading && <p className="collection-health-state" role="status">読み込み中…</p>}
    {!loading && data && <>
      {data.truncated && <p className="collection-health-truncated" role="status">表示件数が上限の 1000 件に達しました。一部のフィードは表示されていません。</p>}
      {visible.length === 0 ? <p className="collection-health-state">該当するフィードはありません。</p> : <div className="collection-health-scroll"><table><thead><tr><th scope="col">フィード</th><th scope="col">状態</th><th scope="col">最終確認</th><th scope="col">最終成功</th><th scope="col">次回予定</th><th scope="col">間隔</th><th scope="col"><span className="sr-only">操作</span></th></tr></thead>
        <tbody>{visible.map(feed => <tr key={feed.id} className={feed.error || feed.consecutive_failures ? 'has-error' : feed.stale ? 'is-stale' : ''}>
          <th scope="row"><span className="collection-health-name">{feed.name}</span><a href={feed.url} title={feed.url}>{feed.url}</a></th>
          <td>{feed.paused ? '停止中' : <span className="collection-health-status">{feed.error && <span>{feed.error}</span>}{feed.consecutive_failures > 0 && <span>連続失敗 {feed.consecutive_failures} 回</span>}{!feed.error && feed.consecutive_failures === 0 && <span>{feed.stale ? '古い' : '正常'}</span>}</span>}</td>
          <td>{absoluteTime(feed.last_checked, timezone)}</td><td>{absoluteTime(feed.last_success, timezone)}</td><td>{absoluteTime(feed.next_due, timezone)}</td>
          <td>{feed.individual_override ? `${feed.poll_interval_seconds} 秒（個別）` : `${data.global_interval_seconds} 秒`}</td>
          <td><button type="button" disabled={feed.paused || retrying !== null} onClick={() => void retry(feed.id)}>{retrying === feed.id ? '再試行中…' : '再試行'}</button></td>
        </tr>)}</tbody></table></div>}
    </>}
  </section>
}
