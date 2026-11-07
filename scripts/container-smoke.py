#!/usr/bin/env python3
"""Verify only a newly generated Linux Compose project and its disposable data."""
import io, json, os, re, socket, subprocess, tarfile, tempfile, urllib.request, urllib.error, uuid, http.cookiejar
from pathlib import Path
root=Path(__file__).resolve().parent.parent
project='feeder-smoke-'+uuid.uuid4().hex[:12]
with socket.socket() as sock:
    sock.bind(('127.0.0.1',0)); port=sock.getsockname()[1]
env=dict(os.environ,FEEDER_PORT=str(port),FEEDER_DEMO='1',FEEDER_ALLOWED_HOSTS=f'localhost:{port},127.0.0.1:{port},127.0.0.1:8080,web:8080',OPENROUTER_API_KEY='FEEDER_SYNTHETIC_OPENROUTER_CANARY',ORCAROUTER_API_KEY='FEEDER_SYNTHETIC_ORCAROUTER_CANARY')
# No real .env/credentials are loaded into this generated test deployment.
compose=['docker','compose','--env-file','/dev/null','-p',project]
restore_compose=['docker','compose','--env-file','/dev/null','-p',project+'-restore']
restore_created=False
def cmd(*args):
    return subprocess.check_output([*compose,*args],cwd=root,env=env,text=True).strip()
cookies=http.cookiejar.CookieJar()
opener=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(cookies))
csrf=''
def request(path,method='GET',data=None,headers=None):
    r=urllib.request.Request(f'http://127.0.0.1:{port}'+path,method=method,data=json.dumps(data).encode() if data is not None else None,headers={'Content-Type':'application/json','X-Feeder-CSRF':csrf,**(headers or {})})
    try:
        with opener.open(r,timeout=10) as response:return response.status,response.read()
    except urllib.error.HTTPError as e:return e.code,e.read()
