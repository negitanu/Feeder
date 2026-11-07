// Sessions are disposable test data; never log the cookie or CSRF value.
export async function browserSession(page, origin) {
  // Reader fixtures do not contact article websites for optional artwork.
  // Image-specific tests override this route with generated raster fixtures.
  await page.context().route(/\/api\/articles\/\d+\/hero$/,route=>route.fulfill({status:404,body:''}));
  const response = await page.request.get(origin+'/api/session');
  if(response.status()!==200)throw new Error('Browser session bootstrap failed');
  const {csrf_token}=await response.json();
  await page.context().setExtraHTTPHeaders({'X-Feeder-CSRF':csrf_token});
}
