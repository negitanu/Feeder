import { apiFetch } from './apiFetch'
import { useEffect, useRef, useState } from 'react'

type Folder = { id: number; name: string }
type Candidate = { url: string; title: string }
type Preview = { token: string; url: string; title: string; entries: { title: string; url: string }[]; expires_in: number }
type Settings = { enabled: boolean; interval_seconds: number }
type OpmlPreview = { items: { url: string; title: string; folder: string | null; status: string }[]; valid: number; new: number; existing: number; duplicates: number; invalid: number }

async function api<T>(path: string, body?: unknown, method = 'POST'): Promise<T> {
  const response = await apiFetch(path, { method, credentials: 'same-origin', ...(body === undefined ? {} : { headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) }) })
  if (!response.ok) {
    let message = response.status === 400 ? '入力内容を確認してください。取得先は公開 HTTP/HTTPS URL に限ります。' : response.status === 403 ? '確認情報が無効か期限切れです。もう一度プレビューしてください。' : response.status === 409 ? '登録済みか、別の更新処理が実行中です。' : '取得または保存に失敗しました。接続先と入力内容を確認し、再試行してください。'
    try { const data = await response.json(); if (data.error) message = data.error } catch { /* Use the generic message. */ }
    throw new Error(message)
  }
  return response.json() as Promise<T>
}

