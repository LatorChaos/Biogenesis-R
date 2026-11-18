"""Verify boot reconciliation and failure boundaries using private files and harmless children"""
import ctypes
from contextlib import nullcontext
import hashlib
import json
import os
from pathlib import Path
import resource
import signal
import stat
import subprocess
import sys
import tempfile
import time
from types import SimpleNamespace
from unittest.mock import patch

os.setpriority(os.PRIO_PROCESS, 0, 19)
HERE = Path(__file__).resolve().parent
SOURCE = HERE / 'launch-living-display.py'
n = {'__name__': 'private_correction_fixture'}
exec(compile(SOURCE.read_bytes(), str(SOURCE), 'exec', dont_inherit=True, optimize=0), n)
BOOT_A = '11111111-1111-1111-1111-111111111111'
BOOT_B = '22222222-2222-2222-2222-222222222222'
checks = []

def check(value, label):
    if not value:
        raise AssertionError(label)
    checks.append(label)

def state(root, name):
    path = root / name
    path.mkdir(mode=0o700)
    (path / 'logs').mkdir(mode=0o700)
    return path

def private_file(path, body):
    path.write_bytes(body)
    path.chmod(0o600)

def low_child():
    resource.setrlimit(resource.RLIMIT_AS, (1024**3, 1024**3))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    os.sched_setaffinity(0, {0})
    os.sched_setscheduler(0, os.SCHED_IDLE, os.sched_param(0))
    os.setpriority(os.PRIO_PROCESS, 0, 19)

