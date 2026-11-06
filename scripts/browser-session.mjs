// Sessions are disposable test data; never log the cookie or CSRF value.
export async function browserSession(page, origin) {
  const response = await page.request.get(origin+'/api/session');
  if(response.status()!==200)throw new Error('Browser session bootstrap failed');
  const {csrf_token}=await response.json();
  await page.context().setExtraHTTPHeaders({'X-Feeder-CSRF':csrf_token});
}
