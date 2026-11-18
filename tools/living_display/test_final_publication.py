"""Preserve child authority after a failed final status replacement"""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import resource
import signal
import sys
import tempfile
import time
from types import SimpleNamespace
from unittest.mock import patch

os.setpriority(os.PRIO_PROCESS, 0, 19)
HERE = Path(__file__).resolve().parent
SOURCE = HERE / 'launch-living-display.py'
checks, results = [], []


def check(condition, name):
    if not condition:
        raise AssertionError(name)
    checks.append(name)


def low_child():
    resource.setrlimit(resource.RLIMIT_AS, (1024**3, 1024**3))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    os.sched_setaffinity(0, {0})
    os.sched_setscheduler(0, os.SCHED_IDLE, os.sched_param(0))
    os.setpriority(os.PRIO_PROCESS, 0, 19)


with tempfile.TemporaryDirectory(prefix='final-publication-fixture-', dir='/tmp') as temporary:
    root = Path(temporary)
    role, source = 'corrected', SOURCE
    n = {'__name__': 'private_publication_fixture'}
    exec(compile(source.read_bytes(), str(source), 'exec', dont_inherit=True, optimize=0), n)
    state = root / role
    state.mkdir(mode=0o700)
    (state / 'logs').mkdir(mode=0o700)
    for part in ['home', 'config', 'cache', 'data', 'tmp']:
        (state / 'profile' / part).mkdir(mode=0o700, parents=True)
    child_file, published = state / 'early.py', state / 'early-child.json'
    child_file.write_text("import json,os,sys,time\nfrom pathlib import Path\nchild=os.fork()\nif child==0:\n Path(sys.argv[1]).write_text(json.dumps({'pid':os.getpid(),'group':os.getpgrp()}))\n while True:time.sleep(.1)\nos._exit(0)\n")
    calls, injected = [], []
    real_supervise, real_sync = n['supervise'], n['sync_directory']
    def harmless_supervise(argv, env, cwd, log, own_state):
        calls.append(argv)
        return real_supervise([sys.executable, '-I', '-B', str(child_file), str(published)],
                              dict(os.environ), state, log, own_state)
    def after_replace(path):
        target = path / 'status.json'
        if target.exists():
            record = json.loads(target.read_bytes())
            if not injected and record.get('status') == 'FAILED' and record.get('cleanup_error'):
                injected.append(record)
                raise OSError('fixture directory fsync failed after final status replacement')
        return real_sync(path)
    def wait_for_unobserved_exit():
        end = time.monotonic() + 3
        while not published.exists() and time.monotonic() < end:
            time.sleep(.01)
        time.sleep(.05)
    n.update(STATE=state, PROFILE=state / 'profile', STOP=0, child_policy=low_child,
             observer_policy=wait_for_unobserved_exit, interpreter=lambda: None,
             bundle=lambda install: (state, {}), display_ready=lambda inherited: None,
             notify_failure=lambda *args: 'fixture_notification_disabled',
             supervise=harmless_supervise, sync_directory=after_replace)
    libc = ctypes.CDLL(None)
    original_subreaper = ctypes.c_int()
    check(libc.prctl(37, ctypes.byref(original_subreaper), 0, 0, 0) == 0
          and libc.prctl(36, 1, 0, 0, 0) == 0, role + ': fixture owns adopted child cleanup')
    child_birth = None
    try:
        with patch.object(os, 'uname', return_value=SimpleNamespace(machine='aarch64')):
            first = n['main']()
            check(first == 1 and len(calls) == 1 and len(injected) == 1,
                  role + ': final post-replace fsync fault occurred after one real harmless child')
            saved = (state / 'status.json').read_bytes()
            record = json.loads(saved)
            descendant = json.loads(published.read_text())
            child_birth = n['birth'](descendant['pid'])
            check(n['same_birth'](child_birth) and child_birth['group'] == injected[0]['birth']['group'],
                  role + ': survivor is the exact owned fixture descendant')
            game_argv = calls[0]
            check(game_argv[game_argv.index('--display-driver') + 1] == 'wayland',
                  role + ': real launcher selects native Wayland')
            attempts = []
            def no_second_child(*args):
                attempts.append(True)
                return {'status': 'EXITED', 'boot_id': n['current_boot']()}
            n['supervise'] = no_second_child
            before_generation = n['generation']((state / 'status.json').lstat())
            check(record['game']['birth'] == injected[0]['birth']
                  and record['game']['cleanup_error'] == injected[0]['cleanup_error']
                  and 'fixture directory fsync' in record['game']['status_publication_error'],
                  'corrected: primary failure retains birth, cleanup error, and publication error')
            codes = [n['main'](), n['main']()]
            check(codes == [1, 1] and attempts == [],
                  'corrected: repeated entry refuses unresolved group without another child')
            check((state / 'status.json').read_bytes() == saved
                  and n['generation']((state / 'status.json').lstat()) == before_generation,
                  'corrected: repeated refusal preserves authoritative diagnostic identity')
            results.append({'role': role, 'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
                            'injected_outcome': injected[0], 'retained_status': record})
    finally:
        if child_birth is None and published.exists():
            child_birth = n['birth'](json.loads(published.read_text())['pid'])
        if child_birth is not None and n['same_birth'](child_birth):
            os.kill(child_birth['pid'], signal.SIGTERM)
        end = time.monotonic() + 3
        while child_birth is not None and n['same_birth'](child_birth) and time.monotonic() < end:
            n['reap_adopted']()
            time.sleep(.02)
        check(child_birth is not None and not n['same_birth'](child_birth), role + ': named fixture descendant is absent')
        try:
            os.killpg(injected[0]['birth']['group'], 0)
        except ProcessLookupError:
            checks.append(role + ': owned fixture group is absent')
        else:
            raise AssertionError('Fixture group remains')
        check(libc.prctl(36, original_subreaper.value, 0, 0, 0) == 0, role + ': subreaper policy restored')
result = {'status': 'PASS_FINAL_PUBLICATION_AUTHORITY_REGRESSION_ONLY', 'count': len(checks),
          'checks': checks, 'roles': results, 'game_Pi_or_install': False,
          'scope': 'Real harmless children and real main entry; one injected directory fsync error after status replacement. No actual filesystem failure or power loss.'}
print(json.dumps(result, sort_keys=True))
