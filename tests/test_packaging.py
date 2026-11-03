"""Exercise package module hooks with fake kernels and command stubs."""
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class KernelModuleHooks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.commands = self.root / 'bin'
        self.commands.mkdir()
        self.log = self.root / 'commands.jsonl'
        self.state = self.root / 'registered'
        self.env = os.environ | {
            'PATH': f'{self.commands}:{os.environ["PATH"]}',
            'TEST_LOG': str(self.log), 'TEST_STATE': str(self.state),
            'TEST_KERNEL': '6.1-test',
        }
        stub = f'''#!{sys.executable}
import json, os, pathlib, sys
name = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
with open(os.environ['TEST_LOG'], 'a') as log:
    log.write(json.dumps([name, *args]) + '\\n')
state = pathlib.Path(os.environ['TEST_STATE'])
if name == 'uname':
    print(os.environ['TEST_KERNEL'])
if name == 'dkms':
    if args[0] in ('install', 'remove'):
        assert os.environ.get('DKMS_DIRECTIVE_POST_TRANSACTION') == 'post_transaction='
        assert args[-2:] == ['--directive', 'post_transaction=']
    if args[0] == 'status' and state.exists():
        print('jiaolong-mifs/0.1.0, 6.1-test, x86_64: installed')
    elif args[0] == 'add':
        state.touch()
    elif args[0] == 'remove':
        state.unlink(missing_ok=True)
    elif args[0] == 'install' and os.environ.get('TEST_BUILD_FAIL'):
        sys.exit(7)
'''
        for name in ('dkms', 'uname', 'modprobe', 'udevadm'):
            path = self.commands / name
            path.write_text(stub)
            path.chmod(0o755)
        headers = self.root / 'modules/6.1-test/build'
        headers.mkdir(parents=True)
        (headers / 'Makefile').touch()
        (self.root / 'wmi/B60BFB48-3E5B-49E4-A0E9-8CFFE1B3434B').mkdir(parents=True)
        self.control = socket.socket(socket.AF_UNIX)
        self.control.bind(str(self.root / 'control'))
        source = (ROOT / 'packaging/kernel-module').read_text()
        source = source.replace('/lib/modules/', f'{self.root}/modules/')
        source = source.replace('/sys/bus/wmi/devices/', f'{self.root}/wmi/')
        source = source.replace('/run/udev/control', str(self.root / 'control'))
        self.helper = self.root / 'kernel-module'
        self.helper.write_text(source)

    def tearDown(self):
        self.control.close()
        self.temporary.cleanup()

    def run_hook(self, action, check=True):
        return subprocess.run(['sh', str(self.helper), action, '0.1.0'], env=self.env,
                              capture_output=True, text=True, check=check)

    def calls(self):
        return [json.loads(line) for line in self.log.read_text().splitlines()]

    def test_install_builds_loads_and_is_idempotent(self):
        self.run_hook('install')
        self.run_hook('install')
        calls = self.calls()
        self.assertEqual(calls.count(['dkms', 'add', '-m', 'jiaolong-mifs', '-v', '0.1.0']), 1)
        self.assertIn(['dkms', 'install', '-m', 'jiaolong-mifs', '-v', '0.1.0', '-k', '6.1-test', '--directive', 'post_transaction='], calls)
        self.assertIn(['modprobe', 'jiaolong_mifs'], calls)
        self.assertIn(['udevadm', 'control', '--reload-rules'], calls)

    def test_remove_unregisters_module(self):
        self.run_hook('install')
        self.run_hook('remove')
        self.assertFalse(self.state.exists())
        self.assertIn(['dkms', 'remove', '-m', 'jiaolong-mifs', '-v', '0.1.0', '--all', '--directive', 'post_transaction='], self.calls())

    def test_failed_build_does_not_load_module(self):
        self.env['TEST_BUILD_FAIL'] = '1'
        self.assertEqual(self.run_hook('install', check=False).returncode, 7)
        self.assertNotIn(['modprobe', 'jiaolong_mifs'], self.calls())

    def test_missing_running_headers_builds_installed_kernel(self):
        self.env['TEST_KERNEL'] = '6.0-running'
        result = self.run_hook('install')
        self.assertIn('reboot', result.stdout)
        self.assertNotIn(['modprobe', 'jiaolong_mifs'], self.calls())


if __name__ == '__main__':
    unittest.main()
