import { useRef, useState } from 'react'
import './bulk-actions.css'

type ArticleState = { read?: boolean; starred?: boolean }
type Action = 'read' | 'unread' | 'star' | 'unstar' | ''

type ArticleBulkActionsProps = {
  count: number
  busy: boolean
  onApply: (state: ArticleState) => Promise<void>
  onClear: () => void
  onExport: () => void
}

const actionStates: Record<Exclude<Action, ''>, ArticleState> = {
  read: { read: true },
  unread: { read: false },
  star: { starred: true },
  unstar: { starred: false },
}

function ArticleBulkActions({ count, busy, onApply, onClear, onExport }: ArticleBulkActionsProps) {
  const [action, setAction] = useState<Action>('')
  const [applying, setApplying] = useState(false)
  const applyingRef = useRef(false)
  const disabled = busy || applying

  async function applyAction() {
    if (!action || disabled || applyingRef.current) return

    applyingRef.current = true
    setApplying(true)
    try {
      await onApply(actionStates[action])
      setAction('')
    } catch {
      // Keep the selection so the user can retry after a rejected callback.
    } finally {
      applyingRef.current = false
      setApplying(false)
    }
  }

  function clearSelection() {
    if (disabled || applyingRef.current) return
    onClear()
    setAction('')
  }

  return (
    <div className="bulk-toolbar bulk-actions" role="group" aria-label="選択した記事の一括操作" aria-busy={disabled}>
      <span className="bulk-actions-count" aria-live="polite">{count} 件選択中</span>
      <label className="bulk-actions-select-label">
        <span className="sr-only">一括操作</span>
        <select
          aria-label="一括操作"
          value={action}
          disabled={disabled}
          onChange={event => setAction(event.target.value as Action)}
        >
          <option value="">操作を選択</option>
          <option value="read">既読にする</option>
          <option value="unread">未読にする</option>
          <option value="star">スターを付ける</option>
          <option value="unstar">スターを解除</option>
        </select>
      </label>
      <button type="button" disabled={disabled || !action} onClick={() => void applyAction()}>
        実行
      </button>
      <button type="button" disabled={disabled} onClick={clearSelection}>
        選択解除
      </button>
      <button type="button" disabled={disabled} onClick={onExport}>リンクを書き出す</button>
    </div>
  )
}

export default ArticleBulkActions
