let session: Promise<string> | null = null
async function csrfToken() {
  if (!session) session = fetch('/api/session', { credentials: 'same-origin', cache: 'no-store' }).then(async response => {
    if (!response.ok) throw new Error('接続を確認して、もう一度操作してください。')
    const data = await response.json() as { csrf_token: string }
    if (typeof data.csrf_token !== 'string' || !/^[a-f0-9]{64}$/.test(data.csrf_token)) throw new Error('確認情報を取得できませんでした。')
    return data.csrf_token
  }).catch(error => { session = null; throw error })
  return session
}
export async function apiFetch(path: string, init: RequestInit = {}) {
  const token = await csrfToken()
  const headers = new Headers(init.headers)
  if (!['GET','HEAD','OPTIONS'].includes((init.method ?? 'GET').toUpperCase())) headers.set('X-Feeder-CSRF', token)
  const response = await fetch(path, { ...init, headers, credentials: 'same-origin' })
  if (response.status === 403) session = null
  return response
}
