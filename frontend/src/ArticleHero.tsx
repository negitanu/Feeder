import { useState } from 'react'

export default function ArticleHero({ id }: { id: number }) {
  const [failed, setFailed] = useState(false)
  const [loaded, setLoaded] = useState(false)
  if (failed) return null
  return <img className="article-hero" src={`/api/articles/${id}/hero`} alt="" width={480} hidden={!loaded} referrerPolicy="no-referrer" decoding="async" onLoad={() => setLoaded(true)} onError={() => setFailed(true)} />
}
