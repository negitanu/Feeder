import { useEffect, useState } from 'react'
import { apiFetch } from './apiFetch'

export default function MuteReasons({ id }: { id: number }) {
  const [reasons, setReasons] = useState<string[]>([])
  const [error, setError] = useState(false)
  const [retry, setRetry] = useState(0)
  useEffect(() => {
    const controller = new AbortController()
    setError(false); setReasons([])
    apiFetch(`/api/articles/${id}/mute-reasons`, { signal: controller.signal })
      .then(async response => { if (!response.ok) throw new Error(); return response.json() as Promise<{ reasons: { name: string }[] }> })
      .then(data => { if (!controller.signal.aborted) setReasons(data.reasons.map(reason => reason.name)) })
      .catch(() => { if (!controller.signal.aborted) setError(true) })
    return () => controller.abort()
  }, [id, retry])
  if (error) return <p className="detail-mute-reasons" role="status">ミュート条件を取得できませんでした。<button onClick={() => setRetry(value => value + 1)}>再試行</button></p>
  return reasons.length ? <p className="detail-mute-reasons">ミュート条件: {reasons.join('、')}</p> : null
}
