import { GripVertical, ChevronRight, ChevronDown, Folder as FolderIcon, Rss, MoreHorizontal } from 'lucide-react'
import { apiFetch } from './apiFetch'
import { useRef, useState } from 'react'
import type { KeyboardEvent, PointerEvent } from 'react'
export type Folder = { id: number; name: string }
export type Feed = { id: number; title: string; url: string; folder_id: number | null; paused: boolean; poll_interval: number | null; last_error?: string | null; last_checked?: string | null }
type Item = { kind: 'folder' | 'feed'; id: number }
type Snapshot = { folders: number[]; feeds: { id: number; folder_id: number | null }[] }
type Target = Item & { after: boolean }
type Props = {
  folders: Folder[]; feeds: Feed[]; folderUnread: Record<number, number>; source: Item | null; collapsed: number[];
  onToggle: (id: number) => void; onSelect: (item: Item) => void;
  onEditFeed: (feed: Feed) => void; onEditFolder: (folder: Folder) => void;
  onMutate: (url: string, init: RequestInit) => Promise<boolean>;
  onReload: () => Promise<void>;
  onMembershipChanged: (folderIds: (number | null)[]) => void;
}
export default function LibrarySidebar(p: Props) {
  const [menu, setMenu] = useState<number | null>(null)
  const [saving, setSaving] = useState(false)
  const savingRef = useRef(false)
  const [message, setMessage] = useState('')
  const [target, setTarget] = useState<Target | null>(null)
  const drag = useRef<{ item: Item; x: number; y: number; moved: boolean; target: Target | null; expected: Snapshot } | null>(null)
  const snapshot = (): Snapshot => ({ folders: p.folders.map(f => f.id), feeds: [...p.folders.map(f => f.id), null].flatMap(folder => p.feeds.filter(f => f.folder_id === folder).map(f => ({ id: f.id, folder_id: folder }))) })
  function group(s: Snapshot, folder: number | null) { return s.feeds.filter(f => f.folder_id === folder) }
  function canonical(s: Snapshot): Snapshot { return { folders: s.folders, feeds: [...s.folders, null].flatMap(folder => group(s, folder)) } }
  async function save(expected: Snapshot, order: Snapshot, item: Item) {
    if (savingRef.current || JSON.stringify(expected) === JSON.stringify(order)) return
    savingRef.current = true; setSaving(true); setMessage('')
    try {
      const response = await apiFetch('/api/library/order', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ expected, order }) })
      if (!response.ok) throw new Error(response.status === 409 ? '別の操作で構成が変わりました。最新の順序を読み込みました。' : '順序を保存できませんでした。再度お試しください。')
      const changed = order.feeds.filter(f => expected.feeds.find(old => old.id === f.id)?.folder_id !== f.folder_id)
      if (changed.length) p.onMembershipChanged(changed.flatMap(f => [f.folder_id,expected.feeds.find(old => old.id === f.id)!.folder_id]))
      setMessage('順序を保存しました。')
    } catch (e) { setMessage((e as Error).message) }
    finally {
      await p.onReload(); savingRef.current = false; setSaving(false)
      requestAnimationFrame(() => document.querySelector<HTMLButtonElement>(`[data-order-handle="${item.kind}-${item.id}"]`)?.focus({ preventScroll: true }))
    }
  }
  function move(item: Item, to: Target, expected = snapshot()) {
    const order = structuredClone(expected)
    if (item.kind === 'folder') {
      if (to.kind !== 'folder' || item.id === to.id) return
      order.folders = order.folders.filter(id => id !== item.id)
      order.folders.splice(order.folders.indexOf(to.id) + Number(to.after), 0, item.id)
    } else {
      if (to.kind === 'feed' && item.id === to.id) return
      const moving = order.feeds.find(f => f.id === item.id)
      if (!moving) return
      const folder = to.kind === 'folder' ? (to.id === 0 ? null : to.id) : order.feeds.find(f => f.id === to.id)?.folder_id
      if (folder === undefined) return
      order.feeds = order.feeds.filter(f => f.id !== item.id)
      moving.folder_id = folder
      const siblings = group(order, folder)
      const at = to.kind === 'folder' ? siblings.length : siblings.findIndex(f => f.id === to.id) + Number(to.after)
      siblings.splice(at, 0, moving)
      order.feeds = [...order.feeds.filter(f => f.folder_id !== folder), ...siblings]
    }
    void save(expected, canonical(order), item)
  }
  function keyboard(e: KeyboardEvent<HTMLButtonElement>, item: Item) {
    if (!['ArrowUp','ArrowDown','Home','End'].includes(e.key)) return
    e.preventDefault()
    if (savingRef.current) return
    const s = snapshot(), feed = s.feeds.find(f => f.id === item.id)
    const ids = item.kind === 'folder' ? s.folders : group(s, feed?.folder_id ?? null).map(f => f.id)
    const current = ids.indexOf(item.id)
    const index = e.key === 'Home' ? 0 : e.key === 'End' ? ids.length - 1 : Math.max(0,Math.min(ids.length - 1,current + (e.key === 'ArrowUp' ? -1 : 1)))
    if (current !== index) move(item, { kind: item.kind, id: ids[index], after: index > current })
  }
  function pointerDown(e: PointerEvent<HTMLButtonElement>, item: Item) {
    if (savingRef.current || !e.isPrimary || e.button !== 0) return
    e.currentTarget.setPointerCapture(e.pointerId)
    drag.current = { item, x: e.clientX, y: e.clientY, moved: false, target: null, expected: snapshot() }
  }
  function pointerMove(e: PointerEvent<HTMLButtonElement>) {
    const d = drag.current
    if (!d) return
    if (Math.hypot(e.clientX-d.x,e.clientY-d.y) < 6 && !d.moved) return
    d.moved = true
    const el = document.elementFromPoint(e.clientX,e.clientY)?.closest<HTMLElement>('[data-order-target]')
    let next: Target | null = null
    if (el) {
      const kind = el.dataset.orderTarget as Item['kind'], id = Number(el.dataset.orderId)
      if ((d.item.kind === 'feed' || kind === 'folder') && !(kind === d.item.kind && id === d.item.id)) {
        const rect = el.getBoundingClientRect()
        next = { kind, id, after: e.clientY > rect.top + rect.height / 2 }
      }
    }
    d.target = next; setTarget(next)
  }
  function pointerEnd(e: PointerEvent<HTMLButtonElement>, cancel = false) {
    const d = drag.current; drag.current = null; setTarget(null)
    if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId)
    if (!cancel && d?.moved && d.target) move(d.item, d.target, d.expected)
  }
  function handle(item: Item, name: string) {
    return <button type="button" className="order-handle" data-order-handle={`${item.kind}-${item.id}`} aria-label={`${name}の順序を変更`} aria-describedby="order-help" aria-disabled={saving} onKeyDown={e => keyboard(e,item)} onPointerDown={e => pointerDown(e,item)} onPointerMove={pointerMove} onPointerUp={e => pointerEnd(e)} onPointerCancel={e => pointerEnd(e,true)} onLostPointerCapture={() => { drag.current = null; setTarget(null) }} onClick={e => e.preventDefault()}><GripVertical className="ui-icon" aria-hidden="true" /></button>
  }
  function marker(kind: Item['kind'], id: number) { return target?.kind === kind && target.id === id ? (kind === 'folder' && drag.current?.item.kind === 'feed' ? 'inside' : target.after ? 'after' : 'before') : undefined }
  function feedRow(feed: Feed) {
    const title = feed.title || feed.url
    return <div className="folder-row feed-row" key={feed.id} data-order-target="feed" data-order-id={feed.id} data-drop={marker('feed',feed.id)} onContextMenu={e => { e.preventDefault(); setMenu(feed.id) }}>
      {handle({ kind:'feed',id:feed.id },title)}<button className={`source-button feed-source ${p.source?.kind === 'feed' && p.source.id === feed.id ? 'active' : ''}`} onClick={() => p.onSelect({ kind:'feed',id:feed.id })}><Rss className="ui-icon" aria-hidden="true" /><span title={title}>{title}</span>{feed.paused && <small>停止中</small>}</button>
      <button className="mini-menu" aria-label={`${title}の操作`} aria-expanded={menu === feed.id} onClick={() => setMenu(menu === feed.id ? null : feed.id)}><MoreHorizontal className="ui-icon" aria-hidden="true" /></button>
      {menu === feed.id && <div className="feed-menu"><button onClick={() => { p.onEditFeed(feed); setMenu(null) }}>編集</button><button onClick={() => { p.onEditFeed(feed); setMenu(null) }}>フォルダーを移動</button><button onClick={() => { void p.onMutate(`/api/feeds/${feed.id}`,{ method:'PATCH', headers:{ 'Content-Type':'application/json' },body:JSON.stringify({ paused:!feed.paused }) }); setMenu(null) }}>{feed.paused ? '再開' : '一時停止'}</button><button onClick={() => { void p.onMutate(`/api/feeds/${feed.id}/refresh`,{ method:'POST' }); setMenu(null) }}>今すぐ更新</button><button className="danger" onClick={() => { if (window.confirm(`「${title}」を削除しますか？このフィードの記事も削除されます。`)) void p.onMutate(`/api/feeds/${feed.id}`,{ method:'DELETE' }); setMenu(null) }}>削除</button></div>}
    </div>
  }
  return <>
    <p id="order-help" className="sr-only">順序ハンドルをドラッグ、または上下矢印・Home・Endで並べ替えます。フィードはフォルダーの上へドラッグするか操作メニューから移動できます。</p>
    <div className="side-heading source-heading">フォルダー</div><nav className="source-list" aria-label="フォルダーとフィード">{p.folders.map(folder => {
      const isCollapsed = p.collapsed.includes(folder.id)
      return <div className="folder-group" key={folder.id}><div className="folder-row" data-order-target="folder" data-order-id={folder.id} data-drop={marker('folder',folder.id)} onContextMenu={e => { e.preventDefault(); p.onEditFolder(folder) }}>
        {handle({kind:'folder',id:folder.id},folder.name)}<button className="folder-toggle" aria-expanded={!isCollapsed} aria-label={`${folder.name}を${isCollapsed ? '展開' : '折りたたむ'}`} onClick={() => p.onToggle(folder.id)}>{isCollapsed ? <ChevronRight className="ui-icon" aria-hidden="true" /> : <ChevronDown className="ui-icon" aria-hidden="true" />}</button><button className={`source-button ${p.source?.kind === 'folder' && p.source.id === folder.id ? 'active' : ''}`} onClick={() => p.onSelect({kind:'folder',id:folder.id})}><FolderIcon className="ui-icon" aria-hidden="true" /><span title={folder.name}>{folder.name}</span>{p.folderUnread[folder.id] !== undefined && <span className="count unread-count" aria-label={`未読 ${p.folderUnread[folder.id]} 件`}>{p.folderUnread[folder.id]}</span>}</button><button className="mini-menu" aria-label={`${folder.name}を編集`} onClick={() => p.onEditFolder(folder)}><MoreHorizontal className="ui-icon" aria-hidden="true" /></button></div>{!isCollapsed && p.feeds.filter(f => f.folder_id === folder.id).map(feedRow)}</div>
    })}</nav><div className="side-heading source-heading unfiled-target" data-order-target="folder" data-order-id="0" data-drop={marker('folder',0)}>フィード（フォルダーなし）</div><nav className="source-list" aria-label="フォルダーなしのフィード">{p.feeds.filter(f => f.folder_id === null).map(feedRow)}</nav>
    <div className="sidebar-status" role="status" aria-live="polite">{saving ? '順序を保存しています…' : message}</div>
  </>
}
