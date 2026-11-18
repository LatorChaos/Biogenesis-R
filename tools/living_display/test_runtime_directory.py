"""Verify the display runtime exception without opening displays or changing permissions"""
import importlib.util
from pathlib import Path
import stat
from types import SimpleNamespace
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).resolve().with_name('launch-living-display.py')
spec = importlib.util.spec_from_file_location('runtime_directory_launcher_fixture', SOURCE)
launcher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(launcher)


class RuntimeDirectoryTests(unittest.TestCase):
    def setUp(self):
        self.state = {
            'uid': 1000, 'gid': 1000, 'mode': stat.S_IFDIR | 0o770,
            'owner': 1000, 'group': 1000, 'members': [],
            'accounts': [(1000, 1000, 'test-user')], 'symlink': False,
        }
        state = self.state

        class RuntimePath(str):
            @property
            def parents(self):
                return [RuntimePath('/run/user'), RuntimePath('/run'), RuntimePath('/')]

            def lstat(self):
                if self == '/run/user/1000':
                    mode = stat.S_IFLNK | 0o770 if state['symlink'] else state['mode']
                    return SimpleNamespace(st_mode=mode, st_uid=state['owner'], st_gid=state['group'])
                return SimpleNamespace(st_mode=stat.S_IFDIR | 0o755, st_uid=0, st_gid=0)

        self.runtime = RuntimePath('/run/user/1000')
        replacements = [
            patch.object(launcher, 'Path', RuntimePath),
            patch.object(launcher.os, 'getuid', side_effect=lambda: state['uid']),
            patch.object(launcher.os, 'getgid', side_effect=lambda: state['gid']),
            patch.object(launcher.pwd, 'getpwuid', return_value=SimpleNamespace(pw_name='test-user', pw_gid=1000)),
            patch.object(launcher.grp, 'getgrgid', side_effect=lambda gid: SimpleNamespace(gr_gid=1000, gr_mem=state['members'])),
            patch.object(launcher.pwd, 'getpwall', side_effect=lambda: [
                SimpleNamespace(pw_uid=uid, pw_gid=gid, pw_name=name)
                for uid, gid, name in state['accounts']
            ]),
        ]
        for replacement in replacements:
            replacement.start()
            self.addCleanup(replacement.stop)

    def refuses(self, **changed):
        self.state.update(changed)
        with self.assertRaises(ValueError):
            launcher.display_runtime_directory(self.runtime)

    def test_generic_guard_still_refuses_group_write(self):
        with self.assertRaises(ValueError):
            launcher.owned_directory(self.runtime)

    def test_private_primary_group_0770(self):
        launcher.display_runtime_directory(self.runtime)

    def test_existing_0700(self):
        self.state['mode'] = stat.S_IFDIR | 0o700
        launcher.display_runtime_directory(self.runtime)

    def test_existing_0755(self):
        self.state['mode'] = stat.S_IFDIR | 0o755
        launcher.display_runtime_directory(self.runtime)

    def test_explicit_same_user_membership(self):
        self.state['members'] = ['test-user']
        launcher.display_runtime_directory(self.runtime)

    def test_other_supplementary_member(self):
        self.refuses(members=['other'])

    def test_other_primary_account(self):
        self.refuses(accounts=[(1000, 1000, 'test-user'), (1001, 1000, 'other')])

    def test_other_account_alias(self):
        self.refuses(accounts=[(1000, 1000, 'alias')])

    def test_wrong_process_uid(self):
        self.refuses(uid=0)

    def test_wrong_process_gid(self):
        self.refuses(gid=0)

    def test_wrong_runtime_uid(self):
        self.refuses(owner=0)

    def test_wrong_runtime_gid(self):
        self.refuses(group=0)

    def test_world_write(self):
        self.refuses(mode=stat.S_IFDIR | 0o772)

    def test_special_mode(self):
        self.refuses(mode=stat.S_IFDIR | 0o2770)

    def test_unknown_writable_mode(self):
        self.refuses(mode=stat.S_IFDIR | 0o775)

    def test_runtime_symlink(self):
        self.refuses(symlink=True)


if __name__ == '__main__':
    unittest.main()
