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
  const [feedName, setFeedName] = useState('')
  const [feedNotice, setFeedNotice] = useState('')
  const [registering, setRegistering] = useState(false)
  const [checkVersion, setCheckVersion] = useState(0)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [settings, setSettings] = useState<Settings>({ enabled: false, interval_seconds: 900 })
  const [intervalDraft, setIntervalDraft] = useState('900')
  const intervalValue = /^[0-9]+$/.test(intervalDraft) ? Number(intervalDraft) : NaN
  const intervalValid = Number.isSafeInteger(intervalValue) && intervalValue >= 60 && intervalValue <= 86400
  const [settingsNotice, setSettingsNotice] = useState('')
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

  useEffect(() => { api<Settings>('/api/collection/settings', undefined, 'GET').then(value => { if (!settingsDirtyRef.current) { setSettings(value); setIntervalDraft(String(value.interval_seconds)) } }).catch(e => setError(e.message)).finally(() => setSettingsLoading(false)) }, [])
  useEffect(() => {
    const refresh = () => { if (settingsDirtyRef.current) { setSettingsNotice('設定が読み込まれました。未保存の入力は維持しています。保存すると入力内容が優先されます。'); return }; void api<Settings>('/api/collection/settings', undefined, 'GET').then(value => { if (!settingsDirtyRef.current) { setSettings(value); setIntervalDraft(String(value.interval_seconds)) } }).catch(e => setError(e.message)) }
    window.addEventListener('feeder-configuration-imported', refresh); return () => window.removeEventListener('feeder-configuration-imported', refresh)
  }, [])
  useEffect(() => {
    const run = ++discoveryRun.current
    ++previewRun.current; setCandidates([]); setCandidate(''); setPreview(null); setFeedName(''); setError('')
    let valid = false
    try { const parsed = new URL(url.trim()); valid = ['http:', 'https:'].includes(parsed.protocol) && !!parsed.hostname } catch { /* Wait for a complete URL. */ }
    if (!valid) { setBusy(false); return }
    setBusy(true)
    const timer = window.setTimeout(async () => {
      try {
        const result = await api<{ candidates: Candidate[] }>('/api/discover', { url: url.trim() })
        if (run !== discoveryRun.current) return
        setCandidates(result.candidates)
        if (!result.candidates.length) throw new Error('登録可能なフィードが見つかりませんでした。')
        let lastError: unknown
        for (const item of result.candidates) {
          try {
            const value = await api<Preview>('/api/preview', { url: item.url })
            if (run !== discoveryRun.current) return
            setCandidate(item.url); setPreview(value); setFeedName(value.title); return
          } catch (error) { lastError = error; if (run !== discoveryRun.current) return }
        }
        throw lastError ?? new Error('登録可能なフィードが見つかりませんでした。')
      } catch (error) { if (run === discoveryRun.current) setError((error as Error).message) }
      finally { if (run === discoveryRun.current) setBusy(false) }
    }, 600)
    return () => { window.clearTimeout(timer); ++discoveryRun.current; ++previewRun.current }
  }, [url, checkVersion])
  async function chooseCandidate(value: string) {
    const run = ++previewRun.current
    setCandidate(value); setPreview(null); setBusy(true); setError(''); setFeedNotice('')
    try { const result = await api<Preview>('/api/preview', { url: value }); if (run === previewRun.current) { setPreview(result); setFeedName(result.title) } }
    catch (error) { if (run === previewRun.current) setError((error as Error).message) }
    finally { if (run === previewRun.current) setBusy(false) }
  }
  async function confirmFeed(e: React.FormEvent) {
    e.preventDefault()
    if (!preview || busy) return
    setBusy(true); setRegistering(true); setError('')
    try { await api('/api/preview/confirm', { token: preview.token, folder_id: folder ? Number(folder) : null, title: feedName.trim() || null }); setPreview(null); setCandidates([]); setCandidate(''); setUrl(''); setFeedNotice('フィードを登録しました'); onChanged() }
    catch (e) { setPreview(null); setError((e as Error).message) } finally { setBusy(false); setRegistering(false) }
  }
  async function saveSettings(e: React.FormEvent) {
    e.preventDefault(); if (!intervalValid || settingsBusy || settingsLoading) return; setSettingsBusy(true); setError(''); setSettingsNotice('')
    try { const result = await api<Settings>('/api/collection/settings', { ...settings, interval_seconds: intervalValue }, 'PATCH'); setSettings(result); setIntervalDraft(String(result.interval_seconds)); settingsDirtyRef.current = false; setSettingsNotice('自動収集の設定を保存しました') } catch (e) { setError((e as Error).message) } finally { setSettingsBusy(false) }
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
    <h2 id="settings-subscriptions" tabIndex={-1}>フィードを集める</h2><p>サイトまたはフィードの URL を入力すると、自動で登録可能なフィードを確認します。</p>
    <form className="collection-form" onSubmit={e => void confirmFeed(e)}>
      <label>サイトまたはフィード URL<input aria-label="サイトまたはフィード URL" type="url" required disabled={registering} placeholder="https://example.com" value={url} onChange={e => { ++discoveryRun.current; ++previewRun.current; setPreview(null); setFeedNotice(''); setUrl(e.target.value) }} /></label>
      {busy && <span role="status">フィードを確認しています…</span>}
      {candidates.length > 1 && <label>フィード候補<select aria-label="フィード候補" value={candidate} disabled={busy} onChange={e => void chooseCandidate(e.target.value)}>{candidates.map(c => <option key={c.url} value={c.url}>{c.title || c.url}</option>)}</select></label>}
      {preview && <><span role="status">登録可能: {preview.url}</span><label>フィード名<input aria-label="フィード名" maxLength={128} value={feedName} onChange={e => setFeedName(e.target.value)} /></label><label>フォルダー<select aria-label="登録先フォルダー" value={folder} onChange={e => setFolder(e.target.value)}><option value="">フォルダーなし</option>{folders.map(f => <option key={f.id} value={f.id}>{f.name}</option>)}</select></label></>}
      <button type="submit" disabled={busy || !preview}>{busy ? '確認中…' : '登録'}</button>
    </form>
    {feedNotice && <p role="status">{feedNotice}</p>}
    {error && <div className="error-message" role="alert">{error} {url.trim() && !busy && !preview && <button type="button" onClick={() => setCheckVersion(value => value + 1)}>もう一度確認</button>}</div>}
    <form className="folder-form" onSubmit={e => void createFolder(e)}><label>フォルダーを作成<input aria-label="フォルダー名" value={folderName} onChange={e => setFolderName(e.target.value)} placeholder="例：デザイン" /></label><button disabled={busy || !folderName.trim()}>作成</button></form>
    <section className="collection-section"><h2 id="settings-collection" tabIndex={-1}>自動収集</h2><p>停止中の購読は収集しません。無効にすると、新しい自動取得を停止します。</p><form className="settings-form" onSubmit={e => void saveSettings(e)}><label className="check-label"><input type="checkbox" disabled={settingsLoading || settingsBusy} checked={settings.enabled} onChange={e => { settingsDirtyRef.current = true; setSettings({ ...settings, enabled: e.target.checked }) }} />自動収集を有効にする</label><label>更新間隔（秒）<input aria-label="更新間隔（秒）" disabled={settingsLoading || settingsBusy} type="text" inputMode="numeric" pattern="[0-9]+" maxLength={5} aria-describedby="collection-interval-help" aria-invalid={!intervalValid} required value={intervalDraft} onChange={e => { settingsDirtyRef.current = true; setIntervalDraft(e.target.value) }} /></label><button disabled={settingsLoading || settingsBusy || !intervalValid}>{settingsLoading ? '読み込み中…' : settingsBusy ? '保存中…' : '設定を保存'}</button></form><p id="collection-interval-help">60〜86400秒の整数を直接入力できます</p>{settingsNotice && <p role="status">{settingsNotice}</p>}</section>
    <section className="collection-section"><h2 id="settings-imports" tabIndex={-1}>OPML の読み込み・書き出し</h2><p>OPML の読み込みは内容を確認してから実行します。ファイルは 1 MiB 以下にしてください。新しく読み込んだフィードは自動収集の対象になります。全体の自動収集が無効な場合は、有効にすると収集を開始します。</p><label className="file-label">OPML ファイルを選択<input ref={opmlInput} disabled={importing} aria-label="OPML ファイル" type="file" accept=".opml,.xml,text/xml,application/xml" onChange={e => void readOpml(e.target.files?.[0])} /></label>{importResult && <p role="status">読み込み完了：追加 {importResult.added} 件・登録済み {importResult.existing} 件・重複 {importResult.duplicates} 件・無効 {importResult.invalid} 件</p>}{opmlPreview && <div className="collection-card"><h3>読み込み内容</h3><p>有効 {opmlPreview.valid} 件・新規 {opmlPreview.new} 件・登録済み {opmlPreview.existing} 件・重複 {opmlPreview.duplicates} 件・無効 {opmlPreview.invalid} 件</p><ul className="preview-entries">{opmlPreview.items.map((item, i) => <li key={`${item.url}-${i}`}><span>{item.title || item.url} <small>{{ new: '新規', existing: '登録済み', duplicate: '重複', invalid: '無効' }[item.status] || item.status}</small></span><small>{item.folder || 'フォルダーなし'} · {item.url}</small></li>)}</ul><button disabled={opmlBusy || importing || !opmlPreview.new} onClick={() => void importOpml()}>{importing ? '読み込み中…' : '確認して読み込む'}</button></div>}{opmlBusy && <p role="status">OPML の内容を確認しています…</p>}{(opmlBusy || opmlPreview) && <button disabled={importing} onClick={cancelOpml}>OPML の読み込みを取り消す</button>}{opmlError && <p className="error-message" role="alert">{opmlError}</p>}<a className="export-link" href="/api/opml/export" download>OPML を書き出す</a></section>
  </section>
}
