import MuteReasons from './MuteReasons'
import SmartFolderList from './SmartFolderList'
import DuplicatePanel from './DuplicatePanel'
import DigestPanel from './DigestPanel'
import SocialSourcesPanel from './SocialSourcesPanel'
import ReadLaterButton from './ReadLaterButton'
import ReadLaterPanel from './ReadLaterPanel'
import MuteRulesPanel from './MuteRulesPanel'
import CollectionHealthPanel from './CollectionHealthPanel'
import CatchupPanel from './CatchupPanel'
import ArticleRevisionsPanel from './ArticleRevisionsPanel'
import RelatedArticlesPanel from './RelatedArticlesPanel'
import { ArticleRowContent } from './ArticleRowContent'
import type { SearchSnapshot } from './SavedSearches'
import './integration.css'
import { parseReaderLocation, readerUrl } from './readerLocation'
import type { ReaderLocation } from './readerLocation'
import ArticleBulkActions from './ArticleBulkActions'
import DetailPresence from './DetailPresence'
import ArticleHero from './ArticleHero'
import { Search, List, Circle, Star, History, SunMoon, ArrowLeft, Inbox, Menu, X } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import SubscriptionManager from './SubscriptionManager'
import ReadabilityPanel from './ReadabilityPanel'
import SavedSearches from './SavedSearches'
import brandMark from './brand-mark.svg'
import { apiFetch } from './apiFetch'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import CollectionPanel from './CollectionPanel'
import ApiKeysPanel from './ApiKeysPanel'
import LibrarySidebar from './LibrarySidebar'
import type { Folder, Feed } from './LibrarySidebar'
import SidebarWidth from './SidebarWidth'
import ReaderSettingsPanel, { storedLayout } from './ReaderSettingsPanel'
import type { Layout, ReaderSettings } from './ReaderSettingsPanel'

type Article = { id: number; title: string; content: string; url: string; published: string; is_read: boolean; is_starred: boolean; last_visited?: string | null; image_url?: string | null; feed_id?: number | null; feed_title?: string | null; is_saved_for_later: boolean; revision?: number | null; content_fingerprint?: string | null; changed_at?: string | null; has_update?: boolean }
type ArticlePage = { items: Article[]; total: number; has_more: boolean; next_cursor: string | null }
export type Filter = 'all' | 'unread' | 'starred' | 'recent' | 'later'
export type AppliedFilters = { q?: string; unread?: boolean; starred?: boolean; history?: boolean; later?: boolean; include_muted?: boolean; order: 'new' | 'old' | 'recent'; scope: 'both' | 'title' | 'body'; any?: string; exclude?: string; published_from?: string; published_to?: string; feed_id?: number; folder_id?: number; feed_ids?: string }
type Theme = 'light' | 'dark' | 'system'
const PAGE_SIZE = 24
class RequestError extends Error {
  constructor(public status: number, message: string) { super(message) }
}
function savedFolders(): number[] { try { const value = JSON.parse(localStorage.getItem('feeder-collapsed-folders') || '[]'); return Array.isArray(value) ? value.filter(Number.isInteger) : [] } catch { return [] } }
function originalUrl(value: string): string | null { try { const url = new URL(value); return url.protocol === 'http:' || url.protocol === 'https:' ? url.href : null } catch { return null } }

async function request<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await apiFetch(url, init)
  if (!response.ok) throw new RequestError(response.status, response.status === 404 ? '記事が見つかりません。' : response.status === 400 || response.status === 422 ? '検索条件または選択した記事を確認してください。' : '通信に失敗しました。時間をおいて再度お試しください。')
  return response.json() as Promise<T>
}

