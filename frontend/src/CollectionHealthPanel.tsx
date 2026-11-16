export type HealthFeed = {
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
export type HealthResponse = {
  global_enabled: boolean
  global_interval_seconds: number
  feeds: HealthFeed[]
  truncated: boolean
}

export function errorGuidance(error: string): { summary: string; action: string } {
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

export function absoluteTime(value: string | null, timezone: 'UTC' | 'Asia/Tokyo') {
  if (!value) return '—'
  const date = new Date(/(?:Z|[+-]\d\d:\d\d)$/.test(value) ? value : `${value.replace(' ', 'T')}Z`)
  if (Number.isNaN(date.getTime())) return value
  return new Intl.DateTimeFormat('ja-JP', {
    timeZone: timezone, year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false, timeZoneName: 'short',
  }).format(date)
}
