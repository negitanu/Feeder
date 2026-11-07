import { useCallback, useEffect, useRef, useState } from 'react'
import { apiFetch } from './apiFetch'
import type { ArticleRowContentProps } from './ArticleRowContent'

export type ReaderTimezone = ArticleRowContentProps['timezone']
// The existing Reader article payload, with the shared revision metadata.
export type CatchupArticle = ArticleRowContentProps['article'] & {
  id: number
  feed_id?: number | null
  is_read: boolean
  is_starred: boolean
  revision: number | null
  content_fingerprint: string | null
  changed_at: string | null
  has_update: boolean
  is_muted?: boolean
}
export type CatchupReason = {
  code: 'interest_term' | 'priority_feed' | 'updated_since_visit' | 'recent_unread' | 'source_diversity'
  value?: string | number | null
}
export type CatchupItem = {
  article: CatchupArticle
  event_key: string
  fingerprint: string
  revision: number
  kind: 'new' | 'updated'
  reasons: CatchupReason[]
  related_count: number
}
export type CatchupResult = {
  generated_at: string
  window_start: string
  window_end: string
  limit: number
  policy_version: string | number
  scanned: number
  eligible_count: number
  truncated: boolean
  collection_incomplete: boolean
  items: CatchupItem[]
}
export type CatchupInterests = { revision: number; terms: string[]; priority_feed_ids: number[] }
export type AcknowledgementAction = 'confirmed' | 'skipped'
export type ArticleRevision = {
  revision: number
  fingerprint: string
  title: string
  content: string
  url: string
  published: string
  source_updated_at: string | null
  observed_at: string
  baseline: boolean
}
export type ArticleRevisions = {
  items: ArticleRevision[]
  has_more: boolean
  next_before_revision: number | null
  history_complete: boolean
  history_start: number | null
}
export type RelatedArticles = {
  article_id: number
  method_version: string | number
  articles: { article: CatchupArticle; reason: string }[]
  total: number
  truncated: boolean
  excluded: boolean
}

export class CatchupRequestError extends Error {
  constructor(public status: number, message: string) { super(message) }
}

async function checkedResponse(path: string, init: RequestInit = {}) {
  const response = await apiFetch(path, { cache: 'no-store', ...init })
  if (!response.ok) {
    const message = response.status === 409 ? '保存対象が変更されています。最新情報を読み込んでください。'
      : response.status === 404 ? '対象が見つかりません。再読み込みしてください。'
      : response.status === 400 || response.status === 422 ? '入力内容や選択した配信元を確認してください。'
      : response.status === 401 || response.status === 403 ? '認証情報を確認して、再読み込みしてください。'
      : '通信に失敗しました。時間をおいて再度お試しください。'
    throw new CatchupRequestError(response.status, message)
  }
  return response
}

export async function catchupWrite<T = unknown>(path: string, method: 'PUT' | 'DELETE', body: unknown): Promise<T | undefined> {
  const response = await checkedResponse(path, { method, headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) })
  return response.status === 204 ? undefined : response.json() as Promise<T>
}

export function catchupError(error: unknown): string {
  return error instanceof CatchupRequestError ? error.message : '接続を確認して、もう一度操作してください。'
}

/** Abort and generation checks both matter: cached/mocked responses can ignore abort. */
export function useCatchupQuery<T>(path: string, refreshKey = 0) {
  const [retry, setRetry] = useState(0)
  const generation = useRef(0)
  const controller = useRef<AbortController | null>(null)
  const key = JSON.stringify([path, refreshKey, retry])
  const [state, setState] = useState<{ key: string; path: string; data: T | null; loading: boolean; error: string }>({ key, path, data: null, loading: true, error: '' })
  const reload = useCallback(() => {
    generation.current += 1
    controller.current?.abort()
    setRetry(value => value + 1)
  }, [])
  useEffect(() => {
    const run = ++generation.current
    const abort = new AbortController()
    controller.current = abort
    setState(previous => ({ key, path, data: previous.path === path ? previous.data : null, loading: true, error: '' }))
    checkedResponse(path, { signal: abort.signal })
      .then(response => response.json() as Promise<T>)
      .then(data => { if (!abort.signal.aborted && run === generation.current) setState({ key, path, data, loading: false, error: '' }) })
      .catch(error => {
        if (!abort.signal.aborted && run === generation.current) setState(previous => ({ ...previous, loading: false, error: catchupError(error) }))
      })
    return () => { abort.abort() }
  }, [key, path])
  return {
    data: state.path === path ? state.data : null,
    loading: state.key !== key || state.loading,
    error: state.key === key ? state.error : '',
    reload,
  }
}

export function originalArticleUrl(value: string): string | null {
  try {
    const url = new URL(value)
    return url.protocol === 'https:' || url.protocol === 'http:' ? url.href : null
  } catch { return null }
}