with tempfile.TemporaryDirectory(prefix='correction-fixture-', dir='/tmp') as temporary:
    root = Path(temporary)
    for total in [17, 128]:
        q = state(root, 'state-entry-admit-' + str(total))
        n['current_boot'] = lambda: BOOT_A
        n['reconcile_boot'](q)
        n['status_write'](q, {'status': 'FAILED', 'error': 'preserved primary error'})
        prior_status = (q / 'status.json').read_bytes()
        for index in range(total - len(list(q.iterdir()))):
            private_file(q / ('owned-extra-' + str(index)), b'owned diagnostic')
        n['current_boot'] = lambda: BOOT_B
        transition = n['reconcile_boot'](q)
        check(transition['transition'] == 'different_verified_boot'
              and (q / ('boot-' + BOOT_B + '.fence')).exists()
              and (q / 'status.json').read_bytes() == prior_status,
              'legitimate state entries permit boot reconciliation: ' + str(total))
    q = state(root, 'state-entry-refuse-129')
    n['current_boot'] = lambda: BOOT_A
    n['reconcile_boot'](q)
    n['status_write'](q, {'status': 'FAILED', 'error': 'preserved at refused bound'})
    private_file(q / 'status.pending.json', b'interrupted diagnostic')
    for index in range(129 - len(list(q.iterdir()))):
        private_file(q / ('owned-extra-' + str(index)), b'owned diagnostic')
    before_entries = {p.name: n['generation'](p.lstat()) for p in q.iterdir()}
    before_bytes = {p.name: p.read_bytes() for p in q.iterdir() if p.is_file()}
    before_directory = n['generation'](q.lstat())
    n['current_boot'] = lambda: BOOT_B
    try:
        n['reconcile_boot'](q)
    except ValueError as error:
        check('Unexpected launcher state entry count' in str(error), '129 state entries refuse at the scan bound')
    else:
        raise AssertionError('129 state entries were accepted')
    check(before_entries == {p.name: n['generation'](p.lstat()) for p in q.iterdir()}
          and before_bytes == {p.name: p.read_bytes() for p in q.iterdir() if p.is_file()}
          and before_directory == n['generation'](q.lstat()),
          '129-entry refusal preserves every entry, diagnostic, fence, and directory generation')

    s = state(root, 'boot-transition')
    n['current_boot'] = lambda: BOOT_A
    first = n['reconcile_boot'](s)
    check(first['transition'] == 'initial_fence' and (s / ('boot-' + BOOT_A + '.fence')).stat().st_size == 0, 'initial empty boot fence is published before diagnostics')
    n['status_write'](s, {'status': 'FAILED', 'error': 'prior primary error'})
    old_status = (s / 'status.json').read_bytes()
    partial = b'{"status":"RUNNING","birth":'
    private_file(s / 'status.pending.json', partial)
    n['current_boot'] = lambda: BOOT_B
    transition = n['reconcile_boot'](s)
    check(transition['pending_archived'] and not (s / 'status.pending.json').exists(), 'verified different boot reconciles interrupted diagnostic bytes')
    check((s / 'interrupted-status.json').read_bytes() == partial and (s / 'status.json').read_bytes() == old_status, 'reconciliation preserves interrupted bytes and prior primary status')
    check((s / ('boot-' + BOOT_B + '.fence')).exists() and not (s / ('boot-' + BOOT_A + '.fence')).exists(), 'fence advances to the verified new epoch')
    n['child_policy'] = low_child
    n['observer_policy'] = lambda: None
    log = n['Log'](s / 'logs')
    recovered = n['supervise']([sys.executable, '-I', '-B', '-c', "import time;print('resumed fixture');time.sleep(.1)"], dict(os.environ), root, log, s)
    log.close()
    check(recovered['status'] == 'EXITED' and recovered['cleanup']['group_absent'], 'new-boot diagnostic recovery permits one harmless gated child without manual cleanup')
    private_file(s / 'status.pending.json', b'partial same-boot diagnostic')
    before = {p.name: p.read_bytes() for p in s.iterdir() if p.is_file()}
    try:
        n['reconcile_boot'](s)
    except n['StartupBlocked']:
        checks.append('same-boot pending refuses a new launch')
    else:
        raise AssertionError('same-boot pending was accepted')
    check(before == {p.name: p.read_bytes() for p in s.iterdir() if p.is_file()}, 'same-boot refusal does not rewrite its primary diagnostics')

    # Simulate process interruption after archival, and after the atomic fence rename
    original_sync = n['sync_directory']
    for phase in ['after_archive', 'after_fence_rename']:
        q = state(root, phase)
        n['current_boot'] = lambda: BOOT_A
        n['reconcile_boot'](q)
        n['status_write'](q, {'status': 'FAILED', 'error': phase})
        private_file(q / 'status.pending.json', b'interrupted bytes')
        n['current_boot'] = lambda: BOOT_B
        def interrupted_sync(path):
            new_exists = (q / ('boot-' + BOOT_B + '.fence')).exists()
            if (phase == 'after_archive' and not new_exists) or (phase == 'after_fence_rename' and new_exists):
                raise OSError('injected interrupted directory synchronization')
            original_sync(path)
        n['sync_directory'] = interrupted_sync
        try:
            n['reconcile_boot'](q)
        except OSError:
            checks.append('retained simulated interruption ' + phase)
        else:
            raise AssertionError('interruption did not execute')
        n['sync_directory'] = original_sync
        n['reconcile_boot'](q)
        check((q / 'interrupted-status.json').read_bytes() == b'interrupted bytes'
              and (q / ('boot-' + BOOT_B + '.fence')).exists(), 'retry safely completes ' + phase)

    # Unsafe records never gain authority merely because a boot changed
    actual_lstat = Path.lstat
    for kind in ['symlink', 'hardlink', 'oversize', 'wrong_owner', 'wrong_mode', 'corrupt_status']:
        q = state(root, 'unsafe-' + kind)
        n['current_boot'] = lambda: BOOT_A
        n['reconcile_boot'](q)
        n['status_write'](q, {'status': 'FAILED', 'error': 'unchanged'})
        pending = q / 'status.pending.json'
        private_file(pending, b'partial')
        if kind == 'symlink':
            pending.unlink()
            target = q / 'owned-target'
            private_file(target, b'preserve')
            pending.symlink_to(target)
        elif kind == 'hardlink':
            os.link(pending, q / 'alias')
        elif kind == 'oversize':
            with pending.open('r+b') as stream:
                stream.truncate(32769)
        elif kind == 'wrong_mode':
            pending.chmod(0o644)
        elif kind == 'corrupt_status':
            private_file(q / 'status.json', b'{invalid current status')
        n['current_boot'] = lambda: BOOT_B
        def changed_owner(path, *args, **kwargs):
            info = actual_lstat(path, *args, **kwargs)
            if Path(path) == pending and kind == 'wrong_owner':
                attrs = {k: getattr(info, k) for k in ['st_mode','st_uid','st_gid','st_nlink','st_size','st_dev','st_ino','st_mtime_ns','st_ctime_ns']}
                attrs['st_uid'] = os.getuid() + 1
                return SimpleNamespace(**attrs)
            return info
        with patch.object(Path, 'lstat', changed_owner):
            try:
                n['reconcile_boot'](q)
            except (ValueError, OSError):
                checks.append('unsafe diagnostic refused: ' + kind)
            else:
                raise AssertionError('unsafe diagnostic accepted: ' + kind)
        check((q / ('boot-' + BOOT_A + '.fence')).exists() and os.path.lexists(pending), 'unsafe record retains the old fence: ' + kind)

    # Complete records must not become stale authority merely through malformed fields
    current_boot = n['current_boot']
    schema_states = [
        {'status':'RUNNING','boot_id':'invalid'},
        {'status':'RUNNING','boot_id':BOOT_A},
        {'status':'FAILED','boot_id':BOOT_A,'game':'invalid'},
        {'status':'FAILED','boot_id':BOOT_A,'game':{'status':'RUNNING','boot_id':BOOT_A}},
        {'status':'RUNNING','boot_id':BOOT_A,'birth':{'pid':True,'group':1,'starttime':1,'namespace':'pid:[1]'}},
        {'status':'RUNNING','boot_id':BOOT_A,'birth':{'pid':1,'group':1,'starttime':1,'namespace':'invalid'}},
        {'status':'FAILED','error':'missing boot'},
    ]
    for index, record in enumerate(schema_states):
        q = state(root, 'invalid-record-' + str(index))
        n['current_boot'] = lambda: BOOT_A
        n['reconcile_boot'](q)
        body = json.dumps(record).encode('ascii')
        private_file(q / 'status.json', body)
        try:
            n['reconcile_boot'](q)
        except n['StartupBlocked']:
            checks.append('malformed complete authority refused: ' + str(index))
        else:
            raise AssertionError('malformed complete authority accepted')
        check((q / 'status.json').read_bytes() == body, 'malformed complete authority is not silently repaired: ' + str(index))
    # Repeated real entry attempts must retain unknown child authority
    saved = {key:n[key] for key in ['STATE','PROFILE','notify_failure','supervise']}
    entered = []
    n['notify_failure'] = lambda *args: 'fixture_notification_disabled'
    n['supervise'] = lambda *args: entered.append(True)
    for index, record in enumerate(schema_states + [None, 'symlink', 'mode', 'owner', 'deep', 'parser_exhaustion']):
        q = state(root, 'repeated-entry-' + str(index))
        n['current_boot'] = lambda: BOOT_A
        n['reconcile_boot'](q)
        path = q / 'status.json'
        body = json.dumps(record).encode('ascii') if isinstance(record, dict) else b'{invalid complete record'
        if record == 'deep':
            body = b'[' * 2000 + b'0' + b']' * 2000
        private_file(path, body)
        if record == 'symlink':
            path.rename(q / 'protected-target')
            path.symlink_to(q / 'protected-target')
        elif record == 'mode':
            path.chmod(0o644)
        before = path.lstat()
        def complete_owner(item, *args, **kwargs):
            info = actual_lstat(item, *args, **kwargs)
            if Path(item) == path and record == 'owner':
                attrs = {k:getattr(info,k) for k in ['st_mode','st_uid','st_gid','st_nlink','st_size','st_dev','st_ino','st_mtime_ns','st_ctime_ns']}
                attrs['st_uid'] = os.getuid() + 1
                return SimpleNamespace(**attrs)
            return info
        n.update(STATE=q, PROFILE=q/'profile')
        parser = patch.object(json, 'loads', side_effect=RecursionError('fixture parser exhaustion')) if record == 'parser_exhaustion' else nullcontext()
        with patch.object(os, 'uname', return_value=SimpleNamespace(machine='aarch64')), patch.object(Path, 'lstat', complete_owner), parser:
            codes = [n['main'](), n['main']()]
        after = path.lstat()
        check(codes == [1,1] and not entered, 'two entry attempts refuse malformed authority without child: ' + str(index))
        check(path.read_bytes() == body and n['generation'](before) == n['generation'](after)
              and not (q/'previous-status.json').exists(), 'repeated refusal preserves complete diagnostic identity: ' + str(index))
    n.update(saved)
    n['current_boot'] = current_boot

    # A real fast leader leaves an unobserved same-group sleeper before the ancestry sample
    q = state(root, 'early-leader')
    n['current_boot'] = lambda: BOOT_A
    n['reconcile_boot'](q)
    child_file, published = root / 'early.py', root / 'early-child.json'
    child_file.write_text("import json,os,sys,time\nfrom pathlib import Path\nchild=os.fork()\nif child==0:\n Path(sys.argv[1]).write_text(json.dumps({'pid':os.getpid(),'group':os.getpgrp()}))\n while True:time.sleep(.1)\nos._exit(0)\n")
    def wait_for_unobserved_exit():
        deadline = time.monotonic() + 3
        while not published.exists() and time.monotonic() < deadline:
            time.sleep(.01)
        time.sleep(.05)
    n['observer_policy'] = wait_for_unobserved_exit
    libc = ctypes.CDLL(None)
    original_subreaper = ctypes.c_int()
    check(libc.prctl(37, ctypes.byref(original_subreaper), 0, 0, 0) == 0 and libc.prctl(36, 1, 0, 0, 0) == 0, 'early-exit fixture owns subreaper adoption')
    log = n['Log'](q / 'logs')
    early = n['supervise']([sys.executable, '-I', '-B', str(child_file), str(published)], dict(os.environ), root, log, q)
    log.close()
    check(early['status'] == 'FAILED' and 'cleanup_error' in early, 'actual unobserved survivor produces unresolved closure rather than a false PASS')
    prior = (q / 'status.json').read_bytes()
    try:
        n['existing_application'](q)
    except n['StartupBlocked']:
        checks.append('dead leader with unresolved same-boot group blocks duplicate engine')
    else:
        raise AssertionError('unresolved closure accepted')
    check((q / 'status.json').read_bytes() == prior, 'unresolved primary failure remains unchanged')
    descendant = json.loads(published.read_text())
    descendant_birth = n['birth'](descendant['pid'])
    check(descendant_birth['group'] == early['birth']['group'] and n['same_birth'](descendant_birth), 'fixture cleanup has exact named descendant authority')
    os.kill(descendant_birth['pid'], signal.SIGTERM)
    until = time.monotonic() + 3
    while time.monotonic() < until:
        n['reap_adopted']()
        if not n['same_birth'](descendant_birth):
            break
        time.sleep(.02)
    check(not n['same_birth'](descendant_birth), 'the deliberately unobserved fixture descendant is closed by its test owner')
    check(libc.prctl(36, original_subreaper.value, 0, 0, 0) == 0, 'early-exit fixture restores subreaper state')
    n['current_boot'] = lambda: BOOT_B
    n['reconcile_boot'](q)
    check(n['existing_application'](q) is False, 'verified next boot removes stale-PID authority after the old group was closed')

    # Kill a stalled native-boundary substitute using the unchanged bounded helper path
    n['observer_policy'] = lambda: None
    n['readiness_policy'] = low_child
    n['display_probe_source'] = lambda: "import signal,time;signal.signal(signal.SIGTERM,signal.SIG_IGN);time.sleep(5)"
    started = time.monotonic()
    probe = n['bounded_display_probe'](':0', .2, '/fixture-not-read')
    check(probe is False and time.monotonic() - started < 1.5, 'present-but-stalled native readiness helper has a hard timeout and finite escalation')

    # Model a root-owned system patch without modifying installed system files
    tool = root / 'system-tool'
    private_file(tool, b'patch-one')
    tool.chmod(0o755)
    ancestors = set(tool.parents)
    n['ROOT_UID'] = os.getuid()
    def trusted_fixture_ancestors(path, *args, **kwargs):
        info = actual_lstat(path, *args, **kwargs)
        if Path(path) in ancestors:
            return SimpleNamespace(st_uid=os.getuid(), st_mode=info.st_mode & ~0o022)
        return info
    with patch.object(Path, 'lstat', trusted_fixture_ancestors):
        first_patch = n['system_file'](tool)
        tool.write_bytes(b'patch-two-same-root-supported-path')
        second_patch = n['system_file'](tool)
        check(first_patch['sha256'] != second_patch['sha256'], 'same trusted system path accepts changed patch bytes and records both observations')
        tool.chmod(0o777)
        try:
            n['system_file'](tool)
        except ValueError:
            checks.append('writable system file remains refused after removing patch hash pin')
        else:
            raise AssertionError('writable system path accepted')
    n['ROOT_UID'] = 0

    # The primary error must survive optional notifier validation failure
    q = state(root, 'bad-notifier')
    profile = q / 'profile'
    profile.mkdir(mode=0o700)
    for part in ['home','config','cache','data','tmp']:
        (profile / part).mkdir(mode=0o700)
    n.update(STATE=q, PROFILE=profile, STOP=0)
    n['bundle'] = lambda install: (root, {})
    n['interpreter'] = lambda: None
    original_ready = n['display_ready']
    n['display_ready'] = lambda inherited: (_ for _ in ()).throw(ValueError('primary missing DISPLAY error'))
    n['notify_failure'] = lambda *args: (_ for _ in ()).throw(ValueError('secondary notifier ownership error'))
    fake_realm = SimpleNamespace(machine='aarch64')
    with patch.object(os, 'uname', return_value=fake_realm), patch.dict(os.environ, {'DISPLAY':':0'}, clear=True):
        code = n['main']()
    failed = json.loads((q / 'status.json').read_text())
    check(code == 1 and failed['status'] == 'FAILED' and 'primary missing DISPLAY error' in failed['error'], 'bad notifier cannot erase the original FAILED status')
    check('secondary notifier ownership error' in failed['notification'], 'notifier validation error is retained as secondary information')

result = {'status':'PASS_FOCUSED_AUTORUN_CORRECTION_FIXTURES_ONLY','source_sha256':hashlib.sha256(SOURCE.read_bytes()).hexdigest(),
          'checks':checks,'count':len(checks),'early_exit_result':early,'simulated_boot_ids':[BOOT_A,BOOT_B],
          'scope':'Boot epochs are controlled fixtures, not hardware reboot. Only harmless owned Python children ran. System-owner seams model trusted files; actual system files stayed unchanged. Parser exhaustion is injected because host Python3.14 parses the nested fixture iteratively.',
          'game_or_Pi_or_install':False}
print(json.dumps(result, sort_keys=True))