try:
    print('Linux container verification project:',project,flush=True)
    cmd('up','--no-build','-d','--wait','--wait-timeout','90','web','worker')
    raw=cmd('ps','--format','json')
    containers=json.loads(raw) if raw.startswith('[') else [json.loads(line) for line in raw.splitlines()]
    assert {c['Service'] for c in containers}=={'web','worker'}, containers
    for c in containers:
        info=json.loads(subprocess.check_output(['docker','inspect',c['ID']],text=True))[0]
        assert info['Config']['User']=='10001:10001', info['Config']['User']
        assert info['HostConfig']['ReadonlyRootfs']
        assert 'ALL' in info['HostConfig']['CapDrop']
        assert any('no-new-privileges' in x for x in info['HostConfig']['SecurityOpt'])
    assert cmd('exec','-T','web','uname','-s')=='Linux'
    assert cmd('exec','-T','web','sh','-c','! command -v node && ! command -v cargo')==''
    assert request('/healthz')[0]==200
    assert request('/')[0]==200
    assert request('/api/articles',headers={'Host':'attacker.invalid'})[0]==403
    status,body=request('/api/articles');assert status==200
    articles=json.loads(body);assert articles['total']==36
    article_id=articles['items'][0]['id']
    csrf=json.loads(request('/api/session')[1])['csrf_token']
    # Synthetic credentials are provisioned only to app services, never exposed
    # through the UI, settings export, public API authorization, or logs.
    canaries=[env['OPENROUTER_API_KEY'],env['ORCAROUTER_API_KEY']]
    for c in containers:
        info=json.loads(subprocess.check_output(['docker','inspect',c['ID']],text=True))[0]
        container_env=info['Config']['Env']
        assert all(any(value.endswith('='+canary) for value in container_env) for canary in canaries)
    assert request('/api/v1/articles',headers={'Authorization':'Bearer '+canaries[0]})[0]==401
    export_status,export_body=request('/api/configuration/export','POST',{'presentation':{'appearance':'light','width':250,'layout':'standard','collapsed_folders':[]}})
    assert export_status==200
    ui=request('/')[1]
    script_path=re.search(rb'src="([^"]+\.js)"',ui).group(1).decode()
    js=request(script_path)[1]
    logs=cmd('logs','--no-color','web','worker')
    assert all(value.encode() not in export_body and value.encode() not in ui and value.encode() not in js and value not in logs for value in canaries)
    cmd('exec','-T','web','sh','-c','! grep -a -q -F "$OPENROUTER_API_KEY" /data/feeder.sqlite3* && ! grep -a -q -F "$ORCAROUTER_API_KEY" /data/feeder.sqlite3*')
    with tempfile.NamedTemporaryFile(mode='w',suffix='.env') as dotenv:
        dotenv.write("OPENROUTER_API_KEY='SYNTHETIC_$DOLLAR_VALUE'\n");dotenv.flush()
        isolated_env={k:v for k,v in env.items() if k!='OPENROUTER_API_KEY'}
        subprocess.run(['docker','compose','--env-file',dotenv.name,'-p',project,'run','--rm','--no-deps','-T','--entrypoint','sh','web','-c',"test \"$OPENROUTER_API_KEY\" = 'SYNTHETIC_$DOLLAR_VALUE'"],cwd=root,env=isolated_env,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    assert request(f'/api/articles/{article_id}/state','POST',{'read':True,'starred':True},headers={'Origin':'https://attacker.invalid'})[0]==403
    assert request(f'/api/articles/{article_id}/state','POST',{'read':True,'starred':True})[0]==200
    assert request('/api/collection/settings','PATCH',{'enabled':False,'interval_seconds':120})[0]==200
    assert cmd('exec','-T','web','sh','-c','test "$(id -u)" = 10001; test ! -w /app; test -w /data')==''
    if env.get('FEEDER_BROWSER_SMOKE')=='1':
        subprocess.run(['node','scripts/browser-smoke.mjs'],cwd=root,env=dict(env,FEEDER_URL=f'http://127.0.0.1:{port}'),check=True)
        subprocess.run(['node','scripts/detail-browser-smoke.mjs'],cwd=root,env=dict(env,FEEDER_URL=f'http://127.0.0.1:{port}'),check=True)
    if env.get('FEEDER_ONBOARDING_SMOKE')=='1':
        subprocess.run(['node','scripts/onboarding-browser-smoke.mjs'],cwd=root,env=dict(env,FEEDER_URL=f'http://127.0.0.1:{port}'),check=True)
    if env.get('FEEDER_PRODUCTIVITY_SMOKE')=='1':
        subprocess.run(['node','scripts/productivity-browser-smoke.mjs'],cwd=root,env=dict(env,FEEDER_URL=f'http://127.0.0.1:{port}'),check=True)
    if env.get('FEEDER_SIDEBAR_SMOKE')=='1':
        subprocess.run(['node','scripts/sidebar-browser-smoke.mjs'],cwd=root,env=dict(env,FEEDER_URL=f'http://127.0.0.1:{port}'),check=True)
    if env.get('FEEDER_SETTINGS_SMOKE')=='1':
        subprocess.run(['node','scripts/settings-browser-smoke.mjs'],cwd=root,env=dict(env,FEEDER_URL=f'http://127.0.0.1:{port}'),check=True)
    if env.get('FEEDER_ACCESS_SMOKE')=='1':
        subprocess.run(['node','scripts/access-browser-smoke.mjs'],cwd=root,env=dict(env,FEEDER_URL=f'http://127.0.0.1:{port}'),check=True)
    if env.get('FEEDER_SAVED_SEARCH_SMOKE')=='1':
        subprocess.run(['node','scripts/saved-search-browser-smoke.mjs'],cwd=root,env=dict(env,FEEDER_URL=f'http://127.0.0.1:{port}'),check=True)
    if env.get('FEEDER_READABILITY_SMOKE')=='1':
        subprocess.run(['node','scripts/readability-browser-smoke.mjs'],cwd=root,env=dict(env,FEEDER_URL=f'http://127.0.0.1:{port}'),check=True)
    if env.get('FEEDER_DETAIL_LAYOUT_SMOKE')=='1':
        subprocess.run(['node','scripts/detail-layout-browser-smoke.mjs'],cwd=root,env=dict(env,FEEDER_URL=f'http://127.0.0.1:{port}'),check=True)
        subprocess.run(['node','scripts/middle-read-browser-smoke.mjs'],cwd=root,env=dict(env,FEEDER_URL=f'http://127.0.0.1:{port}'),check=True)
    collection_settings_snapshot=json.loads(request('/api/collection/settings')[1])
    api_keys_snapshot=json.loads(request('/api/keys')[1])
    api_rate_snapshot=json.loads(request('/api/keys/settings')[1])
    reader_settings_snapshot=json.loads(request('/api/reader/settings')[1])
    library_snapshot=json.loads(request('/api/library')[1])
    history_snapshot=json.loads(request('/api/articles?history=true&order=recent&limit=100')[1])
    cmd('stop','--timeout','30')
    backup=subprocess.check_output([*compose,'run','--rm','--no-deps','-T','--entrypoint','tar','web','-C','/data','-czf','-','.'],cwd=root,env=env)
    with tarfile.open(fileobj=io.BytesIO(backup),mode='r:gz') as archive:
        assert './feeder.sqlite3' in archive.getnames()
    restore_created=True
    subprocess.run([*restore_compose,'run','--rm','--no-deps','-T','--entrypoint','tar','web','-C','/data','-xzf','-'],input=backup,cwd=root,env=env,check=True,stdout=subprocess.DEVNULL)
    subprocess.run([*restore_compose,'up','--no-build','-d','--wait','--wait-timeout','90','web','worker'],cwd=root,env=env,check=True,stdout=subprocess.DEVNULL)
    restored=json.loads(request(f'/api/articles/{article_id}')[1]);assert restored['is_read'] and restored['is_starred']
    assert json.loads(request('/api/articles?history=true&order=recent&limit=100')[1])==history_snapshot
    assert json.loads(request('/api/library')[1])==library_snapshot
    assert json.loads(request('/api/reader/settings')[1])==reader_settings_snapshot
    csrf=json.loads(request('/api/session')[1])['csrf_token']
    assert json.loads(request('/api/keys')[1])==api_keys_snapshot
    assert json.loads(request('/api/keys/settings')[1])==api_rate_snapshot
    assert json.loads(request('/api/collection/settings')[1])==collection_settings_snapshot
    subprocess.run([*restore_compose,'down','--volumes','--remove-orphans','--timeout','30'],cwd=root,env=env,check=True,stdout=subprocess.DEVNULL)
    restore_created=False
    cmd('up','--no-build','-d','--force-recreate','--wait','--wait-timeout','90','web','worker')
    status,body=request(f'/api/articles/{article_id}');assert status==200
    state=json.loads(body);assert state['is_read'] and state['is_starred']
    assert json.loads(request('/api/articles?history=true&order=recent&limit=100')[1])==history_snapshot
    assert json.loads(request('/api/library')[1])==library_snapshot
    assert json.loads(request('/api/reader/settings')[1])==reader_settings_snapshot
    csrf=json.loads(request('/api/session')[1])['csrf_token']
    assert json.loads(request('/api/keys')[1])==api_keys_snapshot
    assert json.loads(request('/api/keys/settings')[1])==api_rate_snapshot
    status,body=request('/api/collection/settings');assert status==200 and json.loads(body)==collection_settings_snapshot
    cmd('stop','--timeout','30','worker')
    worker_id=cmd('ps','--all','--quiet','worker')
    info=json.loads(subprocess.check_output(['docker','inspect',worker_id],text=True))[0]
    assert info['State']['ExitCode']==0,info['State']
    print('Passed: Linux image, health/UI/API, Host/Origin, UID/read-only/capabilities, persistence, isolated backup/restore, graceful worker stop.',flush=True)
finally:
    # This UUID project was created by this invocation and contains generated data only.
    if restore_created:
        subprocess.run([*restore_compose,'down','--volumes','--remove-orphans','--timeout','30'],cwd=root,env=env,check=False,stdout=subprocess.DEVNULL)
    cmd('down','--volumes','--remove-orphans','--timeout','30')
