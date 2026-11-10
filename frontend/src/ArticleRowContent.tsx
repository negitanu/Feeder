import './article-row.css'

type ArticleRowArticle = {
  title: string
  content: string
  url: string
  published: string
  feed_title?: string | null
}

export type ArticleRowContentProps = {
  article: ArticleRowArticle
  timezone: 'UTC' | 'Asia/Tokyo'
}

export function formatArticlePublished(published: string, timezone: ArticleRowContentProps['timezone']): string {
  const normalized = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d+)?$/.test(published)
    ? `${published.replace(' ', 'T')}Z` : published
  const date = new Date(normalized)
  if (!published || Number.isNaN(date.getTime())) return '公開日不明'

  const parts = new Intl.DateTimeFormat('en-CA', {
    timeZone: timezone,
    year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit',
    hourCycle: 'h23',
  }).formatToParts(date)
  const value = (type: Intl.DateTimeFormatPartTypes) => parts.find(part => part.type === type)?.value ?? ''
  const timestamp = `${value('year')}-${value('month')}-${value('day')} ${value('hour')}:${value('minute')}:${value('second')}`
  return `${timestamp} ${timezone === 'Asia/Tokyo' ? 'JST' : 'UTC'}`
}

function sourceLabel(article: ArticleRowArticle): string {
  const feedTitle = article.feed_title?.trim()
  if (feedTitle) return feedTitle
  try {
    const hostname = new URL(article.url).hostname.replace(/^www\./i, '')
    return hostname || '記事'
  } catch {
    return '記事'
  }
}

export function ArticleRowContent({ article, timezone }: ArticleRowContentProps) {
  const source = sourceLabel(article)
  const excerpt = article.content.slice(0, 180) + (article.content.length > 180 ? '…' : '')
  const timestamp = formatArticlePublished(article.published, timezone)
  const compactDate = timestamp === '公開日不明' ? timestamp : timestamp.slice(5, 10)

  return (
    <span className="article-row-content card-copy">
      <span className="source" title={source}>{source}</span>
      <strong title={article.title}>{article.title}</strong>
      <time className="card-meta" dateTime={article.published || undefined} aria-label={timestamp} title={timestamp}><span className="article-date-full" aria-hidden="true">{timestamp}</span><span className="article-date-compact" aria-hidden="true">{compactDate}</span></time>
      <span className="excerpt">{excerpt}</span>
    </span>
  )
}
