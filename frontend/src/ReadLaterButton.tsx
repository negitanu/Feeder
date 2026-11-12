import { useState } from 'react'
import { Bookmark } from 'lucide-react'
import { apiFetch } from './apiFetch'
import './read-later.css'

type Props = { id: number; saved: boolean; onChanged: (saved: boolean) => void; compact?: boolean; mobileLabel?: boolean; toolbar?: boolean }
export default function ReadLaterButton({ id, saved, onChanged, compact = false, mobileLabel = false, toolbar = false }: Props) {
  const [pending, setPending] = useState(false)
  const [error, setError] = useState('')
  async function toggle() {
    setPending(true); setError('')
    try {
      const response = await apiFetch(`/api/articles/${id}/read-later`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ saved: !saved }) })
      if (!response.ok) throw new Error('あとで読むを更新できませんでした。')
      const result = await response.json() as { saved: boolean }
      onChanged(result.saved)
    } catch { setError('あとで読むを更新できませんでした。') } finally { setPending(false) }
  }
  return <span className="read-later-button-wrap"><button type="button" className={`read-later-button${compact ? ' read-later-button-compact' : ''}`} aria-label={saved ? 'あとで読むから削除' : 'あとで読むに保存'} title={saved ? 'あとで読むから削除' : 'あとで読むに保存'} aria-pressed={saved} disabled={pending} onClick={toggle}>{compact || toolbar ? <><Bookmark className="ui-icon" aria-hidden="true" fill={saved ? 'currentColor' : 'none'} />{mobileLabel && <span>あとで</span>}{toolbar && <span className="toolbar-label">あとで読む</span>}</> : saved ? 'あとで読むから削除' : 'あとで読む'}</button>{error && <span role="alert" className="read-later-error">{error}</span>}</span>
}
