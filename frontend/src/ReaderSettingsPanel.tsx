import { apiFetch } from './apiFetch'
import { useEffect, useRef, useState } from 'react'
export type ReaderSettings = { display_timezone: 'UTC' | 'Asia/Tokyo'; default_view: 'all' | 'unread' | 'starred' }
export type Layout = 'standard' | 'line' | 'tiles'
export type Presentation = { appearance: 'system' | 'light' | 'dark'; width: number; layout: Layout; collapsed_folders: number[] }
export function storedLayout(): Layout { try { const value = localStorage.getItem('feeder-layout'); return value === 'line' || value === 'tiles' ? value : 'standard' } catch { return 'standard' } }
export default function ReaderSettingsPanel({ settings, presentation, onSaved, onLayout, onTheme, onRetry }: { settings: ReaderSettings | null; presentation: Presentation; onSaved: (value: ReaderSettings) => void; onLayout: (value: Layout) => void; onTheme: (value: Presentation['appearance']) => void; onRetry: () => void }) {
  const [draft, setDraft] = useState<ReaderSettings>(settings ?? { display_timezone: 'UTC', default_view: 'all' })
  const [saving, setSaving] = useState(false)
  const [exporting, setExporting] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
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
    <section className="collection-section"><h2>表示設定</h2>{!settings && <p role="status">表示設定を取得しています。<button type="button" onClick={onRetry}>表示設定を再取得</button></p>}<p>タイムゾーンと初期表示はこのライブラリを使うブラウザで共有します。日付検索もこのタイムゾーンで判定します。</p>
      <form className="settings-form" onSubmit={e => void save(e)}><label>表示タイムゾーン<select aria-label="表示タイムゾーン" disabled={!settings || saving} value={draft.display_timezone} onChange={e => { dirty.current = true; setDraft({ ...draft, display_timezone: e.target.value as ReaderSettings['display_timezone'] }) }}><option value="UTC">UTC（協定世界時）</option><option value="Asia/Tokyo">日本時間（JST）</option></select></label><label>初期表示<select aria-label="初期表示" disabled={!settings || saving} value={draft.default_view} onChange={e => { dirty.current = true; setDraft({ ...draft, default_view: e.target.value as ReaderSettings['default_view'] }) }}><option value="all">すべて</option><option value="unread">未読</option><option value="starred">スター付き</option></select></label><button disabled={!settings || saving}>{saving ? '保存中…' : '表示設定を保存'}</button></form>
      <div className="settings-form browser-presentation"><label>記事の表示形式<select aria-label="設定の記事表示形式" value={presentation.layout} onChange={e => onLayout(e.target.value as Layout)}><option value="standard">標準</option><option value="line">1行</option><option value="tiles">タイル</option></select></label><label>テーマ<select aria-label="設定のテーマ" value={presentation.appearance} onChange={e => onTheme(e.target.value as Presentation['appearance'])}><option value="system">システム</option><option value="light">ライト</option><option value="dark">ダーク</option></select></label></div><p>表示形式・テーマ・サイドバー幅・フォルダーの開閉はこのブラウザに保存します。</p>
    </section>
    <section className="collection-section"><h2>設定の持ち出し</h2><p>収集・表示設定、購読、配置順とこのブラウザの表示設定をJSONで書き出します。認証情報を含む可能性のある値は省略します。記事や履歴は含まれません。設定全体の読み込みにはまだ対応していません。</p><button className="export-settings" disabled={exporting || !settings} onClick={() => void download()}>{exporting ? '書き出しています…' : '設定をエクスポート'}</button><p>このファイルはデータ全体のバックアップではありません。</p></section>
    {notice && <p className="success-message" role="status">{notice}</p>}{error && <div className="error-message" role="alert">{error}</div>}
  </section>
}
