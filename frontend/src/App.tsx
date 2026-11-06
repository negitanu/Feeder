import { apiFetch } from './apiFetch'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import CollectionPanel from './CollectionPanel'
import ApiKeysPanel from './ApiKeysPanel'
import LibrarySidebar from './LibrarySidebar'
import type { Folder, Feed } from './LibrarySidebar'
import SidebarWidth from './SidebarWidth'
import ReaderSettingsPanel, { storedLayout } from './ReaderSettingsPanel'
import type { Layout, ReaderSettings } from './ReaderSettingsPanel'

type Article = { id: number; title: string; content: string; url: string; published: string; is_read: boolean; is_starred: boolean; last_visited?: string | null }
type Filter = 'all' | 'unread' | 'starred' | 'recent'
type AppliedFilters = { q?: string; unread?: boolean; starred?: boolean; history?: boolean; order: 'new' | 'old' | 'recent'; scope: 'both' | 'title' | 'body'; any?: string; exclude?: string; published_from?: string; published_to?: string; feed_id?: number; folder_id?: number; feed_ids?: string }
type Theme = 'light' | 'dark' | 'system'
const PAGE_SIZE = 24
function savedFolders(): number[] { try { const value = JSON.parse(localStorage.getItem('feeder-collapsed-folders') || '[]'); return Array.isArray(value) ? value.filter(Number.isInteger) : [] } catch { return [] } }

async function request<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await apiFetch(url, init)
  if (!response.ok) throw new Error(response.status === 404 ? '記事が見つかりません。' : response.status === 400 || response.status === 422 ? '検索条件または選択した記事を確認してください。' : '通信に失敗しました。時間をおいて再度お試しください。')
  return response.json() as Promise<T>
}

