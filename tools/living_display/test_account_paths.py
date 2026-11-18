"""Check account-derived paths without starting the installed display"""
import os
from pathlib import Path
import pwd
import shlex
from types import SimpleNamespace
from unittest import TestCase, main
from unittest.mock import patch

HERE = Path(__file__).resolve().parent


class AccountPaths(TestCase):
    def test_launcher_uses_account_home_not_environment(self):
        account = SimpleNamespace(pw_dir='/tmp/fixture home')
        with patch.object(pwd, 'getpwuid', return_value=account), patch.dict(os.environ, {'HOME': '/tmp/untrusted'}, clear=False):
            namespace = {'__name__': 'launcher_fixture'}
            source = HERE / 'launch-living-display.py'
            exec(compile(source.read_bytes(), str(source), 'exec'), namespace)
        self.assertEqual(namespace['INSTALL'], Path(account.pw_dir) / '.local/share/biogenesis-r')
        self.assertEqual(namespace['STATE'], Path(account.pw_dir) / '.local/state/biogenesis-r')

    def test_desktop_launcher_passes_one_account_path_without_a_shell(self):
        line = next(line for line in (HERE / 'biogenesis-r.desktop').read_text().splitlines() if line.startswith('Exec='))
        argv = shlex.split(line[5:])
        self.assertEqual(argv[:8], ['/usr/bin/setpriv', '--no-new-privs', '--inh-caps=-all', '--ambient-caps=-all', '--', '/usr/bin/python3', '-I', '-B'])
        self.assertEqual(argv[8], '-c')
        self.assertEqual(len(argv), 10)
        with patch.object(pwd, 'getpwuid', return_value=SimpleNamespace(pw_dir='/tmp/home $ % quote')), patch.object(os, 'execv') as execute:
            exec(compile(argv[9], '<desktop entry>', 'exec'), {})
        execute.assert_called_once_with('/usr/bin/python3', ['/usr/bin/python3', '-I', '-B', '/tmp/home $ % quote/.local/share/biogenesis-r/launch-living-display.py'])

    def test_relative_account_home_cannot_start_a_launcher(self):
        line = next(line for line in (HERE / 'biogenesis-r.desktop').read_text().splitlines() if line.startswith('Exec='))
        code = shlex.split(line[5:])[9]
        with patch.object(pwd, 'getpwuid', return_value=SimpleNamespace(pw_dir='relative')), patch.object(os, 'execv') as execute:
            with self.assertRaises(SystemExit):
                exec(compile(code, '<desktop entry>', 'exec'), {})
        execute.assert_not_called()

        with patch.object(pwd, 'getpwuid', return_value=SimpleNamespace(pw_dir='relative')):
            namespace = {'__name__': 'launcher_fixture'}
            source = HERE / 'launch-living-display.py'
            exec(compile(source.read_bytes(), str(source), 'exec'), namespace)
        with self.assertRaisesRegex(ValueError, 'Account home must be absolute'):
            namespace['main']()


if __name__ == '__main__':
    main()
