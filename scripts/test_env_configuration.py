"""Compose configuration tests use only temporary, synthetic credentials."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent
OPEN = 'SYNTHETIC_OPENROUTER_CANARY_314159'
ORCA = 'SYNTHETIC_ORCAROUTER_CANARY_271828'
TOKEN = 'SYNTHETIC_TUNNEL_CANARY_161803'


class EnvConfigurationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.project = Path(self.temp.name)
        self.envfile = self.project / '.env'
        self.envfile.write_text('\n'.join([
            'FEEDER_PORT=18765',
            'FEEDER_PUBLIC_ORIGIN=https://reader.example.com',
            'FEEDER_PUBLIC_HOST=reader.example.com',
            'FEEDER_TUNNEL_IMAGE=cloudflare/cloudflared:2026.10.7',
            'FEEDER_TUNNEL_TOKEN_FILE=/tmp/synthetic-token-file',
            'OPENROUTER_API_KEY=' + OPEN,
            'ORCAROUTER_API_KEY=' + ORCA,
            'TUNNEL_TOKEN=' + TOKEN,
        ]) + '\n')
        self.env = {k: v for k, v in os.environ.items() if not (k.startswith(('FEEDER_', 'COMPOSE_')) or k in ('TUNNEL_TOKEN', 'OPENROUTER_API_KEY', 'ORCAROUTER_API_KEY'))}

    def config(self, override=None, explicit=True, env=None):
        command = ['docker', 'compose', '--project-directory', str(self.project)]
        if explicit:
            command += ['--env-file', str(self.envfile)]
        command += ['-f', str(ROOT / 'compose.yml')]
        if override:
            command += ['-f', str(ROOT / override)]
        command += ['config', '--format', 'json']
        result = subprocess.run(command, cwd=ROOT, env=env or self.env, capture_output=True, text=True)
        return result

    def test_dotenv_and_shell_precedence(self):
        for explicit in [False, True]:
            result = self.config(explicit=explicit)
            self.assertEqual(result.returncode, 0)
            config = json.loads(result.stdout)
            for service in ['web', 'worker']:
                env = config['services'][service]['environment']
                self.assertEqual(env['OPENROUTER_API_KEY'], OPEN)
                self.assertEqual(env['ORCAROUTER_API_KEY'], ORCA)
            self.assertEqual(config['services']['web']['ports'][0]['published'], '18765')
        result = self.config(env={**self.env, 'FEEDER_PORT': '18766', 'OPENROUTER_API_KEY': 'SYNTHETIC_SHELL_OVERRIDE'})
        config = json.loads(result.stdout)
        self.assertEqual(config['services']['web']['ports'][0]['published'], '18766')
        self.assertEqual(config['services']['worker']['environment']['OPENROUTER_API_KEY'], 'SYNTHETIC_SHELL_OVERRIDE')
        self.envfile.write_text(self.envfile.read_text().replace('TUNNEL_TOKEN=' + TOKEN, "TUNNEL_TOKEN='SYNTHETIC_$DOLLAR_VALUE'"))
        config = json.loads(self.config('compose.tunnel-env.yml').stdout)
        # `compose config` escapes a literal dollar as `$$` in its reusable output.
        self.assertEqual(config['services']['tunnel']['environment']['TUNNEL_TOKEN'], 'SYNTHETIC_$$DOLLAR_VALUE')

    def test_tunnel_modes_and_missing_token(self):
        for override, mode in [('compose.tunnel-env.yml', 'env'), ('compose.tunnel.yml', 'file')]:
            result = self.config(override)
            self.assertEqual(result.returncode, 0)
            config = json.loads(result.stdout)
            services = config['services']
            self.assertFalse(services['web'].get('ports'))
            self.assertFalse(services['worker'].get('ports'))
            tunnel = services['tunnel']
            self.assertTrue(tunnel['read_only'])
            self.assertNotIn(TOKEN, tunnel['command'])
            self.assertNotIn('OPENROUTER_API_KEY', tunnel.get('environment', {}))
            if mode == 'env':
                self.assertEqual(tunnel['environment']['TUNNEL_TOKEN'], TOKEN)
                self.assertFalse(tunnel.get('secrets'))
            else:
                self.assertNotIn('TUNNEL_TOKEN', tunnel.get('environment', {}))
                self.assertIn('--token-file', tunnel['command'])
            check = subprocess.run(['python3', str(ROOT / 'scripts/check-tunnel-config.py'), '--token-mode', mode, '--env-file', str(self.envfile)], cwd=ROOT, env=self.env, capture_output=True, text=True)
            self.assertEqual(check.returncode, 0)
            self.assertTrue(all(secret not in check.stdout + check.stderr for secret in (OPEN, ORCA, TOKEN)))
        self.envfile.write_text(self.envfile.read_text().replace('TUNNEL_TOKEN=' + TOKEN, 'TUNNEL_TOKEN='))
        missing = self.config('compose.tunnel-env.yml')
        self.assertNotEqual(missing.returncode, 0)
        self.assertTrue(all(secret not in missing.stderr for secret in (OPEN, ORCA, TOKEN)))

    def test_git_and_build_exclusions(self):
        for path in ['.env', '.env.production']:
            result = subprocess.run(['git', 'check-ignore', '--no-index', '-q', path], cwd=ROOT, env=self.env)
            self.assertEqual(result.returncode, 0)
        result = subprocess.run(['git', 'check-ignore', '--no-index', '-q', '.env.example'], cwd=ROOT, env=self.env)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((ROOT / '.dockerignore').read_text().splitlines()[0], '**')


if __name__ == '__main__':
    unittest.main()
