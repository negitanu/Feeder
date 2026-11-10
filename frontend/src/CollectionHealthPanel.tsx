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
  collection_state: 'paused' | 'waiting' | 'error' | 'catching_up' | 'uncollected' | 'idle'
  next_retry: string | null
  more_pending: boolean
  gap_possible: boolean
}
type HealthResponse = {
  global_enabled: boolean
  global_interval_seconds: number
  feeds: HealthFeed[]
  truncated: boolean
}
type Filter = 'all' | 'errors' | 'stale'

function errorGuidance(error: string): { summary: string; action: string } {
  const match = /; diagnostic=([a-z_0-9]+)$/.exec(error)
  const detail = match?.[1]
  const http = detail?.match(/^upstream_http_(\d{3})$/)
  if (http) {
    const code = Number(http[1])
    if (code === 401 || code === 403) return { summary: `配信元がアクセスを拒否しました（HTTP ${code}）`, action: '配信元の公開設定や購読URLを確認してください。' }
    if (code === 404 || code === 410) return { summary: `フィードが見つかりません（HTTP ${code}）`, action: '配信元の新しいRSS URLを確認し、購読管理で変更してください。' }
    if (code === 429 || code === 503) return { summary: `配信元が一時的に取得を制限しています（HTTP ${code}）`, action: '表示された再試行時刻まで待ってください。' }
    return { summary: `配信元がエラーを返しました（HTTP ${code}）`, action: '配信元の状態と購読URLを確認してください。' }
  }
  const guidance: Record<string, { summary: string; action: string }> = {
    dns: { summary: '配信元の名前を解決できませんでした', action: '購読URLとネットワークを確認してください。' },
    connection: { summary: '配信元へ接続できませんでした', action: 'ネットワークと配信元の状態を確認してください。' },
    timeout: { summary: '配信元の応答が時間内に完了しませんでした', action: '時間をおいて再試行してください。' },
    body: { summary: 'フィードの受信が途中で失敗しました', action: 'ネットワークと配信元の状態を確認してください。' },
    redirect: { summary: '転送先をたどれませんでした', action: '購読URLと配信元の転送設定を確認してください。' },
    unsafe_target: { summary: '安全でない接続先として拒否しました', action: '公開されたHTTP(S)のフィードURLを確認してください。' },
    too_large: { summary: 'フィードが取得上限を超えました', action: '配信元に小さいフィードや対象を絞ったURLがあるか確認してください。' },
    invalid_feed: { summary: 'フィードを解析できませんでした', action: 'RSSまたはAtomのURLか確認し、必要なら購読管理で変更してください。' },
  }
  return (detail && guidance[detail]) || { summary: '収集に失敗しました（詳細不明）', action: '次回の再試行後も続く場合は、配信元と購読URLを確認してください。' }
}

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
          <td><span className="collection-health-status">{feed.paused && <span>停止中</span>}{feed.error && <><span>{errorGuidance(feed.error).summary}</span><span>{errorGuidance(feed.error).action} <a href="#settings-subscriptions-list">購読管理へ</a></span></>}{feed.collection_state === 'uncollected' && <span>未収集</span>}{feed.collection_state === 'waiting' && <span>待機中（再試行 {absoluteTime(feed.next_retry, timezone)} 以降）</span>}{feed.more_pending && <span>続きを取得予定</span>}{feed.gap_possible && <span>取得漏れの可能性があります</span>}{feed.consecutive_failures > 0 && <span>連続失敗 {feed.consecutive_failures} 回</span>}{!feed.paused && !feed.error && feed.consecutive_failures === 0 && feed.collection_state !== 'waiting' && feed.collection_state !== 'uncollected' && !feed.more_pending && <span>{feed.stale ? '古い' : '正常'}</span>}</span></td>
          <td>{absoluteTime(feed.last_checked, timezone)}</td><td>{absoluteTime(feed.last_success, timezone)}</td><td>{absoluteTime(feed.next_due, timezone)}</td>
          <td>{feed.individual_override ? `${feed.poll_interval_seconds} 秒（個別）` : `${data.global_interval_seconds} 秒`}</td>
          <td><button type="button" disabled={feed.paused || feed.collection_state === 'waiting' || retrying !== null} onClick={() => void retry(feed.id)}>{retrying === feed.id ? '再試行中…' : '再試行'}</button></td>
        </tr>)}</tbody></table></div>}
    </>}
  </section>
}
