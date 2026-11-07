import { useCallback, useEffect, useState } from 'react'
import { apiFetch } from './apiFetch'
import type { Feed } from './LibrarySidebar'
import './mute-rules.css'

type Field = 'title' | 'body' | 'url' | 'both'
type Rule = { id: number; name: string; term: string; field: Field; feed_id: number | null; enabled: boolean; revision: number }
type Draft = Pick<Rule, 'name' | 'term' | 'field' | 'feed_id' | 'enabled'>
const blank = (): Draft => ({ name: '', term: '', field: 'both', feed_id: null, enabled: true })
const labels: Record<Field, string> = { title: 'タイトル', body: '本文', url: 'URL', both: 'タイトルと本文' }
export default function MuteRulesPanel({ feeds, onChanged }: { feeds: Feed[]; onChanged?: () => void }) {
  const [rules, setRules] = useState<Rule[]>([])
  const [draft, setDraft] = useState<Draft>(blank)
  const [editing, setEditing] = useState<Rule | null>(null)
  const [busy, setBusy] = useState(false)
  const [notice, setNotice] = useState('')
  const load = useCallback(async () => {
    const response = await apiFetch('/api/mute-rules')
    if (!response.ok) throw new Error()
    setRules((await response.json() as { rules: Rule[] }).rules)
  }, [])
  useEffect(() => { void load().catch(() => setNotice('ルールを読み込めませんでした。')) }, [load])
  async function mutate(url: string, method: string, body: object) {
    setBusy(true); setNotice('')
    try {
      const response = await apiFetch(url, { method, headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) })
      if (!response.ok) {
        setNotice(response.status === 409 ? 'ルールが変更されたか、上限に達しました。一覧を更新しました。' : '保存できませんでした。入力と配信元を確認してください。')
        await load(); return false
      }
      await load(); onChanged?.(); return true
    } catch { setNotice('通信に失敗しました。一覧を更新してください。'); return false }
    finally { setBusy(false) }
  }
  async function submit(event: React.FormEvent) {
    event.preventDefault()
    if (!draft.name.trim() || !draft.term.trim()) { setNotice('名前と検索文字列を入力してください。'); return }
    const ok = editing
      ? await mutate(`/api/mute-rules/${editing.id}`, 'PATCH', { ...draft, revision: editing.revision })
      : await mutate('/api/mute-rules', 'POST', draft)
    if (ok) { setEditing(null); setDraft(blank()); setNotice('ルールを保存しました。') }
  }
  async function toggle(rule: Rule) {
    if (await mutate(`/api/mute-rules/${rule.id}`, 'PATCH', { enabled: !rule.enabled, revision: rule.revision })) setNotice('ルールを更新しました。')
  }
  async function remove(rule: Rule) {
    if (await mutate(`/api/mute-rules/${rule.id}`, 'DELETE', { revision: rule.revision })) {
      if (editing?.id === rule.id) { setEditing(null); setDraft(blank()) }
      setNotice('ルールを削除しました。')
    }
  }
  return <section className="mute-rules" aria-label="ミュートルール">
    <h2>ミュートルール</h2><p>一致する記事を一覧から隠します。記事や既読・スターは変更されません。</p>
    <form onSubmit={event => void submit(event)}>
      <label>名前<input required maxLength={80} value={draft.name} onChange={e => setDraft({ ...draft, name: e.target.value })} /></label>
      <label>検索文字列<input required maxLength={200} value={draft.term} onChange={e => setDraft({ ...draft, term: e.target.value })} /></label>
      <label>検索対象<select value={draft.field} onChange={e => setDraft({ ...draft, field: e.target.value as Field })}>{Object.entries(labels).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
      <label>配信元<select value={draft.feed_id ?? ''} onChange={e => setDraft({ ...draft, feed_id: e.target.value ? Number(e.target.value) : null })}><option value="">すべての配信元</option>{feeds.map(feed => <option key={feed.id} value={feed.id}>{feed.title || feed.url}</option>)}</select></label>
      <label className="mute-rules-check"><input type="checkbox" checked={draft.enabled} onChange={e => setDraft({ ...draft, enabled: e.target.checked })} />有効</label>
      <div className="mute-rules-actions"><button disabled={busy || (!editing && rules.length >= 100)}>{editing ? '変更を保存' : 'ルールを追加'}</button>{editing && <button type="button" onClick={() => { setEditing(null); setDraft(blank()) }}>編集をキャンセル</button>}</div>
    </form>
    {notice && <p role="status">{notice}</p>}
    <ul>{rules.map(rule => <li key={rule.id}><div><strong>{rule.name}</strong><p>{labels[rule.field]} · {feeds.find(feed => feed.id === rule.feed_id)?.title || (rule.feed_id === null ? 'すべての配信元' : '配信元')} · {rule.enabled ? '有効' : '停止中'}</p><p className="mute-rules-term">{rule.term}</p></div><div className="mute-rules-actions"><button type="button" disabled={busy} onClick={() => { setEditing(rule); setDraft({ name: rule.name, term: rule.term, field: rule.field, feed_id: rule.feed_id, enabled: rule.enabled }) }}>編集<span className="mute-rules-sr"> {rule.name}</span></button><button type="button" disabled={busy} onClick={() => void toggle(rule)}>{rule.enabled ? '停止' : '有効にする'}<span className="mute-rules-sr"> {rule.name}</span></button><button type="button" disabled={busy} onClick={() => void remove(rule)}>削除<span className="mute-rules-sr"> {rule.name}</span></button></div></li>)}</ul>
    {rules.length === 0 && <p>ルールはありません。</p>}
  </section>
}
