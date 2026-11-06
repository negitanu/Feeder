#!/usr/bin/env python3
"""Validate resolved publication settings; never read the connector token."""
import json, os, re, subprocess
from urllib.parse import urlsplit

def require(condition,message):
    if not condition:raise ValueError(message)

def validate(config):
    services=config['services'];web=services['web'];tunnel=services['tunnel'];env=web['environment']
    require(not web.get('ports'), 'Web host ports must not be published')
    require(env['FEEDER_ACCESS_MODE']=='tunnel', 'Tunnel access mode is required')
    origin=env['FEEDER_PUBLIC_ORIGIN'];parsed=urlsplit(origin)
    require(parsed.scheme=='https' and parsed.hostname and not parsed.username and not parsed.password and not parsed.query and not parsed.fragment and parsed.path=='', 'Exact HTTPS origin required')
    require(env['FEEDER_ALLOWED_HOSTS']==parsed.netloc+',127.0.0.1:8080', 'Public Host must match configured origin')
    image=tunnel['image']
    require(re.fullmatch(r'cloudflare/cloudflared(?::\d{4}\.\d+\.\d+|@sha256:[0-9a-f]{64})',image), 'Pin the official connector image by version or digest')
    require(tunnel['read_only'] and tunnel['cap_drop']==['ALL'], 'Connector privileges must be reduced')
    require('--token-file' in tunnel['command'] and not tunnel.get('environment',{}).get('TUNNEL_TOKEN'), 'Token must come from secret file')
    require({v['target'] for v in tunnel.get('secrets',[])}=={'tunnel-token'}, 'Connector secret mount missing')
    require(not services['worker'].get('ports'), 'Worker must not publish ports')
    return True

if __name__=='__main__':
    command=['docker','compose','--env-file','/dev/null','-f','compose.yml','-f','compose.tunnel.yml','config','--format','json']
    config=json.loads(subprocess.check_output(command,text=True))
    validate(config)
    print('Tunnel configuration passed: unpublished origin, matching HTTPS Host, pinned connector and secret-file reference. Access policy/JWT enforcement still requires external acceptance.')
