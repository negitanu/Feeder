#!/usr/bin/env python3
"""Validate resolved publication settings; never read the connector token."""
import argparse, json, re, subprocess, sys
from urllib.parse import urlsplit

def require(condition,message):
    if not condition:raise ValueError(message)

def validate(config):
    services=config['services'];web=services['web'];tunnel=services['tunnel'];env=web['environment']
    require(all(p.get('host_ip') in ('127.0.0.1', '::1') for p in web.get('ports', [])), 'Web host ports must be loopback only')
    require(env['FEEDER_ACCESS_MODE']=='tunnel', 'Tunnel access mode is required')
    origin=env['FEEDER_PUBLIC_ORIGIN'];parsed=urlsplit(origin)
    require(parsed.scheme=='https' and parsed.hostname and not parsed.username and not parsed.password and not parsed.query and not parsed.fragment and parsed.path=='', 'Exact HTTPS origin required')
    require(env['FEEDER_ALLOWED_HOSTS']==parsed.netloc+',127.0.0.1:8080', 'Public Host must match configured origin')
    image=tunnel['image']
    require(re.fullmatch(r'cloudflare/cloudflared(?::\d{4}\.\d+\.\d+|@sha256:[0-9a-f]{64})',image), 'Pin the official connector image by version or digest')
    require(tunnel['read_only'] and tunnel['cap_drop']==['ALL'], 'Connector privileges must be reduced')
    tunnel_env=tunnel.get('environment',{})
    require(bool(tunnel_env.get('TUNNEL_TOKEN')), 'Tunnel token is required')
    require('--token-file' not in tunnel['command'] and not tunnel.get('secrets'), 'Unexpected token-file configuration')
    require(tunnel_env['TUNNEL_TOKEN'] not in tunnel['command'], 'Tunnel token must not be a command argument')
    require(not services['worker'].get('ports'), 'Worker must not publish ports')
    return True

if __name__=='__main__':
    parser=argparse.ArgumentParser(description='Check tunnel settings without displaying token values.')
    parser.add_argument('--env-file', help='Optional Compose env file; otherwise use the project .env')
    args=parser.parse_args()
    command=['docker','compose']
    if args.env_file:
        command.extend(['--env-file',args.env_file])
    command.extend(['-f','compose.yml','config','--format','json'])
    result=subprocess.run(command,capture_output=True,text=True)
    if result.returncode:
        print('Compose configuration failed. Check required settings without sharing resolved output.',file=sys.stderr)
        sys.exit(1)
    try:
        config=json.loads(result.stdout)
        validate(config)
    except (ValueError, KeyError, TypeError):
        print('Tunnel configuration is invalid. Check public origin, token mode and image settings.',file=sys.stderr)
        sys.exit(1)
    print('Tunnel configuration passed: loopback-only origin, matching HTTPS Host and pinned connector. Access policy/JWT enforcement still requires external acceptance.')
