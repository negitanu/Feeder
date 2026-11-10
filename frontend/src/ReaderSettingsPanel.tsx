import { apiFetch } from './apiFetch'
import { useEffect, useRef, useState } from 'react'
export type ReaderSettings = { display_timezone: 'UTC' | 'Asia/Tokyo'; default_view: 'all' | 'unread' | 'starred' }
type ImportState = { reader: ReaderSettings; collection: { enabled: boolean; interval_seconds: number }; rate_limit: number; folders: string[] }
type ImportPreview = { expected: ImportState; desired: ImportState; omitted: number; presentation: Pick<Presentation, 'appearance' | 'layout' | 'width'> | null }
export type Layout = 'standard' | 'line' | 'tiles'
export type Presentation = { appearance: 'system' | 'light' | 'dark'; width: number; layout: Layout; collapsed_folders: number[] }
export function storedLayout(): Layout { try { const value = localStorage.getItem('feeder-layout'); return value === 'line' || value === 'tiles' ? value : 'standard' } catch { return 'standard' } }
export default function ReaderSettingsPanel({ settings, presentation, onSaved, onLayout, onTheme, onRetry }: { settings: ReaderSettings | null; presentation: Presentation; onSaved: (value: ReaderSettings) => void; onLayout: (value: Layout) => void; onTheme: (value: Presentation['appearance']) => void; onRetry: () => void }) {
  const [draft, setDraft] = useState<ReaderSettings>(settings ?? { display_timezone: 'UTC', default_view: 'all' })
  const [saving, setSaving] = useState(false)
  const [exporting, setExporting] = useState(false)
  const [preview, setPreview] = useState<ImportPreview | null>(null)
  const [importing, setImporting] = useState(false)
  const [importPresentation, setImportPresentation] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const importSequence = useRef(0)
  const dirty = useRef(false)
  const mounted = useRef(true)
  useEffect(() => { mounted.current = true; return () => { mounted.current = false } }, [])
  useEffect(() => { if (settings && !dirty.current) setDraft(settings) }, [settings])
  async function save(e: React.FormEvent) {
    e.preventDefault(); setSaving(true); setError(''); setNotice('')
    try {
      const response = await apiFetch('/api/reader/settings', { method: 'PATCH', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(draft) })
      if (!response.ok) throw new Error('表示設定を保存できませんでした。入力内容と接続を確認してください。')
      const result: ReaderSettings = await response.json()
      onSaved(result)
      if (mounted.current) { dirty.current = false; setDraft(result); setNotice('表示設定を保存しました。初期表示は次回起動から適用されます。') }
    } catch (e) { if (mounted.current) setError((e as Error).message) } finally { if (mounted.current) setSaving(false) }
  }
  async function readImport(file: File | undefined) {
    const sequence = ++importSequence.current
    setPreview(null); setImportPresentation(false); setError(''); setNotice('')
    if (!file) return
    try {
      if (file.size > 1024 * 1024) throw new Error('ファイルは1 MiB以下にしてください。')
      const text = new TextDecoder('utf-8', { fatal: true }).decode(await file.arrayBuffer())
      const value: unknown = JSON.parse(text)
      if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('設定ファイルの形式が正しくありません。')
      const root = value as Record<string, unknown>
      if (root.format !== 'feeder-settings' || root.format_version !== 1) throw new Error('対応していない設定ファイル形式またはバージョンです。')
      const sections = root.settings
      if (!sections || typeof sections !== 'object' || Array.isArray(sections)) throw new Error('設定ファイルの形式が正しくありません。')
      const reader = (sections as Record<string, unknown>).reader
      if (!reader || typeof reader !== 'object' || Array.isArray(reader)) throw new Error('表示設定が見つかりません。')
      const r = reader as Record<string, unknown>
      if (typeof r.display_timezone !== 'string' || !['UTC', 'Asia/Tokyo'].includes(r.display_timezone) || typeof r.default_view !== 'string' || !['all', 'unread', 'starred'].includes(r.default_view)) throw new Error('表示設定の値が正しくありません。')
      const values = sections as Record<string, unknown>
      const collection = values.collection
      const api = values.api
      if (!collection || typeof collection !== 'object' || Array.isArray(collection) || !api || typeof api !== 'object' || Array.isArray(api) || !Array.isArray(values.folders)) throw new Error('収集設定、API上限またはフォルダーの形式が正しくありません。')
      const c = collection as Record<string, unknown>, a = api as Record<string, unknown>
      const rawPresentation = values.presentation
      let importedPresentation: ImportPreview['presentation'] = null
      if (rawPresentation !== undefined && rawPresentation !== null) {
        if (typeof rawPresentation !== 'object' || Array.isArray(rawPresentation)) throw new Error('ブラウザー表示設定の形式が正しくありません。')
        const p = rawPresentation as Record<string, unknown>
        if (typeof p.appearance !== 'string' || !['system', 'light', 'dark'].includes(p.appearance) || typeof p.layout !== 'string' || !['standard', 'line', 'tiles'].includes(p.layout) || !Number.isInteger(p.width) || (p.width as number) < 200 || (p.width as number) > 480) throw new Error('ブラウザー表示設定の値が正しくありません。')
        importedPresentation = { appearance: p.appearance as Presentation['appearance'], layout: p.layout as Layout, width: p.width as number }
      }
      if (typeof c.enabled !== 'boolean' || !Number.isSafeInteger(c.interval_seconds) || (c.interval_seconds as number) < 60 || (c.interval_seconds as number) > 86400 || !Number.isSafeInteger(a.rate_limit) || (a.rate_limit as number) < 1 || (a.rate_limit as number) > 1000 || values.folders.length > 1000) throw new Error('収集設定、API上限またはフォルダー件数が正しくありません。')
      const folders: string[] = []
      let omitted = 0
      for (const item of values.folders) {
        if (!item || typeof item !== 'object' || Array.isArray(item)) throw new Error('フォルダーの形式が正しくありません。')
        const name = (item as Record<string, unknown>).name
        if (name === null) { omitted++; continue }
        if (typeof name !== 'string' || name !== name.trim() || !name || new TextEncoder().encode(name).length > 512 || folders.includes(name)) throw new Error('フォルダー名が無効または重複しています。')
        folders.push(name)
      }
      const response = await apiFetch('/api/configuration/import')
      if (!response.ok) throw new Error('現在の設定を取得できませんでした。')
      const expected: ImportState = await response.json()
      if (sequence !== importSequence.current || !mounted.current) return
      setPreview({ expected, omitted, presentation: importedPresentation, desired: { reader: { display_timezone: r.display_timezone as ReaderSettings['display_timezone'], default_view: r.default_view as ReaderSettings['default_view'] }, collection: { enabled: c.enabled, interval_seconds: c.interval_seconds as number }, rate_limit: a.rate_limit as number, folders } })
    } catch (e) { if (sequence !== importSequence.current || !mounted.current) return; setError(e instanceof SyntaxError || e instanceof TypeError ? 'UTF-8の有効なJSONファイルを選択してください。' : (e as Error).message) }
  }
  async function applyImport() {
    if (!preview) return
    setImporting(true); setError(''); setNotice('')
    try {
      const response = await apiFetch('/api/configuration/import', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ expected: preview.expected, desired: preview.desired }) })
      if (response.status === 409) { setPreview(null); throw new Error('プレビュー後に設定またはフォルダーが変更されました。ファイルをもう一度選択してください。') }
      if (!response.ok) throw new Error('設定を読み込めませんでした。フォルダー名や接続を確認してください。')
      const result: ImportState = await response.json()
      onSaved(result.reader)
      if (importPresentation && preview.presentation) {
        onTheme(preview.presentation.appearance)
        onLayout(preview.presentation.layout)
        window.dispatchEvent(new CustomEvent('feeder-import-sidebar-width', { detail: preview.presentation.width }))
      }
      window.dispatchEvent(new Event('feeder-configuration-imported'))
      if (mounted.current) { dirty.current = false; setDraft(result.reader); setPreview(null); setNotice('設定を読み込みました。初期表示は次回起動から適用されます。') }
    } catch (e) { if (mounted.current) setError((e as Error).message) } finally { if (mounted.current) setImporting(false) }
  }
  async function download() {
    setExporting(true); setError(''); setNotice('')
    try {
      const response = await apiFetch('/api/configuration/export', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ presentation }) })
      if (!response.ok) throw new Error('設定を書き出せませんでした。接続を確認して再度お試しください。')
      const content = await response.blob()
      const result = JSON.parse(await content.text()) as { omissions: unknown[] }
      const url = URL.createObjectURL(content), anchor = document.createElement('a')
      anchor.href = url; anchor.download = 'feeder-settings.json'; document.body.appendChild(anchor); anchor.click(); anchor.remove()
      setTimeout(() => URL.revokeObjectURL(url), 1000)
      if (mounted.current) setNotice(`設定を書き出しました。${result.omissions.length ? `秘密情報を含む可能性がある ${result.omissions.length} 項目を省略しました。` : ''}`)
    } catch (e) { if (mounted.current) setError((e as Error).message) } finally { if (mounted.current) setExporting(false) }
  }
  return <section className="reader-settings collection-page" aria-label="表示設定とエクスポート">
    <section className="collection-section"><h2 id="settings-appearance" tabIndex={-1}>表示設定</h2>{!settings && <p role="status">表示設定を取得しています。<button type="button" onClick={onRetry}>表示設定を再取得</button></p>}<p>タイムゾーンと初期表示はこのライブラリを使うブラウザで共有します。日付検索もこのタイムゾーンで判定します。</p>
      <form className="settings-form" onSubmit={e => void save(e)}><label>表示タイムゾーン<select aria-label="表示タイムゾーン" disabled={!settings || saving || importing} value={draft.display_timezone} onChange={e => { dirty.current = true; setDraft({ ...draft, display_timezone: e.target.value as ReaderSettings['display_timezone'] }) }}><option value="UTC">UTC（協定世界時）</option><option value="Asia/Tokyo">日本時間（JST）</option></select></label><label>初期表示<select aria-label="初期表示" disabled={!settings || saving || importing} value={draft.default_view} onChange={e => { dirty.current = true; setDraft({ ...draft, default_view: e.target.value as ReaderSettings['default_view'] }) }}><option value="all">すべて</option><option value="unread">未読</option><option value="starred">スター付き</option></select></label><button disabled={!settings || saving || importing}>{saving ? '保存中…' : '表示設定を保存'}</button></form>
      <div className="settings-form browser-presentation"><label>記事の表示形式<select aria-label="設定の記事表示形式" value={presentation.layout} onChange={e => onLayout(e.target.value as Layout)}><option value="standard">標準</option><option value="line">1行</option><option value="tiles">タイル</option></select></label><label>テーマ<select aria-label="設定のテーマ" value={presentation.appearance} onChange={e => onTheme(e.target.value as Presentation['appearance'])}><option value="system">システム</option><option value="light">ライト</option><option value="dark">ダーク</option></select></label></div><p>表示形式・テーマ・サイドバー幅・フォルダーの開閉はこのブラウザに保存します。</p>
    </section>
    <section className="collection-section"><h2 id="settings-export" tabIndex={-1}>設定の持ち出し</h2><p>収集・表示設定、購読、配置順とこのブラウザの表示設定をJSONで書き出します。認証情報を含む可能性のある値は省略します。記事や履歴は含まれません。設定ファイルから共有設定とフォルダーを読み込めます。</p><button className="export-settings" disabled={exporting || !settings} onClick={() => void download()}>{exporting ? '書き出しています…' : '設定をエクスポート'}</button><p>このファイルはデータ全体のバックアップではありません。</p></section>
    <section className="collection-section"><h2>設定の読み込み</h2><p>表示・自動収集・API上限とフォルダー名を読み込みます。確認してから適用します。</p><label>設定JSONファイル <input type="file" accept="application/json,.json" disabled={importing || saving} onChange={e => { const file = e.currentTarget.files?.[0]; e.currentTarget.value = ''; void readImport(file) }} /></label>{preview && <div aria-label="読み込みプレビュー"><h3>読み込みプレビュー</h3><p>表示タイムゾーン: {preview.expected.reader.display_timezone} → {preview.desired.reader.display_timezone}</p><p>初期表示: {preview.expected.reader.default_view} → {preview.desired.reader.default_view}</p><p>自動収集: {preview.expected.collection.enabled ? '有効' : '無効'} → {preview.desired.collection.enabled ? '有効' : '無効'}、間隔: {preview.expected.collection.interval_seconds} → {preview.desired.collection.interval_seconds} 秒</p><p>API上限: {preview.expected.rate_limit} → {preview.desired.rate_limit} 回／分</p><p>フォルダー: 追加 {preview.desired.folders.filter(name => !preview.expected.folders.includes(name)).length} 件、登録済み {preview.desired.folders.filter(name => preview.expected.folders.includes(name)).length} 件、省略 {preview.omitted} 件。既存の並びや購読は変更しません。</p><ul>{preview.desired.folders.map(name => <li key={name}>{name}（{preview.expected.folders.includes(name) ? '登録済み' : '追加'}）</li>)}</ul><p>購読は対象外です。移す場合はOPMLを使ってください。APIキー・秘密情報とフォルダーの開閉は読み込みません。</p>{preview.presentation && <label><input type="checkbox" checked={importPresentation} onChange={event => setImportPresentation(event.target.checked)} /> このブラウザーにも表示設定を適用（テーマ: {presentation.appearance} → {preview.presentation.appearance}、形式: {presentation.layout} → {preview.presentation.layout}、幅: {presentation.width} → {preview.presentation.width} px）</label>}<button type="button" disabled={!settings || importing || saving} onClick={() => void applyImport()}>{importing ? '適用中…' : '設定を適用'}</button> <button type="button" disabled={importing} onClick={() => setPreview(null)}>キャンセル</button></div>}</section>
    {notice && <p className="success-message" role="status">{notice}</p>}{error && <div className="error-message" role="alert">{error}</div>}
  </section>
}