export default function CollectionPanel({ folders, onChanged, onCreateFolder }: { folders: Folder[]; onChanged: () => void; onCreateFolder: (name: string) => Promise<boolean> }) {
  const [url, setUrl] = useState('')
  const [candidates, setCandidates] = useState<Candidate[]>([])
  const [candidate, setCandidate] = useState('')
  const [preview, setPreview] = useState<Preview | null>(null)
  const [folder, setFolder] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [settings, setSettings] = useState<Settings>({ enabled: false, interval_seconds: 900 })
  const [settingsBusy, setSettingsBusy] = useState(false)
  const [settingsLoading, setSettingsLoading] = useState(true)
  const [opmlBusy, setOpmlBusy] = useState(false)
  const [importing, setImporting] = useState(false)
  const [opmlError, setOpmlError] = useState('')
  const opmlInput = useRef<HTMLInputElement>(null)
  const [xml, setXml] = useState('')
  const [opmlPreview, setOpmlPreview] = useState<OpmlPreview | null>(null)
  const [folderName, setFolderName] = useState('')
  const [importResult, setImportResult] = useState<{ added: number; existing: number; duplicates: number; invalid: number } | null>(null)
  const discoveryRun = useRef(0)
  const previewRun = useRef(0)
  const opmlRun = useRef(0)
  const settingsDirtyRef = useRef(false)

  useEffect(() => { api<Settings>('/api/collection/settings', undefined, 'GET').then(value => { if (!settingsDirtyRef.current) setSettings(value) }).catch(e => setError(e.message)).finally(() => setSettingsLoading(false)) }, [])
  const clearPlan = () => { previewRun.current++; setPreview(null) }
  async function discover(e: React.FormEvent) {
    e.preventDefault(); const run = ++discoveryRun.current; setBusy(true); setError(''); setCandidates([]); setCandidate(''); clearPlan()
    try { const result = await api<{ candidates: Candidate[] }>('/api/discover', { url }); if (run !== discoveryRun.current) return; setCandidates(result.candidates); if (!result.candidates.length) setError('フィード候補が見つかりませんでした。RSS / Atom の URL を入力してお試しください。') }
    catch (e) { if (run === discoveryRun.current) setError((e as Error).message) } finally { if (run === discoveryRun.current) setBusy(false) }
  }
  async function makePreview() {
    if (!candidate) return
    const run = ++previewRun.current; setBusy(true); setError(''); setPreview(null)
    try { const result = await api<Preview>('/api/preview', { url: candidate }); if (run === previewRun.current) setPreview(result) } catch (e) { if (run === previewRun.current) setError((e as Error).message) } finally { if (run === previewRun.current) setBusy(false) }
  }
  async function confirmFeed() {
    if (!preview) return
    setBusy(true); setError('')
    try { await api('/api/preview/confirm', { token: preview.token, folder_id: folder ? Number(folder) : null }); clearPlan(); setCandidates([]); setCandidate(''); setUrl(''); onChanged() }
    catch (e) { setError((e as Error).message) } finally { setBusy(false) }
  }
  async function saveSettings(e: React.FormEvent) {
    e.preventDefault(); setSettingsBusy(true); setError('')
    try { setSettings(await api<Settings>('/api/collection/settings', settings, 'PATCH')); settingsDirtyRef.current = false } catch (e) { setError((e as Error).message) } finally { setSettingsBusy(false) }
  }
  function cancelOpml() {
    if (importing) return
    ++opmlRun.current; setOpmlPreview(null); setXml(''); setOpmlBusy(false); setOpmlError(''); setImportResult(null)
    if (opmlInput.current) opmlInput.current.value = ''
  }
  async function readOpml(file?: File) {
    if (importing) return
    const run = ++opmlRun.current; setOpmlPreview(null); setXml(''); setImportResult(null); setOpmlError(''); setOpmlBusy(false)
    if (opmlInput.current) opmlInput.current.value = ''
    if (!file) return
    if (file.size > 1024 * 1024) { setOpmlError('OPML ファイルは 1 MiB 以下にしてください。'); return }
    setOpmlBusy(true)
    try { const contents = await file.text(); if (run !== opmlRun.current) return; setXml(contents); const result = await api<OpmlPreview>('/api/opml/preview', { xml: contents }); if (run === opmlRun.current) setOpmlPreview(result) }
    catch (e) { if (run === opmlRun.current) setOpmlError((e as Error).message) } finally { if (run === opmlRun.current) setOpmlBusy(false) }
  }
  async function importOpml() {
    if (opmlBusy || importing || !opmlPreview || !xml) return
    setImporting(true); setOpmlError('')
    try { setImportResult(await api('/api/opml/import', { xml })); setOpmlPreview(null); setXml(''); onChanged() }
    catch (e) { setOpmlError((e as Error).message) } finally { setImporting(false) }
  }
  async function createFolder(e: React.FormEvent) { e.preventDefault(); if (!folderName.trim()) return; try { if (await onCreateFolder(folderName.trim())) setFolderName('') } catch (e) { setError((e as Error).message) } }

  return <section className="collection-page">
    <h2 id="settings-subscriptions" tabIndex={-1}>フィードを集める</h2><p>サイトからフィードを探し、内容を確認してから登録できます。</p>
    <form className="collection-form" onSubmit={discover}>
      <label>サイトまたはフィード URL<input aria-label="サイトまたはフィード URL" type="url" required placeholder="https://example.com" value={url} onChange={e => { discoveryRun.current++; previewRun.current++; setBusy(false); setUrl(e.target.value); setCandidates([]); setCandidate(''); setPreview(null) }} /></label>
      <button type="submit" disabled={busy || !url.trim()}>{busy ? '探しています…' : 'フィードを探す'}</button>
    </form>
    {candidates.length > 0 && <div className="collection-card"><h2>見つかったフィード</h2><label>フィード候補<select aria-label="フィード候補" value={candidate} onChange={e => { previewRun.current++; setBusy(false); setCandidate(e.target.value); setPreview(null) }}><option value="">選択してください</option>{candidates.map(c => <option key={c.url} value={c.url}>{c.title || c.url} — {c.url}</option>)}</select></label><button disabled={busy || !candidate} onClick={() => void makePreview()}>{busy ? '確認中…' : '内容をプレビュー'}</button></div>}
    {preview && <div className="collection-card" aria-live="polite"><h2>登録前のプレビュー</h2><strong>{preview.title}</strong><small>{preview.url}</small><ul className="preview-entries">{preview.entries.slice(0, 5).map((entry, i) => <li key={`${entry.url}-${i}`}><span>{entry.title}</span><small>{entry.url}</small></li>)}</ul><label>フォルダー<select aria-label="登録先フォルダー" value={folder} onChange={e => setFolder(e.target.value)}><option value="">フォルダーなし</option>{folders.map(f => <option key={f.id} value={f.id}>{f.name}</option>)}</select></label><button disabled={busy} onClick={() => void confirmFeed()}>{busy ? '登録しています…' : 'このフィードを登録'}</button></div>}
    <form className="folder-form" onSubmit={e => void createFolder(e)}><label>フォルダーを作成<input aria-label="フォルダー名" value={folderName} onChange={e => setFolderName(e.target.value)} placeholder="例：デザイン" /></label><button disabled={busy || !folderName.trim()}>作成</button></form>
    <section className="collection-section"><h2 id="settings-collection" tabIndex={-1}>自動収集</h2><p>停止中の購読は収集しません。無効にすると、新しい自動取得を停止します。</p><form className="settings-form" onSubmit={e => void saveSettings(e)}><label className="check-label"><input type="checkbox" disabled={settingsLoading || settingsBusy} checked={settings.enabled} onChange={e => { settingsDirtyRef.current = true; setSettings({ ...settings, enabled: e.target.checked }) }} />自動収集を有効にする</label><label>更新間隔（秒）<input aria-label="更新間隔（秒）" disabled={settingsLoading || settingsBusy} type="number" min="60" max="86400" required value={settings.interval_seconds} onChange={e => { settingsDirtyRef.current = true; setSettings({ ...settings, interval_seconds: Number(e.target.value) }) }} /></label><button disabled={settingsLoading || settingsBusy || settings.interval_seconds < 60 || settings.interval_seconds > 86400}>{settingsLoading ? '読み込み中…' : settingsBusy ? '保存中…' : '設定を保存'}</button></form></section>
    <section className="collection-section"><h2 id="settings-imports" tabIndex={-1}>OPML の読み込み・書き出し</h2><p>OPML の読み込みは内容を確認してから実行します。ファイルは 1 MiB 以下にしてください。新しく読み込んだフィードは停止状態で追加されます。</p><label className="file-label">OPML ファイルを選択<input ref={opmlInput} disabled={importing} aria-label="OPML ファイル" type="file" accept=".opml,.xml,text/xml,application/xml" onChange={e => void readOpml(e.target.files?.[0])} /></label>{importResult && <p role="status">読み込み完了：追加 {importResult.added} 件・登録済み {importResult.existing} 件・重複 {importResult.duplicates} 件・無効 {importResult.invalid} 件</p>}{opmlPreview && <div className="collection-card"><h3>読み込み内容</h3><p>有効 {opmlPreview.valid} 件・新規 {opmlPreview.new} 件・登録済み {opmlPreview.existing} 件・重複 {opmlPreview.duplicates} 件・無効 {opmlPreview.invalid} 件</p><ul className="preview-entries">{opmlPreview.items.map((item, i) => <li key={`${item.url}-${i}`}><span>{item.title || item.url} <small>{{ new: '新規', existing: '登録済み', duplicate: '重複', invalid: '無効' }[item.status] || item.status}</small></span><small>{item.folder || 'フォルダーなし'} · {item.url}</small></li>)}</ul><button disabled={opmlBusy || importing || !opmlPreview.new} onClick={() => void importOpml()}>{importing ? '読み込み中…' : '確認して読み込む'}</button></div>}{opmlBusy && <p role="status">OPML の内容を確認しています…</p>}{(opmlBusy || opmlPreview) && <button disabled={importing} onClick={cancelOpml}>OPML の読み込みを取り消す</button>}{opmlError && <p className="error-message" role="alert">{opmlError}</p>}<a className="export-link" href="/api/opml/export" download>OPML を書き出す</a></section>
    {error && <div className="error-message" role="alert">{error}</div>}
  </section>
}