function App() {
  const initialLocation = useRef(parseReaderLocation(new URL(window.location.href)))
  const hasInitialView = new URL(window.location.href).searchParams.has('view')
  const [filter, setFilter] = useState<Filter>(initialLocation.current.filter)
  const [readerSettings, setReaderSettings] = useState<ReaderSettings | null>(null)
  const [layout, setLayout] = useState<Layout>(storedLayout)
  const [sidebarWidth, setSidebarWidth] = useState(250)
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false)
  const [isMobile, setIsMobile] = useState(() => window.matchMedia('(max-width: 760px)').matches)
  const menuButtonRef = useRef<HTMLButtonElement>(null)
  const menuCloseRef = useRef<HTMLButtonElement>(null)
  const swipeStart = useRef<{ x: number; y: number; open: boolean } | null>(null)
  useEffect(() => {
    const media = window.matchMedia('(max-width: 760px)')
    const update = () => { setIsMobile(media.matches); if (!media.matches) setMobileSidebarOpen(false) }
    media.addEventListener('change', update)
    return () => media.removeEventListener('change', update)
  }, [])
  useEffect(() => {
    if (!isMobile || !mobileSidebarOpen) return
    // The drawer becomes focusable after its visibility transition starts.
    const focusTimer = window.setTimeout(() => {
      if (document.activeElement === menuButtonRef.current || document.activeElement === document.body) menuCloseRef.current?.focus()
    }, 220)
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !event.defaultPrevented) { setMobileSidebarOpen(false); menuButtonRef.current?.focus() }
    }
    document.addEventListener('keydown', onKeyDown)
    return () => { window.clearTimeout(focusTimer); document.removeEventListener('keydown', onKeyDown) }
  }, [isMobile, mobileSidebarOpen])
  useEffect(() => {
    if (!isMobile) return
    const start = (event: TouchEvent) => {
      swipeStart.current = null
      if (event.touches.length !== 1 || (event.target instanceof Element && event.target.closest('.order-handle'))) return
      const touch = event.touches[0]
      if (mobileSidebarOpen || touch.clientX <= 24) swipeStart.current = { x: touch.clientX, y: touch.clientY, open: mobileSidebarOpen }
    }
    const end = (event: TouchEvent) => {
      const startPoint = swipeStart.current; swipeStart.current = null
      if (!startPoint || event.changedTouches.length !== 1) return
      const touch = event.changedTouches[0], dx = touch.clientX - startPoint.x, dy = touch.clientY - startPoint.y
      if (Math.abs(dx) < 70 || Math.abs(dx) < Math.abs(dy) * 1.5) return
      if (!startPoint.open && dx > 0) setMobileSidebarOpen(true)
      if (startPoint.open && dx < 0) setMobileSidebarOpen(false)
    }
    document.addEventListener('touchstart', start, { passive: true })
    document.addEventListener('touchend', end, { passive: true })
    return () => { document.removeEventListener('touchstart', start); document.removeEventListener('touchend', end) }
  }, [isMobile, mobileSidebarOpen])
  const initialViewChosen = useRef(hasInitialView)
  const readerSettingsRef = useRef<ReaderSettings | null>(null)
  const readerSettingsRun = useRef(0)
  const [draftScope, setDraftScope] = useState(() => ({ scope: initialLocation.current.applied.scope, any: initialLocation.current.applied.any || '', exclude: initialLocation.current.applied.exclude || '', published_from: initialLocation.current.applied.published_from || '', published_to: initialLocation.current.applied.published_to || '', feed_ids: initialLocation.current.applied.feed_ids || '', order: initialLocation.current.applied.order }))
  const [applied, setApplied] = useState<AppliedFilters>(initialLocation.current.applied)
  const [selectedIds, setSelectedIds] = useState<number[]>([])
  const [page, setPage] = useState<'read' | 'settings'>('read')
  const [readerMode, setReaderMode] = useState<'articles' | 'catchup'>(initialLocation.current.mode ?? 'articles')
  const [folders, setFolders] = useState<Folder[]>([])
  const [feeds, setFeeds] = useState<Feed[]>([])
  const [source, setSource] = useState<{ kind: 'feed' | 'folder'; id: number } | null>(initialLocation.current.source)
  const [collapsed, setCollapsed] = useState<number[]>(savedFolders)
  const [busy, setBusy] = useState(false)
  const [refreshVersion, setRefreshVersion] = useState(0)
  const [laterRefresh, setLaterRefresh] = useState(0)
  const [sharedRefresh, setSharedRefresh] = useState(0)
  const [digestOpen, setDigestOpen] = useState(false)
  const [duplicatesOpen, setDuplicatesOpen] = useState(false)
  const [laterOpen, setLaterOpen] = useState(false)
  const [editingFeed, setEditingFeed] = useState<Feed | null>(null)
  const originalFeedUrl = useRef('')
  const [editingFolder, setEditingFolder] = useState<Folder | null>(null)
  const [query, setQuery] = useState(initialLocation.current.search)
  const [search, setSearch] = useState(initialLocation.current.search)
  const [items, setItems] = useState<Article[]>([])
  const [total, setTotal] = useState(0)
  const [globalUnread, setGlobalUnread] = useState<number | null>(null)
  const unreadGeneration = useRef(0)
  const [nextCursor, setNextCursor] = useState<string | null>(null)
  const [hasMore, setHasMore] = useState(false)
  const [selected, setSelected] = useState<Article | null>(null)
  const [detailSequence, setDetailSequence] = useState<Article[]>([])
  const [openingId, setOpeningId] = useState<number | null>(null)
  const [detailTop, setDetailTop] = useState(80)
  const [theme, setTheme] = useState<Theme>(() => { try { const t = localStorage.getItem('feeder-theme'); return t === 'light' || t === 'dark' ? t : 'system' } catch { return 'system' } })
  const [loading, setLoading] = useState(true)
  const [moreLoading, setMoreLoading] = useState(false)
  const [moreError, setMoreError] = useState<{ message: string; fresh: boolean } | null>(null)
  const [error, setError] = useState('')
  const [libraryError, setLibraryError] = useState('')
  const [libraryReady, setLibraryReady] = useState(false)
  const generation = useRef(0)
  const moreInFlight = useRef<number | null>(null)
  const libraryGeneration = useRef(0)
  const returnFocus = useRef<number | null>(null)
  const detailGeneration = useRef(0)
  const detailRef = useRef<HTMLElement>(null)
  const detailCloseRef = useRef<HTMLButtonElement>(null)
  const detailPreviousRef = useRef<HTMLButtonElement>(null)
  const detailNextRef = useRef<HTMLButtonElement>(null)
  const navigationFocus = useRef<'previous' | 'next' | null>(null)
  const mainRef = useRef<HTMLElement>(null)
  const visitRemovals = useRef(new Set<number>())
  const originalReadPending = useRef(new Set<number>())
  const sentinel = useRef<HTMLDivElement>(null)
  const params = useMemo(() => {
    const p = new URLSearchParams({ limit: String(PAGE_SIZE), pagination: 'cursor' })
    const f: AppliedFilters = { ...applied, q: search.trim() || undefined, unread: filter === 'unread' || undefined, starred: filter === 'starred' || undefined, history: filter === 'recent' || undefined, later: filter === 'later' || undefined, feed_id: source?.kind === 'feed' ? source.id : undefined, folder_id: source?.kind === 'folder' ? source.id : undefined }
    Object.entries(f).forEach(([key, value]) => { if (value !== undefined && value !== '') p.set(key, String(value)) })
    return p.toString()
  }, [applied, filter, search, source])
  const activeFilters = useMemo<AppliedFilters>(() => ({ ...applied, q: search.trim() || undefined, unread: filter === 'unread' || undefined, starred: filter === 'starred' || undefined, history: filter === 'recent' || undefined, later: filter === 'later' || undefined, feed_id: source?.kind === 'feed' ? source.id : undefined, folder_id: source?.kind === 'folder' ? source.id : undefined }), [applied, filter, search, source])
  const globalUnreadActive = filter === 'unread' && !applied.include_muted && !source && !search.trim() && !applied.any && !applied.exclude && !applied.published_from && !applied.published_to && !applied.feed_ids
  const filtersKey = JSON.stringify([activeFilters,readerSettings?.display_timezone])
  const laterScopeRef = useRef({ filter, items })
  laterScopeRef.current = { filter, items }
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
    try { const value = await request<ReaderSettings>('/api/reader/settings'); if (readerSettingsRun.current !== run) return false; applyReaderSettings(value); return true }
    catch { if (readerSettingsRun.current === run && !readerSettingsRef.current) { setError('表示設定を取得できませんでした。再試行してください。'); setLoading(false) } return false }
  }, [applyReaderSettings])
  useEffect(() => { void loadReaderSettings(); const refresh = () => { void loadReaderSettings() }; window.addEventListener('focus',refresh); return () => window.removeEventListener('focus',refresh) }, [loadReaderSettings])
  useEffect(() => { try { localStorage.setItem('feeder-layout',layout) } catch { /* Keep session presentation available. */ } }, [layout])
  function settingsSaved(value: ReaderSettings) { ++readerSettingsRun.current; applyReaderSettings(value) }

  const loadUnreadCount = useCallback(async () => {
    const run = ++unreadGeneration.current
    try { const value = await request<{ total: number }>('/api/articles?unread=true&limit=1'); if (run === unreadGeneration.current) setGlobalUnread(value.total) } catch { /* Keep the last known count. */ }
  }, [])
  useEffect(() => {
    void loadUnreadCount()
    const onFocus = () => { void loadUnreadCount() }
    window.addEventListener('focus', onFocus)
    const timer = window.setInterval(onFocus, 30000)
    return () => { window.removeEventListener('focus', onFocus); window.clearInterval(timer); ++unreadGeneration.current }
  }, [loadUnreadCount])

  const loadLibrary = useCallback(async () => {
    const current = ++libraryGeneration.current
    try { const result = await request<{ folders: Folder[]; feeds: Feed[] }>('/api/library'); if (current !== libraryGeneration.current) return; setFolders(result.folders); setFeeds(result.feeds); setLibraryReady(true); setSource(current => current && (current.kind === 'feed' ? !result.feeds.some(f => f.id === current.id) : !result.folders.some(f => f.id === current.id)) ? null : current); setLibraryError('') }
    catch (e) { if (current === libraryGeneration.current) setLibraryError((e as Error).message) }
  }, [])
  useEffect(() => { void loadLibrary() }, [loadLibrary])
  useEffect(() => { const refresh = () => { void loadLibrary() }; window.addEventListener('feeder-configuration-imported', refresh); return () => window.removeEventListener('feeder-configuration-imported', refresh) }, [loadLibrary])
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
    if (selected || openingId !== null || returnFocus.current === null) return
    const id = returnFocus.current
    returnFocus.current = null
    requestAnimationFrame(() => {
      document.querySelector<HTMLButtonElement>(`[data-article-id="${id}"]`)?.focus({ preventScroll: true })
    })
  }, [selected, openingId])

  function closeDetail(restoreFocus: boolean) {
    ++detailGeneration.current
    pendingArticle.current = null
    navigationFocus.current = null
    if (!restoreFocus) returnFocus.current = null
    setOpeningId(null); setSelected(null); setDetailSequence([])
  }
  useEffect(() => { if (openingId !== null) detailCloseRef.current?.focus({ preventScroll: true }) }, [openingId])
  useEffect(() => {
    if (openingId !== null || !selected || !navigationFocus.current) return
    const preferred = navigationFocus.current === 'next' ? detailNextRef.current : detailPreviousRef.current
    const fallback = navigationFocus.current === 'next' ? detailPreviousRef.current : detailNextRef.current
    navigationFocus.current = null
    const target = preferred && !preferred.disabled ? preferred : fallback && !fallback.disabled ? fallback : detailCloseRef.current
    target?.focus({ preventScroll: true })
  }, [openingId, selected])
  useEffect(() => {
    if ((!selected && openingId === null) || page !== 'read') return
    const dismiss = (event: PointerEvent) => {
      if (event.button !== 0 || !event.isPrimary || !(event.target instanceof Node) || detailRef.current?.contains(event.target)) return
      closeDetail(false)
    }
    const dismissKeyboardClick = (event: MouseEvent) => {
      if (event.button !== 0 || !(event.target instanceof Node) || detailRef.current?.contains(event.target)) return
      const current = detailGeneration.current
      window.setTimeout(() => { if (detailGeneration.current === current) closeDetail(false) }, 0)
    }
    const escape = (event: KeyboardEvent) => { if (event.key === 'Escape' && !editingFeed && !editingFolder) { event.preventDefault(); closeDetail(true) } }
    document.addEventListener('pointerdown', dismiss, true)
    document.addEventListener('click', dismissKeyboardClick, true)
    document.addEventListener('keydown', escape)
    return () => { document.removeEventListener('pointerdown', dismiss, true); document.removeEventListener('click', dismissKeyboardClick, true); document.removeEventListener('keydown', escape) }
  }, [selected, openingId, page, editingFeed, editingFolder])
  useEffect(() => {
    if ((!selected && openingId === null) || page !== 'read') return
    const place = () => {
      const headerBottom = document.querySelector('.topbar')?.getBoundingClientRect().bottom ?? 0
      const mainTop = mainRef.current?.getBoundingClientRect().top ?? 0
      setDetailTop(Math.min(window.innerHeight - 24, Math.max(0, headerBottom, mainTop) + 8))
    }
    place(); window.addEventListener('resize', place); window.addEventListener('scroll', place, { passive: true })
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(place)
    if (mainRef.current) observer?.observe(mainRef.current)
    const sidebar = document.querySelector('.sidebar'); if (sidebar) observer?.observe(sidebar)
    return () => { window.removeEventListener('resize', place); window.removeEventListener('scroll', place); observer?.disconnect() }
  }, [selected, openingId, page])

  type Snapshot = { location: ReaderLocation; items: Article[]; total: number; nextCursor: string | null; hasMore: boolean; selected: Article | null; sequence: Article[]; selectedIds: number[]; scroll: number }
  const snapshots = useRef(new Map<string, Snapshot>())
  useEffect(() => { snapshots.current.clear() }, [refreshVersion])
  const historyId = useRef<string>('')
  const restoring = useRef(false)
  const restoreList = useRef(false)
  const pendingArticle = useRef<number | null>(initialLocation.current.article)
  const pendingWaitForList = useRef(true)
  const historyReady = useRef(false)
  const currentLocation: ReaderLocation = { mode: readerMode, filter, search, source, applied, article: openingId ?? selected?.id ?? null }
  const snapshot: Snapshot = { location: currentLocation, items: items, total, nextCursor: items.length <= 500 ? nextCursor : null, hasMore: items.length <= 500 && hasMore, selected, sequence: detailSequence, selectedIds: selectedIds.slice(0, 100), scroll: window.scrollY }
  const latestSnapshot = useRef(snapshot)
  const saveSnapshot = () => {
    if (!historyId.current) return
    if (latestSnapshot.current.items.length > 500 || latestSnapshot.current.sequence.length > 500 || loading) { snapshots.current.delete(historyId.current); return }
    snapshots.current.delete(historyId.current); snapshots.current.set(historyId.current, { ...latestSnapshot.current, scroll: window.scrollY })
    while (snapshots.current.size > 30) snapshots.current.delete(snapshots.current.keys().next().value!)
  }
  useEffect(() => {
    const onPop = (event: PopStateEvent) => {
      saveSnapshot(); ++generation.current; ++detailGeneration.current; moreInFlight.current = null
      const location = parseReaderLocation(new URL(window.location.href))
      const key = typeof event.state?.readerHistoryId === 'string' ? event.state.readerHistoryId : ''
      const saved = snapshots.current.get(key)
      historyId.current = key || crypto.randomUUID(); restoring.current = true; initialViewChosen.current = true
      pendingArticle.current = saved?.selected ? null : location.article
      pendingWaitForList.current = !saved
      setFilter(location.filter); setSearch(location.search); setQuery(location.search); setSource(location.source); setApplied(location.applied)
      setReaderMode(location.mode ?? 'articles')
      setDraftScope({ scope: location.applied.scope, order: location.applied.order, any: location.applied.any || '', exclude: location.applied.exclude || '', published_from: location.applied.published_from || '', published_to: location.applied.published_to || '', feed_ids: location.applied.feed_ids || '' })
      setPage('read'); setOpeningId(null); setError(''); setMoreError(null); setMoreLoading(false)
      if (saved) {
        restoreList.current = JSON.stringify([filter, search, source, applied]) !== JSON.stringify([location.filter, location.search, location.source, location.applied])
        setItems(saved.items); setTotal(saved.total); setNextCursor(saved.nextCursor); setHasMore(saved.hasMore); setSelected(saved.selected); setDetailSequence(saved.sequence); setSelectedIds(saved.selectedIds); setLoading(false)
        requestAnimationFrame(() => requestAnimationFrame(() => { setSelectedIds(saved.selectedIds); window.scrollTo(0, saved.scroll) }))
      } else { setSelected(null); setDetailSequence([]); setRefreshVersion(v => v + 1) }
    }
    window.addEventListener('popstate', onPop); return () => window.removeEventListener('popstate', onPop)
  })
  useEffect(() => {
    if (!readerSettings || !libraryReady) return
    const url = readerUrl(currentLocation)
    if (!historyReady.current) {
      historyReady.current = true; historyId.current = crypto.randomUUID()
      window.history.replaceState({ ...window.history.state, readerHistoryId: historyId.current }, '', readerUrl({ ...currentLocation, article: pendingArticle.current }))
      latestSnapshot.current = snapshot; return
    }
    if (restoring.current) { restoring.current = false; window.history.replaceState({ ...window.history.state, readerHistoryId: historyId.current }, '', readerUrl({ ...currentLocation, article: pendingArticle.current ?? currentLocation.article })); latestSnapshot.current = snapshot; return }
    if (pendingArticle.current !== null) return
    if (url !== window.location.pathname + window.location.search + window.location.hash) {
      saveSnapshot(); historyId.current = crypto.randomUUID(); window.history.pushState({ ...window.history.state, readerHistoryId: historyId.current }, '', url)
    }
    latestSnapshot.current = snapshot
  }, [readerMode, filter, search, source, applied, selected, openingId, readerSettings, libraryReady, items, total, nextCursor, hasMore, detailSequence, selectedIds])
  useEffect(() => {
    if (!readerSettings || !libraryReady || loading || pendingWaitForList.current || pendingArticle.current === null) return
    const id = pendingArticle.current; pendingArticle.current = null
    const item = items.find(row => row.id === id) ?? { id, title: '', content: '', url: '', published: '', is_read: false, is_starred: false, is_saved_for_later: false }
    void openArticle(item)
  }, [readerSettings, libraryReady, loading, items])

  const loadFirst = useCallback(async () => {
    if (!readerSettings) return
    if (restoreList.current) { restoreList.current = false; return }
    const currentGeneration = ++generation.current
    moreInFlight.current = null
    ++detailGeneration.current; returnFocus.current = null; visitRemovals.current.clear()
    navigationFocus.current = null
    setLoading(true); setMoreLoading(false); setMoreError(null); setError(''); setOpeningId(null); setSelected(null); setDetailSequence([])
    try {
      const result = await request<ArticlePage>(`/api/articles?${params}`)
      if (currentGeneration !== generation.current) return
      setItems(result.items); setTotal(result.total); setHasMore(result.has_more); setNextCursor(result.next_cursor)
    } catch (e) { if (currentGeneration === generation.current) { setItems([]); setTotal(0); setHasMore(false); setNextCursor(null); setError((e as Error).message) } }
    finally { if (currentGeneration === generation.current) { pendingWaitForList.current = false; setLoading(false) } }
  }, [params, refreshVersion, readerSettings?.display_timezone, !!readerSettings])
  useEffect(() => { void loadFirst(); return () => { ++generation.current } }, [loadFirst])

  const loadMore = useCallback(async (retry = false) => {
    if (!readerSettings || moreInFlight.current !== null || loading || !hasMore || !nextCursor || (moreError && (!retry || moreError.fresh))) return
    const currentGeneration = generation.current
    moreInFlight.current = currentGeneration
    setMoreLoading(true)
    setMoreError(null)
    try {
      const p = new URLSearchParams(params); p.set('cursor', nextCursor)
      const result = await request<ArticlePage>(`/api/articles?${p}`)
      if (currentGeneration !== generation.current) return
      setItems(current => [...current, ...result.items.filter(a => !current.some(c => c.id === a.id))]); setTotal(result.total); setHasMore(result.has_more); setNextCursor(result.next_cursor)
    } catch (e) {
      if (currentGeneration === generation.current) setMoreError({ message: e instanceof RequestError && e.status === 409 ? '一覧の条件が変更されました。最新の一覧を読み直してください。' : '続きの記事を読み込めませんでした。', fresh: e instanceof RequestError && e.status === 409 })
    } finally {
      if (moreInFlight.current === currentGeneration) moreInFlight.current = null
      if (currentGeneration === generation.current) setMoreLoading(false)
    }
  }, [loading, params, readerSettings, moreError, hasMore, nextCursor])
  useEffect(() => {
    const node = sentinel.current
    if (readerMode !== 'articles' || !node || moreError || typeof IntersectionObserver === 'undefined') return
    const observer = new IntersectionObserver(entries => { if (entries[0]?.isIntersecting) void loadMore() }, { rootMargin: '500px' })
    observer.observe(node); return () => observer.disconnect()
  }, [loadMore, moreError, readerMode])
  async function refreshAfterContinuationConflict() {
    if (await loadReaderSettings()) setRefreshVersion(value => value + 1)
    else setMoreError({ message: '表示設定を取得できませんでした。接続を確認して一覧を読み直してください。', fresh: true })
  }

  function eligible(item: Article) { return !(filter === 'unread' && item.is_read) && !(filter === 'starred' && !item.is_starred) && !(filter === 'later' && !item.is_saved_for_later) }
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
      snapshots.current.clear()
      if (state.read !== undefined) void loadUnreadCount()
      if (filtersKeyRef.current !== key) return
      setSelected(current => current?.id === updated.id ? updated : current)
      setDetailSequence(current => current.map(item => item.id === updated.id ? updated : item))
      const was = eligible(article), now = eligible(updated)
      if (now) visitRemovals.current.delete(updated.id)
      setItems(current => now ? ordered(current.some(item => item.id === updated.id) ? current.map(item => item.id === updated.id ? updated : item) : [...current,updated]) : current.filter(item => item.id !== updated.id))
      if (was !== now) { if (now) setTotal(value => value + 1); else if (!visitRemovals.current.has(updated.id)) { visitRemovals.current.add(updated.id); setTotal(value => Math.max(0,value - 1)) } }
      if (!now) setSelectedIds(current => current.filter(id => id !== updated.id))
    } catch (e) { if (filtersKeyRef.current === key) setError((e as Error).message) }
  }
  async function markOriginalRead(article: Article) {
    if (article.is_read || originalReadPending.current.has(article.id)) return
    originalReadPending.current.add(article.id)
    try { await updateState(article, { read: true }) } finally { originalReadPending.current.delete(article.id) }
  }
  async function openArticle(article: Article, navigation?: 'previous' | 'next', sequence?: Article[]) {
    if (navigation) navigationFocus.current = navigation
    else { navigationFocus.current = null; setDetailSequence(sequence ?? (readerMode === 'catchup' ? [] : items)) }
    if (!navigation) returnFocus.current = article.id
    const detailRun = ++detailGeneration.current; const listRun = generation.current
    const key = filtersKey; setError(''); setOpeningId(article.id)
    try {
      const full = await request<Article>(`/api/articles/${article.id}/visit`, { method: 'POST' })
      snapshots.current.clear(); setLaterRefresh(value => value + 1)
      void loadUnreadCount()
      if (filtersKeyRef.current !== key || listRun !== generation.current) return
      setDetailSequence(current => current.map(item => item.id === full.id ? full : item))
      setItems(current => !eligible(full) ? current.filter(item => item.id !== full.id) : ordered(current.map(item => item.id === full.id ? full : item)))
      if (items.some(item => item.id === article.id) && eligible(article) && !eligible(full) && !visitRemovals.current.has(full.id)) { visitRemovals.current.add(full.id); setTotal(value => Math.max(0,value-1)); setSelectedIds(current => current.filter(id => id !== full.id)) }
      if (detailRun === detailGeneration.current) { returnFocus.current = full.id; setSelected(full); setOpeningId(null) }
    } catch (e) { if (detailRun === detailGeneration.current && filtersKeyRef.current === key && listRun === generation.current) { setOpeningId(null); setError((e as Error).message) } }
  }
  function submitSearch(e: React.FormEvent) { e.preventDefault(); closeDetail(false); setReaderMode('articles'); setPage('read'); setSearch(query) }
  async function applyAdvanced(e: React.FormEvent) { e.preventDefault(); closeDetail(false); setApplied({ include_muted: applied.include_muted, order: draftScope.order, scope: draftScope.scope, any: draftScope.any.trim() || undefined, exclude: draftScope.exclude.trim() || undefined, published_from: draftScope.published_from || undefined, published_to: draftScope.published_to || undefined, feed_ids: draftScope.feed_ids || undefined }) }
  const [notice, setNotice] = useState('')
  const [bulkBusy, setBulkBusy] = useState(false)
  async function handleBulkError(e: unknown, key: string) {
    if (filtersKeyRef.current !== key) return
    if (e instanceof RequestError && e.status === 409) {
      setNotice('表示タイムゾーンが変更されたため操作を中止しました。設定の再取得後、日付範囲を確認してからもう一度操作してください。')
      await loadReaderSettings()
    } else setError((e as Error).message)
  }
  async function markAllRead() {
    if (!readerSettings) return
    const key = filtersKey; setError(''); setBulkBusy(true)
    try {
      const result = await request<{ updated: number }>('/api/articles/read-all', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ filters: activeFilters, expected_timezone: readerSettings.display_timezone }) })
      snapshots.current.clear(); void loadUnreadCount()
      if (filtersKeyRef.current !== key) return
      if (filter === 'unread') { setItems([]); setTotal(0); setHasMore(false); setNextCursor(null) } else setItems(current => current.map(item => ({ ...item, is_read: true })))
      setSelectedIds([]); setNotice(`全て既読にしました（${result.updated} 件）`)
    } catch (e) { await handleBulkError(e, key) } finally { setBulkBusy(false) }
  }
  async function bulkState(state: { read?: boolean; starred?: boolean }) {
    if (!selectedIds.length || !readerSettings) return false
    const key = filtersKey; const ids = new Set(selectedIds); setError(''); setBulkBusy(true)
    try {
      const result = await request<{ updated: number }>('/api/articles/bulk-state', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ ids: selectedIds, ...state, filters: activeFilters, expected_timezone: readerSettings.display_timezone }) })
      snapshots.current.clear()
      if (state.read !== undefined) void loadUnreadCount()
      if (filtersKeyRef.current !== key) return false
      const leavesScope = (filter === 'unread' && state.read === true) || (filter === 'starred' && state.starred === false)
      setItems(current => current.flatMap(item => !ids.has(item.id) ? [item] : leavesScope ? [] : [{ ...item, ...(state.read !== undefined ? { is_read: state.read } : {}), ...(state.starred !== undefined ? { is_starred: state.starred } : {}) }]))
      if (leavesScope) { setTotal(value => Math.max(0,value-result.updated)); setSelectedIds([]) }
      setNotice(`${result.updated} 件を更新しました`); return true
    } catch (e) { await handleBulkError(e, key); return false } finally { setBulkBusy(false) }
  }
  async function mutate(url: string, init: RequestInit, reload = true) {
    setBusy(true); setLibraryError('')
    try { const response = await apiFetch(url, init); if (!response.ok) throw new Error(response.status === 404 ? '見つかりません。' : response.status === 400 || response.status === 422 ? '入力内容を確認してください。URLは認証情報のない公開HTTP(S)を指定してください。' : response.status === 409 ? 'URLが登録済みか、フィードを更新中です。内容を確認し、しばらくしてから再試行してください。' : '操作に失敗しました。時間をおいて再度お試しください。'); if (reload) { await loadLibrary(); void loadUnreadCount(); setRefreshVersion(value => value + 1) } return true }
    catch (e) { setLibraryError((e as Error).message); return false }
    finally { setBusy(false) }
  }
  const json = (body: unknown): RequestInit => ({ headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) })
  function selectSource(value: { kind: 'feed' | 'folder'; id: number } | null) { setMobileSidebarOpen(false); initialViewChosen.current = true; closeDetail(false); setReaderMode('articles'); setSource(value); setPage('read') }
  function detailTimestamp(value: string) {
    const normalized = /^\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}(:\d{2}(\.\d+)?)?$/.test(value) ? value.replace(' ', 'T') + 'Z' : value
    const instant = new Date(normalized)
    if (Number.isNaN(instant.getTime())) return '公開日時不明'
    const parts = new Intl.DateTimeFormat('en-CA', { timeZone: 'Asia/Tokyo', year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit', hourCycle: 'h23' }).formatToParts(instant)
    const part = (name: string) => parts.find(item => item.type === name)?.value ?? ''
    return `${part('year')}-${part('month')}-${part('day')} ${part('hour')}:${part('minute')}:${part('second')} JST`
  }
  function applySavedSearch(value: SearchSnapshot): string | null {
            if (value.source && !(value.source.kind === 'feed' ? feeds : folders).some(item => item.id === value.source!.id)) return '保存した情報源が見つかりません。検索を変更せずに中止しました。'
            const ids = value.applied.feed_ids?.split(',').filter(Boolean).map(Number) ?? []
            if (ids.some(id => !feeds.some(feed => feed.id === id))) return '保存した検索の対象フィードが見つかりません。'
            initialViewChosen.current = true; closeDetail(false); setReaderMode('articles'); setSelectedIds([]); setFilter(value.filter); setSource(value.source); setQuery(value.query); setSearch(value.query); setApplied(value.applied)
            setDraftScope({ scope: value.applied.scope, order: value.applied.order, any: value.applied.any ?? '', exclude: value.applied.exclude ?? '', published_from: value.applied.published_from ?? '', published_to: value.applied.published_to ?? '', feed_ids: value.applied.feed_ids ?? '' })
            setRefreshVersion(current => current + 1); return null
            }
  function openById(id: number, sequenceIds?: number[]) {
    const itemById = (articleId: number): Article => items.find(item => item.id === articleId) ?? { id: articleId, title: '', content: '', url: '', published: '', is_read: false, is_starred: false, is_saved_for_later: false }
    void openArticle(itemById(id), undefined, sequenceIds?.map(itemById))
  }
  async function markDuplicatesRead(ids: number[]) {
    await Promise.all(ids.map(id => request<Article>(`/api/articles/${id}/state`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ read: true }) })))
    snapshots.current.clear(); void loadUnreadCount(); setRefreshVersion(value => value + 1)
  }
  function updateLater(id: number, saved: boolean) {
    snapshots.current.clear(); setLaterRefresh(value => value + 1)
    const update = (item: Article) => item.id === id ? { ...item, is_saved_for_later: saved } : item
    setSelected(current => current ? update(current) : current)
    setDetailSequence(current => current.map(update))
    const inLaterList = laterScopeRef.current.filter === 'later' && laterScopeRef.current.items.some(item => item.id === id && item.is_saved_for_later)
    setItems(current => current.map(update).filter(item => laterScopeRef.current.filter !== 'later' || item.is_saved_for_later))
    if (inLaterList && !saved && !visitRemovals.current.has(id)) { visitRemovals.current.add(id); setTotal(value => Math.max(0, value - 1)); setSelectedIds(current => current.filter(value => value !== id)) }
    if (saved) visitRemovals.current.delete(id)
  }
  const detailIndex = selected ? detailSequence.findIndex(item => item.id === selected.id) : -1

  return <div className="app-shell"><a className="skip-navigation" href="#page-content">本文へ移動</a>
    <header className="topbar">
      <button ref={menuButtonRef} type="button" className="mobile-menu-button" aria-label={mobileSidebarOpen ? "メニューを閉じる" : "メニューを開く"} aria-controls="reader-sidebar" aria-expanded={mobileSidebarOpen} onClick={() => setMobileSidebarOpen(value => !value)}>{mobileSidebarOpen ? <X aria-hidden="true" /> : <Menu aria-hidden="true" />}</button>
      <a className="brand" href="#" onClick={e => { e.preventDefault(); closeDetail(false); setReaderMode('articles'); setPage('read'); setSource(null) }} aria-label="Feeder ホーム"><img className="brand-mark" src={brandMark} alt="" /><span>feeder</span></a>
      <nav className="global-nav" aria-label="メインナビゲーション"><button className={page === 'read' ? 'active' : ''} onClick={() => { closeDetail(false); setPage('read') }}>読む</button><button className={page === 'settings' ? 'active' : ''} onClick={() => { closeDetail(false); setPage('settings') }}>設定</button></nav>
      <form className="search" onSubmit={submitSearch} role="search"><Search className="ui-icon" aria-hidden="true" /><input aria-label="記事を検索" placeholder="記事を検索" value={query} onChange={e => setQuery(e.target.value)} /><button type="submit" aria-label="検索">検索</button></form>
      <label className="theme-control"><span className="sr-only">テーマ</span><select aria-label="テーマ" value={theme} onChange={e => setTheme(e.target.value as Theme)}><option value="system">システム</option><option value="light">ライト</option><option value="dark">ダーク</option></select><SunMoon className="ui-icon" aria-hidden="true" /></label>
    </header>
    {isMobile && mobileSidebarOpen && <button className="mobile-sidebar-backdrop" aria-label="メニューを閉じる" onClick={() => setMobileSidebarOpen(false)} />}
    <SidebarWidth onWidthChange={setSidebarWidth}>
      <aside id="reader-sidebar" className={`sidebar${mobileSidebarOpen ? ' mobile-sidebar-open' : ''}`} aria-label="記事フィルター" inert={isMobile && !mobileSidebarOpen}><div className="mobile-sidebar-heading"><span>ライブラリ</span><button ref={menuCloseRef} type="button" aria-label="メニューを閉じる" onClick={() => { setMobileSidebarOpen(false); menuButtonRef.current?.focus() }}><X aria-hidden="true" /></button></div><div className="side-heading">ライブラリ</div><nav className="filters">
        {([['all', 'すべて', List], ['unread', '未読', Circle], ['starred', 'スター付き', Star], ['recent', '最近読んだ記事', History], ['later', 'あとで読む', Inbox]] as [Filter, string, LucideIcon][]).map(([key, label, Icon]) => <button key={key} className={`filter ${(readerMode === 'articles' && (key === 'unread' ? globalUnreadActive : filter === key)) ? 'active' : ''}`} onClick={() => { setMobileSidebarOpen(false); initialViewChosen.current = true; setReaderMode('articles'); setFilter(key); setSource(null); closeDetail(false); setPage('read'); if (key === 'unread') { setSearch(''); setQuery(''); setApplied(current => ({ order: 'new', scope: 'both', include_muted: current.include_muted })); setDraftScope({ scope: 'both', order: 'new', any: '', exclude: '', published_from: '', published_to: '', feed_ids: '' }) } if (key === 'recent' || filter === 'recent') { const order = key === 'recent' ? 'recent' : 'new'; setApplied(current => ({ ...current, order })); setDraftScope(current => ({ ...current, order })) } }} aria-current={(readerMode === 'articles' && (key === 'unread' ? globalUnreadActive : filter === key)) ? 'page' : undefined}><span className="filter-icon" aria-hidden="true"><Icon className="ui-icon" /></span>{label}{key === 'all' && <span className="count">{total}</span>}{key === 'unread' && globalUnread !== null && <span className="count unread-count" aria-label={`未読 ${globalUnread} 件`}>{globalUnread}</span>}</button>)}
      </nav><LibrarySidebar folders={folders} feeds={feeds} source={source} collapsed={collapsed} onToggle={id => setCollapsed(current => current.includes(id) ? current.filter(value => value !== id) : [...current,id])} onSelect={selectSource} onEditFeed={feed => { originalFeedUrl.current = feed.url; setLibraryError(''); setEditingFeed(feed) }} onEditFolder={setEditingFolder} onMutate={mutate} onReload={loadLibrary} onMembershipChanged={ids => { if (source?.kind === 'folder' && ids.includes(source.id)) { setSelectedIds([]); setRefreshVersion(value => value + 1) } }} />{libraryError && <div className="sidebar-error" role="alert">{libraryError}</div>}</aside>
      <main id="page-content" tabIndex={-1} className="main-area" ref={mainRef}>
        {page === 'settings' && <><header className="settings-heading"><h1>設定</h1><p>購読・収集、表示、データの持ち出しとAPIアクセスを管理します。</p></header><nav className="settings-jumps" aria-label="設定の項目">{[['subscriptions','購読の追加'],['collection','自動収集'],['subscriptions-list','購読管理'],['imports','OPML'],['appearance','表示設定'],['export','設定の持ち出し'],['readability','読みやすさ'],['social','公開SNS'],['mute','ミュート'],['health','収集状態'],['keys','APIキー']].map(([id,label]) => <a key={id} href={`#settings-${id}`}>{label}</a>)}</nav><CollectionPanel folders={folders} onChanged={() => { void loadLibrary(); void loadUnreadCount(); setRefreshVersion(value => value + 1) }} onCreateFolder={name => mutate('/api/folders', { method: 'POST', ...json({ name }) })} /></>}
        {page === 'settings' && <><section className="collection-page collection-section" id="settings-subscriptions-list"><SubscriptionManager feeds={feeds} folders={folders} onChanged={() => { void loadLibrary(); void loadUnreadCount(); setRefreshVersion(value => value + 1) }} onEdit={feed => { originalFeedUrl.current = feed.url; setLibraryError(''); setEditingFeed(feed) }} /></section><ReaderSettingsPanel settings={readerSettings} presentation={{ appearance: theme, width: Math.round(sidebarWidth), layout, collapsed_folders: [...new Set(collapsed.filter(id => id > 0 && folders.some(folder => folder.id === id)))].slice(0,2000) }} onSaved={settingsSaved} onLayout={setLayout} onTheme={setTheme} onRetry={() => void loadReaderSettings()} /><div className="collection-page" id="settings-readability"><ReadabilityPanel /></div><SocialSourcesPanel onChanged={() => { void loadLibrary(); void loadUnreadCount(); setRefreshVersion(value => value + 1) }} /><div className="collection-page" id="settings-mute"><MuteRulesPanel feeds={feeds} onChanged={() => { void loadUnreadCount(); setRefreshVersion(value => value + 1) }} /></div><div className="collection-page" id="settings-health"><CollectionHealthPanel timezone={readerSettings?.display_timezone ?? 'UTC'} refreshKey={refreshVersion} onRetry={async id => { try { await request(`/api/feeds/${id}/refresh`, { method: 'POST' }); void loadLibrary(); void loadUnreadCount() } finally { setRefreshVersion(value => value + 1) } }} /></div><ApiKeysPanel />{libraryError && <div className="error-message" role="alert">{libraryError}</div>}</>}
        {page === 'read' && <>
        <div className="reader-mode-switch" role="group" aria-label="読むモード"><button type="button" aria-pressed={readerMode === 'articles'} onClick={() => { closeDetail(false); setReaderMode('articles') }}>記事一覧</button><button type="button" aria-pressed={readerMode === 'catchup'} onClick={() => { closeDetail(false); setReaderMode('catchup') }}>キャッチアップ</button></div>
        <DetailPresence open={selected !== null || openingId !== null}>{(selected || openingId !== null) && <section ref={detailRef} className="article-detail" style={{ top: detailTop }} aria-label="記事の詳細" aria-busy={openingId !== null} onKeyDown={event => {
          if (!selected || openingId !== null || event.defaultPrevented || event.nativeEvent.isComposing || event.repeat || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight')) return
          const target = event.target
          if (!(target instanceof Element) || target.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="slider"], [role="spinbutton"], [role="textbox"], [role="combobox"], [role="listbox"], [role="menu"], [role="radiogroup"], [role="tablist"], [role="tree"], [role="grid"]')) return
          const index = detailSequence.findIndex(item => item.id === selected.id)
          const direction = event.key === 'ArrowRight' ? 'next' : 'previous'
          const next = detailSequence[index + (direction === 'next' ? 1 : -1)]
          if (index < 0 || !next) return
          event.preventDefault(); void openArticle(next, direction)
        }}>
          <div className="detail-toolbar"><button ref={detailCloseRef} className="back-button" onClick={() => closeDetail(true)}><ArrowLeft className="ui-icon" aria-hidden="true" /> <span>一覧に戻る</span></button>{selected && openingId === null && <><button onClick={() => void updateState(selected, { read: !selected.is_read })}>{selected.is_read ? '未読にする' : '既読にする'}</button><ReadLaterButton key={selected.id} id={selected.id} saved={!!selected.is_saved_for_later} onChanged={saved => updateLater(selected.id, saved)} /><button className={`icon-button star-button ${selected.is_starred ? 'is-starred' : ''}`} aria-label={selected.is_starred ? 'スターを解除' : 'スターを付ける'} aria-pressed={selected.is_starred} onClick={() => void updateState(selected, { starred: !selected.is_starred })}><Star className="ui-icon" aria-hidden="true" fill={selected.is_starred ? 'currentColor' : 'none'} /></button></>}</div>
          {selected && openingId === null && applied.include_muted && <MuteReasons key={selected.id} id={selected.id} />}
          {selected && openingId === null && <p className="detail-keyboard-help">記事を移動 <kbd>←</kbd> <kbd>→</kbd></p>}
          {openingId !== null ? <div className="detail-loading" role="status">記事を読み込んでいます...</div> : selected && <>{error && <div className="error-message" role="alert">{error}</div>}<article className="article-body">{selected.image_url && <ArticleHero key={selected.id} id={selected.id} />}<h1>{originalUrl(selected.url) ? <a href={originalUrl(selected.url)!} target="_blank" rel="noopener noreferrer">{selected.title}</a> : selected.title}</h1><div className="article-meta"><time dateTime={selected.published}>{detailTimestamp(selected.published)}</time></div><hr className="detail-separator" /><div className="article-text">{selected.content}</div><hr className="detail-separator" /></article><ArticleRevisionsPanel key={`revisions-${selected.id}`} articleId={selected.id} timezone={readerSettings?.display_timezone ?? 'UTC'} /><RelatedArticlesPanel key={`related-${selected.id}`} articleId={selected.id} onOpen={openById} />{detailIndex >= 0 && <nav className="detail-navigation" aria-label="記事の移動"><button ref={detailPreviousRef} type="button" disabled={detailIndex === 0} onClick={() => void openArticle(detailSequence[detailIndex - 1], 'previous')}>前の記事</button><span aria-live="polite">{detailIndex + 1} / {detailSequence.length}</span><button ref={detailNextRef} type="button" disabled={detailIndex >= detailSequence.length - 1} onClick={() => void openArticle(detailSequence[detailIndex + 1], 'next')}>次の記事</button></nav>}</>}
        </section>}</DetailPresence>
        {readerMode === 'catchup' && error && !selected && <div className="error-message" role="alert">{error}</div>}
        {readerMode === 'catchup' ? <CatchupPanel feeds={feeds} folders={folders} timezone={readerSettings?.display_timezone ?? 'UTC'} refreshKey={refreshVersion} onOpen={openById} /> : <section>
          <section className="list-heading"><div><h1>{source ? (source.kind === 'feed' ? feeds.find(f => f.id === source.id)?.title : folders.find(f => f.id === source.id)?.name) || '記事' : filter === 'all' ? '記事' : filter === 'unread' ? '未読' : filter === 'recent' ? '最近読んだ記事' : filter === 'later' ? 'あとで読む' : 'スター付き'}</h1><p>{search ? `「${search}」の検索結果` : '購読中の情報源から集めた記事'}</p></div><span className="result-count">{total} 件</span></section>
          <SavedSearches snapshot={{ filter, query: search, source, applied: Object.fromEntries(Object.entries(applied).filter(([key, value]) => key !== 'include_muted' || value)) as AppliedFilters }} onApply={applySavedSearch} onShared={() => setSharedRefresh(value => value + 1)} />
          <SmartFolderList onApply={applySavedSearch} refreshKey={sharedRefresh} />
          <div className="reader-tools">
          <details className="integration-panel" onToggle={e => setDigestOpen(e.currentTarget.open)}><summary>新着ダイジェスト</summary>{digestOpen && <DigestPanel feeds={feeds} folders={folders} timezone={readerSettings?.display_timezone ?? 'UTC'} onOpen={openById} />}</details>
          <details className="integration-panel" onToggle={e => setDuplicatesOpen(e.currentTarget.open)}><summary>重複記事</summary>{duplicatesOpen && <DuplicatePanel onOpen={openById} onRead={markDuplicatesRead} />}</details>
          <details className="integration-panel" onToggle={e => setLaterOpen(e.currentTarget.open)}><summary>あとで読むの管理</summary>{laterOpen && <ReadLaterPanel onOpen={openById} refreshKey={laterRefresh} onChanged={updateLater} />}</details>
          </div><div className="reader-actions"><label className="integration-muted"><input type="checkbox" checked={!!applied.include_muted} onChange={e => { closeDetail(false); setApplied(current => ({ ...current, include_muted: e.target.checked || undefined })) }} />ミュートした記事も表示</label><label className="layout-control">表示形式<select aria-label="記事の表示形式" value={layout} onChange={e => setLayout(e.target.value as Layout)}><option value="standard">標準</option><option value="line">1行</option><option value="tiles">タイル</option></select></label><span className="timezone-label">日付: {readerSettings?.display_timezone === 'Asia/Tokyo' ? '日本時間（JST）' : 'UTC'}</span><button type="button" className="read-all" disabled={bulkBusy || loading || !total} onClick={() => void markAllRead()}>全て既読</button></div><details className="advanced-filters"><summary>詳細検索</summary><form onSubmit={e => void applyAdvanced(e)}><div className="advanced-row"><label>検索対象<select aria-label="検索対象" value={draftScope.scope} onChange={e => setDraftScope({ ...draftScope, scope: e.target.value as AppliedFilters['scope'] })}><option value="both">タイトルと本文</option><option value="title">タイトル</option><option value="body">本文</option></select></label><label>並び順<select aria-label="並び順" value={draftScope.order} onChange={e => setDraftScope({ ...draftScope, order: e.target.value as AppliedFilters['order'] })}><option value="new">新しい順</option><option value="old">古い順</option><option value="recent">最近見た順</option></select></label><label>公開日・開始<input aria-label="公開日・開始" type="date" value={draftScope.published_from} onChange={e => setDraftScope({ ...draftScope, published_from: e.target.value })} /></label><label>公開日・終了<input aria-label="公開日・終了" type="date" value={draftScope.published_to} onChange={e => setDraftScope({ ...draftScope, published_to: e.target.value })} /></label></div><div className="advanced-row"><label>いずれかを含む<input aria-label="いずれかを含む" value={draftScope.any} onChange={e => setDraftScope({ ...draftScope, any: e.target.value })} /></label><label>含まない<input aria-label="含まない" value={draftScope.exclude} onChange={e => setDraftScope({ ...draftScope, exclude: e.target.value })} /></label><details className="feed-picker"><summary>対象フィード</summary><div>{feeds.map(feed => <label key={feed.id}><input type="checkbox" checked={draftScope.feed_ids.split(',').filter(Boolean).includes(String(feed.id))} onChange={e => { const ids = new Set(draftScope.feed_ids.split(',').filter(Boolean)); e.target.checked ? ids.add(String(feed.id)) : ids.delete(String(feed.id)); setDraftScope({ ...draftScope, feed_ids: [...ids].join(',') }) }} />{feed.title || feed.url}</label>)}</div></details><button>条件を適用</button><button type="button" onClick={() => { const order = filter === 'recent' ? 'recent' : 'new'; setDraftScope({ scope: 'both', order, any: '', exclude: '', published_from: '', published_to: '', feed_ids: '' }); setApplied({ scope: 'both', order, include_muted: applied.include_muted }) }}>詳細条件をクリア</button></div></form></details>
          {notice && <div className="success-message" role="status">{notice}</div>}
          {selectedIds.length > 0 && <ArticleBulkActions count={selectedIds.length} busy={bulkBusy} onApply={async state => { if (!(await bulkState(state))) throw new Error('一括操作を完了できませんでした') }} onClear={() => setSelectedIds([])} onExport={() => {
            const chosen = items.filter(article => selectedIds.includes(article.id))
            const links = chosen.flatMap(article => { const url = originalUrl(article.url); return url ? [`${article.title.replace(/[\r\n]+/g, ' ')}\n${url}`] : [] })
            const skipped = selectedIds.length - links.length
            if (links.length) {
              const url = URL.createObjectURL(new Blob([links.join('\n\n') + '\n'], { type: 'text/plain;charset=utf-8' }))
              const anchor = document.createElement('a'); anchor.href = url; anchor.download = 'feeder-links.txt'; anchor.click(); window.setTimeout(() => URL.revokeObjectURL(url), 1000)
            }
            setNotice(`${links.length} 件のリンクを書き出しました${skipped ? `（${skipped} 件はURLが無効か一覧にないため除外）` : ''}`)
          }} />}
          {error && <div className="error-message" role="alert">{error}<button onClick={() => void (readerSettings ? loadFirst() : loadReaderSettings())}>再試行</button></div>}
          <div className={`article-list layout-${layout}`} data-layout={layout} aria-label="記事一覧" aria-busy={loading}>
            {loading ? <div className="empty-state"><span className="spinner" />記事を読み込んでいます...</div> : items.length === 0 ? <div className="empty-state"><span className="empty-icon"><Inbox className="ui-icon" aria-hidden="true" /></span><strong>{error ? '読み込みに失敗しました' : '記事がありません'}</strong><span>{error ? '接続を確認して、再試行してください。' : '条件に合う記事が見つかりませんでした。'}</span></div> : items.map(article => <article key={article.id} className={`article-card ${article.is_read ? 'read' : ''}`}>
              <input className="article-select" disabled={bulkBusy} aria-label={`記事を選択: ${article.title}`} type="checkbox" checked={selectedIds.includes(article.id)} onChange={e => setSelectedIds(current => e.target.checked ? current.length < 100 ? [...current, article.id] : current : current.filter(id => id !== article.id))} />
              <button className="article-open" data-article-id={article.id} onClick={() => void openArticle(article)} onAuxClick={e => { if (e.button !== 1) return; e.preventDefault(); const url = originalUrl(article.url); if (url) { window.open(url, '_blank', 'noopener,noreferrer'); void markOriginalRead(article) } }}><span className="unread-mark" aria-hidden="true" /><ArticleRowContent article={article} timezone={readerSettings?.display_timezone ?? 'UTC'} /></button>
              <ReadLaterButton compact id={article.id} saved={!!article.is_saved_for_later} onChanged={saved => updateLater(article.id, saved)} /><button className={`icon-button card-star ${article.is_starred ? 'is-starred' : ''}`} aria-label={article.is_starred ? 'スターを解除' : 'スターを付ける'} aria-pressed={article.is_starred} onClick={() => void updateState(article, { starred: !article.is_starred })}><Star className="ui-icon" aria-hidden="true" fill={article.is_starred ? 'currentColor' : 'none'} /></button>
            </article>)}
            <div ref={sentinel} className="load-sentinel" aria-hidden="true" />
            {moreLoading && <div className="more-loading"><span className="spinner" />さらに読み込み中...</div>}
            {moreError && <div className="error-message" role="alert">{moreError.message} {moreError.fresh ? <button type="button" onClick={() => void refreshAfterContinuationConflict()}>最新の一覧を読み直す</button> : <button type="button" onClick={() => void loadMore(true)}>続きの読込を再試行</button>}</div>}
            {!loading && !moreLoading && !moreError && hasMore && <div className="more-loading"><button type="button" onClick={() => void loadMore()}>さらに読み込む</button></div>}
            {!loading && items.length > 0 && !hasMore && <div className="list-end">すべての記事を表示しました</div>}
          </div>
        </section>}
        </>}
        {editingFeed && <div className="modal-backdrop" role="presentation" onMouseDown={e => { if (e.target === e.currentTarget) setEditingFeed(null) }}><form role="dialog" aria-modal="true" aria-labelledby="feed-edit-title" className="edit-dialog" onSubmit={async e => { e.preventDefault(); if (await mutate(`/api/feeds/${editingFeed.id}`, { method: 'PATCH', ...json({ title: editingFeed.title, folder_id: editingFeed.folder_id, poll_interval: editingFeed.poll_interval, ...(editingFeed.url !== originalFeedUrl.current ? { url: editingFeed.url } : {}) }) })) setEditingFeed(null) }}><h2 id="feed-edit-title">フィードを編集</h2>{libraryError && <p role="alert">{libraryError}</p>}<label>表示名<input value={editingFeed.title} onChange={e => setEditingFeed({ ...editingFeed, title: e.target.value })} /></label><label>フィードURL<input type="url" required value={editingFeed.url} onChange={e => setEditingFeed({ ...editingFeed, url: e.target.value })} /></label><label>個別の収集間隔（秒）<input aria-label="個別の収集間隔（秒）" type="number" min="60" max="86400" step="1" placeholder="空欄は全体設定" value={editingFeed.poll_interval ?? ''} onChange={e => setEditingFeed({ ...editingFeed, poll_interval: e.target.value === '' ? null : Number(e.target.value) })} /></label><label>フォルダー<select value={editingFeed.folder_id ?? ''} onChange={e => setEditingFeed({ ...editingFeed, folder_id: e.target.value ? Number(e.target.value) : null })}><option value="">フォルダーなし</option>{folders.map(f => <option key={f.id} value={f.id}>{f.name}</option>)}</select></label><div className="dialog-actions"><button type="button" onClick={() => setEditingFeed(null)}>キャンセル</button><button disabled={busy}>保存</button></div></form></div>}
        {editingFolder && <div className="modal-backdrop" role="presentation" onMouseDown={e => { if (e.target === e.currentTarget) setEditingFolder(null) }}><form role="dialog" aria-modal="true" aria-labelledby="folder-edit-title" className="edit-dialog" onSubmit={async e => { e.preventDefault(); if (await mutate(`/api/folders/${editingFolder.id}`, { method: 'PATCH', ...json({ name: editingFolder.name }) })) setEditingFolder(null) }}><h2 id="folder-edit-title">フォルダーを編集</h2>{libraryError && <p role="alert">{libraryError}</p>}<label>名前<input value={editingFolder.name} onChange={e => setEditingFolder({ ...editingFolder, name: e.target.value })} /></label><div className="dialog-actions"><button type="button" onClick={() => setEditingFolder(null)}>キャンセル</button><button disabled={busy}>保存</button><button type="button" className="danger" onClick={() => { if (window.confirm(`「${editingFolder.name}」を削除しますか？中のフィードはフォルダーなしになります。`)) void mutate(`/api/folders/${editingFolder.id}`, { method: 'DELETE' }).then(ok => ok && setEditingFolder(null)) }}>削除</button></div></form></div>}
      </main>
    </SidebarWidth>
  </div>
}

export default App