function App() {
  const [filter, setFilter] = useState<Filter>('all')
  const [readerSettings, setReaderSettings] = useState<ReaderSettings | null>(null)
  const [layout, setLayout] = useState<Layout>(storedLayout)
  const [sidebarWidth, setSidebarWidth] = useState(250)
  const initialViewChosen = useRef(false)
  const readerSettingsRef = useRef<ReaderSettings | null>(null)
  const readerSettingsRun = useRef(0)
  const [draftScope, setDraftScope] = useState({ scope: 'both' as AppliedFilters['scope'], any: '', exclude: '', published_from: '', published_to: '', feed_ids: '', order: 'new' as AppliedFilters['order'] })
  const [applied, setApplied] = useState<AppliedFilters>({ order: 'new', scope: 'both' })
  const [selectedIds, setSelectedIds] = useState<number[]>([])
  const [page, setPage] = useState<'read' | 'collect'>('read')
  const [folders, setFolders] = useState<Folder[]>([])
  const [feeds, setFeeds] = useState<Feed[]>([])
  const [source, setSource] = useState<{ kind: 'feed' | 'folder'; id: number } | null>(null)
  const [collapsed, setCollapsed] = useState<number[]>(savedFolders)
  const [busy, setBusy] = useState(false)
  const [refreshVersion, setRefreshVersion] = useState(0)
  const [editingFeed, setEditingFeed] = useState<Feed | null>(null)
  const [editingFolder, setEditingFolder] = useState<Folder | null>(null)
  const [query, setQuery] = useState('')
  const [search, setSearch] = useState('')
  const [items, setItems] = useState<Article[]>([])
  const [total, setTotal] = useState(0)
  const [selected, setSelected] = useState<Article | null>(null)
  const [theme, setTheme] = useState<Theme>(() => { try { const t = localStorage.getItem('feeder-theme'); return t === 'light' || t === 'dark' ? t : 'system' } catch { return 'system' } })
  const [loading, setLoading] = useState(true)
  const [moreLoading, setMoreLoading] = useState(false)
  const [error, setError] = useState('')
  const [libraryError, setLibraryError] = useState('')
  const generation = useRef(0)
  const libraryGeneration = useRef(0)
  const returnFocus = useRef<number | null>(null)
  const returnScroll = useRef(0)
  const sentinel = useRef<HTMLDivElement>(null)
  const offset = items.length
  const params = useMemo(() => {
    const p = new URLSearchParams({ limit: String(PAGE_SIZE), offset: '0' })
    const f: AppliedFilters = { ...applied, q: search.trim() || undefined, unread: filter === 'unread' || undefined, starred: filter === 'starred' || undefined, history: filter === 'recent' || undefined, feed_id: source?.kind === 'feed' ? source.id : undefined, folder_id: source?.kind === 'folder' ? source.id : undefined }
    Object.entries(f).forEach(([key, value]) => { if (value !== undefined && value !== '') p.set(key, String(value)) })
    return p.toString()
  }, [applied, filter, search, source])
  const activeFilters = useMemo<AppliedFilters>(() => ({ ...applied, q: search.trim() || undefined, unread: filter === 'unread' || undefined, starred: filter === 'starred' || undefined, history: filter === 'recent' || undefined, feed_id: source?.kind === 'feed' ? source.id : undefined, folder_id: source?.kind === 'folder' ? source.id : undefined }), [applied, filter, search, source])
  const filtersKey = JSON.stringify([activeFilters,readerSettings?.display_timezone])
  const filtersKeyRef = useRef(filtersKey)
  filtersKeyRef.current = filtersKey
  useEffect(() => { setSelectedIds([]) }, [filtersKey])

  const applyReaderSettings = useCallback((value: ReaderSettings) => {
    const previous = readerSettingsRef.current
    readerSettingsRef.current = value; setReaderSettings(value)
    if (!previous && !initialViewChosen.current) { setFilter(value.default_view); initialViewChosen.current = true }
    if (previous && previous.display_timezone !== value.display_timezone) { setSelectedIds([]); setRefreshVersion(version => version + 1) }
  }, [])
  const loadReaderSettings = useCallback(async () => {
    const run = ++readerSettingsRun.current
    try { const value = await request<ReaderSettings>('/api/reader/settings'); if (readerSettingsRun.current === run) applyReaderSettings(value) }
    catch { if (readerSettingsRun.current === run && !readerSettingsRef.current) { setError('表示設定を取得できませんでした。再試行してください。'); setLoading(false) } }
  }, [applyReaderSettings])
  useEffect(() => { void loadReaderSettings(); const refresh = () => { void loadReaderSettings() }; window.addEventListener('focus',refresh); return () => window.removeEventListener('focus',refresh) }, [loadReaderSettings])
  useEffect(() => { try { localStorage.setItem('feeder-layout',layout) } catch { /* Keep session presentation available. */ } }, [layout])
  function settingsSaved(value: ReaderSettings) { ++readerSettingsRun.current; applyReaderSettings(value) }

  const loadLibrary = useCallback(async () => {
    const current = ++libraryGeneration.current
    try { const result = await request<{ folders: Folder[]; feeds: Feed[] }>('/api/library'); if (current !== libraryGeneration.current) return; setFolders(result.folders); setFeeds(result.feeds); setSource(current => current && (current.kind === 'feed' ? !result.feeds.some(f => f.id === current.id) : !result.folders.some(f => f.id === current.id)) ? null : current); setLibraryError('') }
    catch (e) { if (current === libraryGeneration.current) setLibraryError((e as Error).message) }
  }, [])
  useEffect(() => { void loadLibrary() }, [loadLibrary])
  useEffect(() => { try { localStorage.setItem('feeder-collapsed-folders', JSON.stringify(collapsed)) } catch { /* Keep the current session usable when storage is unavailable. */ } }, [collapsed])

  useEffect(() => {
    try { localStorage.setItem('feeder-theme', theme) } catch { /* Storage may be unavailable. */ }
    document.documentElement.dataset.theme = theme
  }, [theme])
  useEffect(() => {
    if (!editingFeed && !editingFolder) return
    const close = (event: KeyboardEvent) => { if (event.key === 'Escape') { setEditingFeed(null); setEditingFolder(null) } }
    window.addEventListener('keydown', close); return () => window.removeEventListener('keydown', close)
  }, [editingFeed, editingFolder])
  useEffect(() => {
    if (selected || returnFocus.current === null) return
    const id = returnFocus.current
    returnFocus.current = null
    requestAnimationFrame(() => {
      window.scrollTo({ top: returnScroll.current })
      document.querySelector<HTMLButtonElement>(`[data-article-id="${id}"]`)?.focus({ preventScroll: true })
    })
  }, [selected])

  const loadFirst = useCallback(async () => {
    if (!readerSettings) return
    const currentGeneration = ++generation.current
    setLoading(true); setError(''); setSelected(null)
    try {
      const result = await request<{ items: Article[]; total: number }>(`/api/articles?${params}`)
      if (currentGeneration !== generation.current) return
      setItems(result.items); setTotal(result.total)
    } catch (e) { if (currentGeneration === generation.current) { setItems([]); setTotal(0); setError((e as Error).message) } }
    finally { if (currentGeneration === generation.current) setLoading(false) }
  }, [params, refreshVersion, readerSettings?.display_timezone, !!readerSettings])
  useEffect(() => { void loadFirst() }, [loadFirst])

  const loadMore = useCallback(async () => {
    if (!readerSettings || moreLoading || loading || items.length >= total) return
    setMoreLoading(true)
    const currentGeneration = generation.current
    try {
      const p = new URLSearchParams(params); p.set('offset', String(offset))
      const result = await request<{ items: Article[]; total: number }>(`/api/articles?${p}`)
      if (currentGeneration !== generation.current) return
      setItems(current => [...current, ...result.items.filter(a => !current.some(c => c.id === a.id))]); setTotal(result.total)
    } catch (e) { setError((e as Error).message) }
    finally { setMoreLoading(false) }
  }, [items.length, loading, moreLoading, offset, params, total, readerSettings])
  useEffect(() => {
    const node = sentinel.current
    if (!node || typeof IntersectionObserver === 'undefined') return
    const observer = new IntersectionObserver(entries => { if (entries[0]?.isIntersecting) void loadMore() }, { rootMargin: '500px' })
    observer.observe(node); return () => observer.disconnect()
  }, [loadMore])

  function eligible(item: Article) { return !(filter === 'unread' && item.is_read) && !(filter === 'starred' && !item.is_starred) }
  function ordered(rows: Article[]) {
    return [...rows].sort((a,b) => {
      if (applied.order === 'recent') return (b.last_visited || '').localeCompare(a.last_visited || '') || b.id-a.id
      const ad = Date.parse(a.published), bd = Date.parse(b.published)
      if (Number.isNaN(ad) !== Number.isNaN(bd)) return Number.isNaN(ad) ? 1 : -1
      return (applied.order === 'old' ? 1 : -1) * ((Number.isNaN(ad) ? 0 : ad-bd) || a.id-b.id)
    })
  }
  async function updateState(article: Article, state: { read?: boolean; starred?: boolean }) {
    const key = filtersKey; setError('')
    try {
      const updated = await request<Article>(`/api/articles/${article.id}/state`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(state) })
      if (filtersKeyRef.current !== key) return
      setSelected(current => current?.id === updated.id ? updated : current)
      const was = eligible(article), now = eligible(updated)
      setItems(current => now ? ordered(current.some(item => item.id === updated.id) ? current.map(item => item.id === updated.id ? updated : item) : [...current,updated]) : current.filter(item => item.id !== updated.id))
      if (was !== now) setTotal(value => Math.max(0,value+(now ? 1 : -1)))
      if (!now) setSelectedIds(current => current.filter(id => id !== updated.id))
    } catch (e) { if (filtersKeyRef.current === key) setError((e as Error).message) }
  }
  async function openArticle(article: Article) {
    returnFocus.current = article.id; returnScroll.current = window.scrollY
    const key = filtersKey; setError('')
    try {
      const full = await request<Article>(`/api/articles/${article.id}/visit`, { method: 'POST' })
      if (filtersKeyRef.current !== key) return
      setSelected(full)
      setItems(current => !eligible(full) ? current.filter(item => item.id !== full.id) : ordered(current.map(item => item.id === full.id ? full : item)))
      if (eligible(article) && !eligible(full)) { setTotal(value => Math.max(0,value-1)); setSelectedIds(current => current.filter(id => id !== full.id)) }
    } catch (e) { if (filtersKeyRef.current === key) setError((e as Error).message) }
  }
  function submitSearch(e: React.FormEvent) { e.preventDefault(); setSearch(query) }
  async function applyAdvanced(e: React.FormEvent) { e.preventDefault(); setApplied({ order: draftScope.order, scope: draftScope.scope, any: draftScope.any.trim() || undefined, exclude: draftScope.exclude.trim() || undefined, published_from: draftScope.published_from || undefined, published_to: draftScope.published_to || undefined, feed_ids: draftScope.feed_ids || undefined }); setSelected(null) }
  const [notice, setNotice] = useState('')
  const [bulkBusy, setBulkBusy] = useState(false)
  async function markAllRead() {
    const key = filtersKey; setError(''); setBulkBusy(true)
    try {
      const result = await request<{ updated: number }>('/api/articles/read-all', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ filters: activeFilters }) })
      if (filtersKeyRef.current !== key) return
      if (filter === 'unread') { setItems([]); setTotal(0) } else setItems(current => current.map(item => ({ ...item, is_read: true })))
      setSelectedIds([]); setNotice(`全て既読にしました（${result.updated} 件）`)
    } catch (e) { if (filtersKeyRef.current === key) setError((e as Error).message) } finally { setBulkBusy(false) }
  }
  async function bulkState(state: { read?: boolean; starred?: boolean }) {
    if (!selectedIds.length) return
    const key = filtersKey; const ids = new Set(selectedIds); setError(''); setBulkBusy(true)
    try {
      const result = await request<{ updated: number }>('/api/articles/bulk-state', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ ids: selectedIds, ...state, filters: activeFilters }) })
      if (filtersKeyRef.current !== key) return
      const leavesScope = (filter === 'unread' && state.read === true) || (filter === 'starred' && state.starred === false)
      setItems(current => current.flatMap(item => !ids.has(item.id) ? [item] : leavesScope ? [] : [{ ...item, ...(state.read !== undefined ? { is_read: state.read } : {}), ...(state.starred !== undefined ? { is_starred: state.starred } : {}) }]))
      if (leavesScope) { setTotal(value => Math.max(0,value-result.updated)); setSelectedIds([]) }
      setNotice(`${result.updated} 件を更新しました`)
    } catch (e) { if (filtersKeyRef.current === key) setError((e as Error).message) } finally { setBulkBusy(false) }
  }
  async function mutate(url: string, init: RequestInit, reload = true) {
    setBusy(true); setLibraryError('')
    try { const response = await apiFetch(url, init); if (!response.ok) throw new Error(response.status === 404 ? '見つかりません。' : '操作に失敗しました。時間をおいて再度お試しください。'); if (reload) { await loadLibrary(); setRefreshVersion(value => value + 1) } return true }
    catch (e) { setLibraryError((e as Error).message); return false }
    finally { setBusy(false) }
  }
  const json = (body: unknown): RequestInit => ({ headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) })
  function selectSource(value: { kind: 'feed' | 'folder'; id: number } | null) { initialViewChosen.current = true; setSource(value); setPage('read'); setSelected(null) }
  const date = (value: string) => {
    const normalized = /^\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}(:\d{2}(\.\d+)?)?$/.test(value) ? value.replace(' ','T') + 'Z' : value
    const d = new Date(normalized)
    return Number.isNaN(d.getTime()) ? '公開日不明' : new Intl.DateTimeFormat('ja-JP', { dateStyle: 'medium', timeZone: readerSettings?.display_timezone ?? 'UTC' }).format(d)
  }

  return <div className="app-shell">
    <header className="topbar">
      <a className="brand" href="#" onClick={e => { e.preventDefault(); setPage('read'); setSelected(null); setSource(null) }} aria-label="Feeder ホーム"><span className="brand-mark">f</span><span>feeder</span></a>
      <nav className="global-nav" aria-label="メインナビゲーション"><button className={page === 'read' ? 'active' : ''} onClick={() => { setPage('read'); setSelected(null) }}>読む</button><button className={page === 'collect' ? 'active' : ''} onClick={() => { setPage('collect'); setSelected(null) }}>集める</button></nav>
      <form className="search" onSubmit={submitSearch} role="search"><span aria-hidden="true">⌕</span><input aria-label="記事を検索" placeholder="記事を検索..." value={query} onChange={e => setQuery(e.target.value)} /><button type="submit" aria-label="検索">検索</button></form>
      <label className="theme-control"><span className="sr-only">テーマ</span><select aria-label="テーマ" value={theme} onChange={e => setTheme(e.target.value as Theme)}><option value="system">システム</option><option value="light">ライト</option><option value="dark">ダーク</option></select><span aria-hidden="true">◐</span></label>
    </header>
    <SidebarWidth onWidthChange={setSidebarWidth}>
      <aside id="reader-sidebar" className="sidebar" aria-label="記事フィルター"><div className="side-heading">ライブラリ</div><nav className="filters">
        {([['all', 'すべて', '▤'], ['unread', '未読', '◉'], ['starred', 'スター付き', '☆'], ['recent', '最近読んだ記事', '◷']] as [Filter, string, string][]).map(([key, label, icon]) => <button key={key} className={`filter ${filter === key ? 'active' : ''}`} onClick={() => { initialViewChosen.current = true; setFilter(key); setSource(null); setSelected(null); setPage('read'); if (key === 'recent' || filter === 'recent') { const order = key === 'recent' ? 'recent' : 'new'; setApplied(current => ({ ...current, order })); setDraftScope(current => ({ ...current, order })) } }} aria-current={filter === key ? 'page' : undefined}><span className="filter-icon">{icon}</span>{label}{key === 'all' && <span className="count">{total}</span>}</button>)}
      </nav><LibrarySidebar folders={folders} feeds={feeds} source={source} collapsed={collapsed} onToggle={id => setCollapsed(current => current.includes(id) ? current.filter(value => value !== id) : [...current,id])} onSelect={selectSource} onEditFeed={setEditingFeed} onEditFolder={setEditingFolder} onMutate={mutate} onReload={loadLibrary} onMembershipChanged={ids => { if (source?.kind === 'folder' && ids.includes(source.id)) { setSelectedIds([]); setRefreshVersion(value => value + 1) } }} />{libraryError && <div className="sidebar-error" role="alert">{libraryError}</div>}</aside>
      <main className="main-area">
        {page === 'collect' && <><CollectionPanel folders={folders} onChanged={() => { void loadLibrary(); setRefreshVersion(value => value + 1) }} onCreateFolder={name => mutate('/api/folders', { method: 'POST', ...json({ name }) })} /><ReaderSettingsPanel settings={readerSettings} presentation={{ appearance: theme, width: Math.round(sidebarWidth), layout, collapsed_folders: [...new Set(collapsed.filter(id => id > 0 && folders.some(folder => folder.id === id)))].slice(0,2000) }} onSaved={settingsSaved} onLayout={setLayout} onTheme={setTheme} onRetry={() => void loadReaderSettings()} /><ApiKeysPanel />{libraryError && <div className="error-message" role="alert">{libraryError}</div>}</>}
        {page === 'read' && <>
        {selected && <section className="article-detail" aria-label="記事の詳細">
          <div className="detail-toolbar"><button className="back-button" onClick={() => setSelected(null)}>← <span>一覧に戻る</span></button><button onClick={() => void updateState(selected, { read: !selected.is_read })}>{selected.is_read ? '未読にする' : '既読にする'}</button><button className={`icon-button star-button ${selected.is_starred ? 'is-starred' : ''}`} aria-label={selected.is_starred ? 'スターを解除' : 'スターを付ける'} aria-pressed={selected.is_starred} onClick={() => void updateState(selected, { starred: !selected.is_starred })}>{selected.is_starred ? '★' : '☆'}</button></div>
          {error && <div className="error-message" role="alert">{error}</div>}
          <article className="article-body"><div className="article-meta"><span>{date(selected.published)}</span><span className="meta-separator">·</span>{(() => { try { const url = new URL(selected.url); return ['http:', 'https:'].includes(url.protocol) ? <a href={url.href} target="_blank" rel="noreferrer">元の記事を開く ↗</a> : null } catch { return null } })()}</div><h1>{selected.title}</h1><div className="article-text">{selected.content}</div></article>
        </section>}
        <section hidden={!!selected}>
          <section className="list-heading"><div><div className="eyebrow">YOUR READING SPACE</div><h1>{source ? (source.kind === 'feed' ? feeds.find(f => f.id === source.id)?.title : folders.find(f => f.id === source.id)?.name) || '記事' : filter === 'all' ? '記事' : filter === 'unread' ? '未読' : filter === 'recent' ? '最近読んだ記事' : 'スター付き'}</h1><p>{search ? `「${search}」の検索結果` : '保存した記事を、あなたのペースで。'}</p></div><span className="result-count">{total} 件</span></section>
          <div className="reader-actions"><label className="layout-control">表示形式<select aria-label="記事の表示形式" value={layout} onChange={e => setLayout(e.target.value as Layout)}><option value="standard">標準</option><option value="line">1行</option><option value="tiles">タイル</option></select></label><span className="timezone-label">日付: {readerSettings?.display_timezone === 'Asia/Tokyo' ? '日本時間（JST）' : 'UTC'}</span><button type="button" className="read-all" disabled={bulkBusy || loading || !total} onClick={() => void markAllRead()}>全て既読</button></div><details className="advanced-filters"><summary>詳細検索</summary><form onSubmit={e => void applyAdvanced(e)}><div className="advanced-row"><label>検索対象<select aria-label="検索対象" value={draftScope.scope} onChange={e => setDraftScope({ ...draftScope, scope: e.target.value as AppliedFilters['scope'] })}><option value="both">タイトルと本文</option><option value="title">タイトル</option><option value="body">本文</option></select></label><label>並び順<select aria-label="並び順" value={draftScope.order} onChange={e => setDraftScope({ ...draftScope, order: e.target.value as AppliedFilters['order'] })}><option value="new">新しい順</option><option value="old">古い順</option><option value="recent">最近見た順</option></select></label><label>公開日・開始<input aria-label="公開日・開始" type="date" value={draftScope.published_from} onChange={e => setDraftScope({ ...draftScope, published_from: e.target.value })} /></label><label>公開日・終了<input aria-label="公開日・終了" type="date" value={draftScope.published_to} onChange={e => setDraftScope({ ...draftScope, published_to: e.target.value })} /></label></div><div className="advanced-row"><label>いずれかを含む<input aria-label="いずれかを含む" value={draftScope.any} onChange={e => setDraftScope({ ...draftScope, any: e.target.value })} /></label><label>含まない<input aria-label="含まない" value={draftScope.exclude} onChange={e => setDraftScope({ ...draftScope, exclude: e.target.value })} /></label><details className="feed-picker"><summary>対象フィード</summary><div>{feeds.map(feed => <label key={feed.id}><input type="checkbox" checked={draftScope.feed_ids.split(',').filter(Boolean).includes(String(feed.id))} onChange={e => { const ids = new Set(draftScope.feed_ids.split(',').filter(Boolean)); e.target.checked ? ids.add(String(feed.id)) : ids.delete(String(feed.id)); setDraftScope({ ...draftScope, feed_ids: [...ids].join(',') }) }} />{feed.title || feed.url}</label>)}</div></details><button>条件を適用</button><button type="button" onClick={() => { const order = filter === 'recent' ? 'recent' : 'new'; setDraftScope({ scope: 'both', order, any: '', exclude: '', published_from: '', published_to: '', feed_ids: '' }); setApplied({ scope: 'both', order }) }}>詳細条件をクリア</button></div></form></details>
          {notice && <div className="success-message" role="status">{notice}</div>}
          {selectedIds.length > 0 && <div className="bulk-toolbar"><span>{selectedIds.length} 件選択中</span><button disabled={bulkBusy} onClick={() => void bulkState({ read: true })}>選択を既読にする</button><button disabled={bulkBusy} onClick={() => void bulkState({ read: false })}>選択を未読にする</button><button disabled={bulkBusy} onClick={() => void bulkState({ starred: true })}>選択にスターを付ける</button><button disabled={bulkBusy} onClick={() => void bulkState({ starred: false })}>選択のスター解除</button><button onClick={() => setSelectedIds([])}>選択解除</button></div>}
          {error && <div className="error-message" role="alert">{error}<button onClick={() => void (readerSettings ? loadFirst() : loadReaderSettings())}>再試行</button></div>}
          <div className={`article-list layout-${layout}`} data-layout={layout} aria-label="記事一覧" aria-busy={loading}>
            {loading ? <div className="empty-state"><span className="spinner" />記事を読み込んでいます...</div> : items.length === 0 ? <div className="empty-state"><span className="empty-icon">▤</span><strong>{error ? '読み込みに失敗しました' : '記事がありません'}</strong><span>{error ? '接続を確認して、再試行してください。' : '条件に合う記事が見つかりませんでした。'}</span></div> : items.map(article => <article key={article.id} className={`article-card ${article.is_read ? 'read' : ''}`}>
              <input className="article-select" aria-label={`記事を選択: ${article.title}`} type="checkbox" checked={selectedIds.includes(article.id)} onChange={e => setSelectedIds(current => e.target.checked ? current.length < 100 ? [...current, article.id] : current : current.filter(id => id !== article.id))} />
              <button className="article-open" data-article-id={article.id} onClick={() => void openArticle(article)}><span className="unread-mark" aria-hidden="true" /><span className="card-copy"><span className="card-meta"><time dateTime={article.published}>{date(article.published)}</time><span className="meta-separator">·</span><span className="source">{(() => { try { return new URL(article.url).hostname.replace(/^www\./, '') } catch { return '記事' } })()}</span></span><strong>{article.title}</strong><span className="excerpt">{article.content.slice(0, 180)}{article.content.length > 180 ? '…' : ''}</span></span></button>
              <button className={`icon-button card-star ${article.is_starred ? 'is-starred' : ''}`} aria-label={article.is_starred ? 'スターを解除' : 'スターを付ける'} aria-pressed={article.is_starred} onClick={() => void updateState(article, { starred: !article.is_starred })}>{article.is_starred ? '★' : '☆'}</button>
            </article>)}
            <div ref={sentinel} className="load-sentinel" aria-hidden="true" />
            {moreLoading && <div className="more-loading"><span className="spinner" />さらに読み込み中...</div>}
            {!loading && items.length > 0 && items.length >= total && <div className="list-end">すべての記事を表示しました</div>}
          </div>
        </section>
        </>}
        {editingFeed && <div className="modal-backdrop" role="presentation" onMouseDown={e => { if (e.target === e.currentTarget) setEditingFeed(null) }}><form role="dialog" aria-modal="true" aria-labelledby="feed-edit-title" className="edit-dialog" onSubmit={async e => { e.preventDefault(); if (await mutate(`/api/feeds/${editingFeed.id}`, { method: 'PATCH', ...json({ title: editingFeed.title, folder_id: editingFeed.folder_id }) })) setEditingFeed(null) }}><h2 id="feed-edit-title">フィードを編集</h2>{libraryError && <p role="alert">{libraryError}</p>}<label>表示名<input value={editingFeed.title} onChange={e => setEditingFeed({ ...editingFeed, title: e.target.value })} /></label><label>フォルダー<select value={editingFeed.folder_id ?? ''} onChange={e => setEditingFeed({ ...editingFeed, folder_id: e.target.value ? Number(e.target.value) : null })}><option value="">フォルダーなし</option>{folders.map(f => <option key={f.id} value={f.id}>{f.name}</option>)}</select></label><div className="dialog-actions"><button type="button" onClick={() => setEditingFeed(null)}>キャンセル</button><button disabled={busy}>保存</button></div></form></div>}
        {editingFolder && <div className="modal-backdrop" role="presentation" onMouseDown={e => { if (e.target === e.currentTarget) setEditingFolder(null) }}><form role="dialog" aria-modal="true" aria-labelledby="folder-edit-title" className="edit-dialog" onSubmit={async e => { e.preventDefault(); if (await mutate(`/api/folders/${editingFolder.id}`, { method: 'PATCH', ...json({ name: editingFolder.name }) })) setEditingFolder(null) }}><h2 id="folder-edit-title">フォルダーを編集</h2>{libraryError && <p role="alert">{libraryError}</p>}<label>名前<input value={editingFolder.name} onChange={e => setEditingFolder({ ...editingFolder, name: e.target.value })} /></label><div className="dialog-actions"><button type="button" onClick={() => setEditingFolder(null)}>キャンセル</button><button disabled={busy}>保存</button><button type="button" className="danger" onClick={() => { if (window.confirm(`「${editingFolder.name}」を削除しますか？中のフィードはフォルダーなしになります。`)) void mutate(`/api/folders/${editingFolder.id}`, { method: 'DELETE' }).then(ok => ok && setEditingFolder(null)) }}>削除</button></div></form></div>}
      </main>
    </SidebarWidth>
  </div>
}

export default App
