import type { AppliedFilters, Filter } from './App'

export type ReaderLocation = { mode?: 'articles' | 'catchup'; filter: Filter; search: string; source: { kind: 'feed' | 'folder'; id: number } | null; applied: AppliedFilters; article: number | null }
const keys = ['mode', 'view', 'search', 'feed', 'folder', 'article', 'order', 'scope', 'any', 'exclude', 'published_from', 'published_to', 'feed_ids', 'include_muted']
const id = (s: string | null) => s && /^[1-9]\d{0,14}$/.test(s) && Number.isSafeInteger(Number(s)) ? Number(s) : null
const word = (s: string | null) => s && s.length <= 500 ? s.trim() : ''
const date = (s: string | null) => s && /^\d{4}-\d{2}-\d{2}$/.test(s) && !Number.isNaN(Date.parse(s)) && new Date(s).toISOString().slice(0, 10) === s ? s : undefined
export function parseReaderLocation(url: URL): ReaderLocation {
  const p = url.searchParams
  const view = p.get('view'); const order = p.get('order'); const scope = p.get('scope')
  const feed = id(p.get('feed')); const folder = id(p.get('folder'))
  const ids = (p.get('feed_ids') || '').split(',').map(x => id(x)).filter((x): x is number => x !== null).slice(0, 100)
  return { mode: p.get('mode') === 'catchup' ? 'catchup' : 'articles', filter: view === 'unread' || view === 'starred' || view === 'recent' || view === 'later' ? view : 'all', search: word(p.get('search')), source: feed ? { kind: 'feed', id: feed } : folder ? { kind: 'folder', id: folder } : null, article: id(p.get('article')), applied: { ...(p.get('include_muted') === 'true' ? { include_muted: true } : {}), order: order === 'old' || order === 'recent' ? order : 'new', scope: scope === 'title' || scope === 'body' ? scope : 'both', any: word(p.get('any')) || undefined, exclude: word(p.get('exclude')) || undefined, published_from: date(p.get('published_from')), published_to: date(p.get('published_to')), feed_ids: ids.length ? [...new Set(ids)].join(',') : undefined } }
}
export function readerUrl(location: ReaderLocation, base = new URL(window.location.href)): string {
  const p = base.searchParams; keys.forEach(k => p.delete(k)); p.set('view', location.filter)
  if (location.mode === 'catchup') p.set('mode', 'catchup')
  if (location.search) p.set('search', location.search.trim().slice(0, 500))
  if (location.source) p.set(location.source.kind, String(location.source.id))
  if (location.article) p.set('article', String(location.article))
  if (location.applied.include_muted) p.set('include_muted', 'true')
  if (location.applied.order !== 'new') p.set('order', location.applied.order)
  if (location.applied.scope !== 'both') p.set('scope', location.applied.scope)
  for (const key of ['any', 'exclude', 'published_from', 'published_to', 'feed_ids'] as const) if (location.applied[key]) p.set(key, location.applied[key]!)
  return base.pathname + (p.size ? `?${p}` : '') + base.hash
}
