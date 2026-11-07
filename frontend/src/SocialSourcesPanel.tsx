import { useCallback, useEffect, useState } from 'react'
import { apiFetch } from './apiFetch'
import './SocialSourcesPanel.css'

type Source = { feed_id: number; provider: 'mastodon' | 'bluesky'; actor: string; server: string; title: string; paused: boolean }
type Props = { onChanged: () => void }
async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await apiFetch(path, init)
  if (!response.ok) throw new Error(response.status === 409 ? '登録済みか、更新中です。' : '操作できませんでした。入力内容と接続先を確認してください。')
  return response.status === 204 ? undefined as T : response.json() as Promise<T>
}
export default function SocialSourcesPanel({ onChanged }: Props) {
  const [sources, setSources] = useState<Source[]>([])
  const [provider, setProvider] = useState<'mastodon' | 'bluesky'>('mastodon')
  const [actor, setActor] = useState('')
  const [server, setServer] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [ready, setReady] = useState(false)
  const load = useCallback(async () => { try { const data = await request<{ sources: Source[] }>('/api/social/sources'); setSources(data.sources); setReady(true); setError('') } catch (e) { setError((e as Error).message) } }, [])
  useEffect(() => { void load() }, [load])
  async function add(e: React.FormEvent) {
    e.preventDefault(); setBusy(true); setError('')
    try { await request('/api/social/sources', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ provider, actor: actor.trim(), ...(provider === 'mastodon' ? { server: server.trim() } : {}) }) }); setActor(''); await load(); onChanged() } catch (e) { setError((e as Error).message) } finally { setBusy(false) }
  }
  async function remove(source: Source) {
    if (!window.confirm(`「${source.title}」の購読と保存された記事を削除しますか？`)) return
    setBusy(true); setError('')
    try { await request(`/api/social/sources/${source.feed_id}`, { method: 'DELETE' }); await load(); onChanged() } catch (e) { setError((e as Error).message) } finally { setBusy(false) }
  }
  return <section className="collection-page social-sources" aria-label="公開SNS購読"><section className="collection-section"><h2 id="settings-social" tabIndex={-1}>公開SNS購読</h2><p>公開アカウントの投稿を通常の購読として表示します。非公開投稿にはアクセスしません。</p>
    <form className="social-form" onSubmit={e => void add(e)}><label>サービス<select value={provider} onChange={e => setProvider(e.target.value as typeof provider)}><option value="mastodon">Mastodon</option><option value="bluesky">Bluesky</option></select></label>{provider === 'mastodon' && <label>サーバー URL<input type="url" required maxLength={2048} placeholder="https://mastodon.example" value={server} onChange={e => setServer(e.target.value)} /></label>}<label>アカウント<input required maxLength={253} placeholder={provider === 'mastodon' ? 'user' : 'user.bsky.social'} value={actor} onChange={e => setActor(e.target.value)} /></label><button disabled={busy || !ready}>購読を追加</button></form>
    <ul className="social-list">{sources.map(source => <li key={source.feed_id}><div><strong>{source.title}</strong><span>{source.provider === 'mastodon' ? 'Mastodon' : 'Bluesky'} · {source.actor}{source.paused ? ' · 停止中' : ''}</span></div><button type="button" disabled={busy} onClick={() => void remove(source)}>削除</button></li>)}</ul>{ready && sources.length === 0 && <p>公開SNSの購読はありません。</p>}{error && <div className="error-message" role="alert">{error}<button type="button" onClick={() => void load()}>再取得</button></div>}
  </section></section>
}
