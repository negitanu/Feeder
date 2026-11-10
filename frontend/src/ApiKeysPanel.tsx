import { useCallback, useEffect, useRef, useState } from 'react'
import { apiFetch } from './apiFetch'
type ApiKey = { id: number; name: string; prefix: string; scope: 'read' | 'write'; created_at: string; expires_at: string; revoked_at: string | null; last_used_at: string | null }
async function read<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await apiFetch(path, init)
  if (!response.ok) throw new Error(response.status === 403 ? '確認情報が更新されました。もう一度操作してください。' : '操作できませんでした。入力内容と接続を確認してください。')
  if (response.status === 204) return undefined as T
  return response.json() as Promise<T>
}
const json = (body: unknown): RequestInit => ({ headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) })
export default function ApiKeysPanel() {
  const [keys, setKeys] = useState<ApiKey[]>([])
  const [name, setName] = useState('')
  const [scope, setScope] = useState<'read' | 'write'>('read')
  const [days, setDays] = useState(90)
  const [limit, setLimit] = useState(120)
  const limitDirty = useRef(false)
  const [ready, setReady] = useState(false)
  const [busy, setBusy] = useState(false)
  const [issued, setIssued] = useState<string | null>(null)
  const [notice, setNotice] = useState('')
  const [error, setError] = useState('')
  const load = useCallback(async () => {
    setError('')
    try { const [list, settings] = await Promise.all([read<{ keys: ApiKey[] }>('/api/keys'), read<{ rate_limit: number }>('/api/keys/settings')]); setKeys(list.keys); setLimit(settings.rate_limit); setReady(true) }
    catch (e) { setError((e as Error).message) }
  }, [])
  useEffect(() => { void load() }, [load])
  useEffect(() => {
    const refresh = () => { if (limitDirty.current) { setNotice('API上限が読み込まれました。未保存の入力は維持しています。保存すると入力内容が優先されます。'); return }; void load() }
    window.addEventListener('feeder-configuration-imported', refresh); return () => window.removeEventListener('feeder-configuration-imported', refresh)
  }, [load])
  async function issue(e: React.FormEvent) {
    e.preventDefault(); setBusy(true); setIssued(null); setError(''); setNotice('')
    try { const result = await read<{ id: number; token: string }>('/api/keys', { method: 'POST', ...json({ name: name.trim(), scope, days }) }); setIssued(result.token); setName(''); await load() }
    catch (e) { setError((e as Error).message) } finally { setBusy(false) }
  }
  async function revoke(key: ApiKey) {
    if (!window.confirm(`「${key.name}」のAPIキーを失効しますか？このキーを使う接続は利用できなくなります。`)) return
    setBusy(true); setError(''); setIssued(null); setNotice('')
    try { await read(`/api/keys/${key.id}`, { method: 'DELETE' }); await load(); setNotice('キーを失効しました。') }
    catch (e) { setError((e as Error).message) } finally { setBusy(false) }
  }
  async function saveLimit(e: React.FormEvent) {
    e.preventDefault(); setBusy(true); setError(''); setNotice('')
    try { const result = await read<{ rate_limit: number }>('/api/keys/settings', { method: 'PATCH', ...json({ rate_limit: limit }) }); setLimit(result.rate_limit); limitDirty.current = false; setNotice('APIの上限を保存しました。') }
    catch (e) { setError((e as Error).message) } finally { setBusy(false) }
  }
  return <section className="collection-page api-keys-panel" aria-label="APIキー管理"><section className="collection-section"><h2 id="settings-keys" tabIndex={-1}>APIキー</h2><p>外部のクライアントから記事や購読を扱うためのキーです。全文は発行時だけ表示します。共有や公開をせず、安全な場所へ保存してください。</p>
    <form className="key-issue-form" onSubmit={e => void issue(e)}><label>キー名<input aria-label="APIキー名" value={name} maxLength={80} required onChange={e => setName(e.target.value)} /></label><label>権限<select aria-label="APIキーの権限" value={scope} onChange={e => setScope(e.target.value as 'read' | 'write')}><option value="read">読み取りのみ</option><option value="write">読み取り・書き込み</option></select></label><label>有効期間（日）<input aria-label="APIキーの有効期間（日）" type="number" min="1" max="365" required value={days} onChange={e => setDays(Number(e.target.value))} /></label><button disabled={busy || !ready || !name.trim() || days < 1 || days > 365}>APIキーを発行</button></form>{scope === 'write' && <p>書き込み権限では購読や記事の既読・スター状態を変更できます。</p>}
    {issued && <div className="issued-key" role="status"><strong>このキーは再表示できません。</strong><code aria-label="発行したAPIキー">{issued}</code><button type="button" onClick={() => setIssued(null)}>キーを隠す</button></div>}
    <ul className="api-key-list">{keys.map(key => <li key={key.id}><div><strong>{key.name}</strong><span>{key.prefix}… · {key.scope === 'read' ? '読み取り' : '読み書き'}</span><span>期限: {key.expires_at} UTC{key.revoked_at ? ' · 失効済み' : ''}</span></div>{!key.revoked_at && <button disabled={busy} onClick={() => void revoke(key)} aria-label={`${key.name}を失効`}>失効</button>}</li>)}</ul>{ready && keys.length === 0 && <p>発行済みのキーはありません。</p>}
    <form className="settings-form" onSubmit={e => void saveLimit(e)}><label>キーごとの1分間の上限<input aria-label="APIリクエスト上限" type="number" required min="1" max="1000" value={limit} onChange={e => { limitDirty.current = true; setLimit(Number(e.target.value)) }} /></label><button disabled={busy || !ready || limit < 1 || limit > 1000}>API上限を保存</button></form>
    {notice && <p role="status" className="success-message">{notice}</p>}{error && <div className="error-message" role="alert">{error}<button onClick={() => void load()}>API設定を再取得</button></div>}
  </section></section>
}
