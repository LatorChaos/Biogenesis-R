"""Exercise launcher boundaries with disposable harmless processes, never the product CLI"""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import resource
import signal
import subprocess
import sys
import tempfile
import time
from unittest.mock import patch

os.setpriority(os.PRIO_PROCESS, 0, 19)
HERE = Path(__file__).resolve().parent
SOURCE = HERE / 'launch-living-display.py'
module = {'__name__': 'private_launcher_test_import'}
exec(compile(SOURCE.read_bytes(), str(SOURCE), 'exec', dont_inherit=True, optimize=0), module)
checks = []

def check(value, label):
    if not value:
        raise AssertionError(label)
    checks.append(label)

def folder(path):
    path.mkdir(mode=0o700)
    return path

def fixture(parent):
    state = folder(parent / 'state')
    folder(state / 'logs')
    module['reconcile_boot'](state)
    profile = folder(state / 'profile')
    for name in ['home', 'config', 'cache', 'data', 'tmp']:
        folder(profile / name)
    install = folder(parent / 'install')
    app = folder(install / 'app')
    folder(app / 'bin')
    engine = bytearray(64)
    engine[:6] = b'\x7fELF\x02\x01'
    engine[18:20] = (183).to_bytes(2, 'little')
    files = {'Biogenesis-R.arm64': bytes(engine), 'Biogenesis-R.pck': b'private-not-a-game-pack',
             'bin/libbiogenesis_godot.arm64.so': b'private-not-a-library'}
    rows = {}
    for name, body in files.items():
        path = app / name
        path.write_bytes(body)
        mode = 0o755 if name.endswith('.arm64') else 0o644
        path.chmod(mode)
        rows[name] = {'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest(), 'mode': mode}
    manifest = {'schema': 1, 'ready': True, 'startup_interlock_qualified': True,
                'file_logging_disabled_qualified': True, 'files': rows}
    (install / 'bundle-manifest.json').write_text(json.dumps(manifest))
    return install, state, profile, manifest

def low_child():
    resource.setrlimit(resource.RLIMIT_AS, (1024**3, 1024**3))
    resource.setrlimit(resource.RLIMIT_FSIZE, (20 * 1024**2, 20 * 1024**2))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    os.sched_setaffinity(0, {0})
    os.sched_setscheduler(0, os.SCHED_IDLE, os.sched_param(0))
    os.setpriority(os.PRIO_PROCESS, 0, 19)

with tempfile.TemporaryDirectory(prefix='launcher-fixture-', dir='/tmp') as temporary:
    root = Path(temporary)
    install, state, profile, manifest = fixture(root)
    check(module['bundle'](install)[0] == install / 'app', 'fixed relative bundle validates admitted fixture bytes')
    changed = json.loads(json.dumps(manifest))
    changed['files']['../escape'] = changed['files']['Biogenesis-R.pck']
    (install / 'bundle-manifest.json').write_text(json.dumps(changed))
    try:
        module['bundle'](install)
    except ValueError:
        checks.append('traversal manifest refused')
    else:
        raise AssertionError('traversal accepted')
    (install / 'bundle-manifest.json').write_text(json.dumps(manifest))
    extra = install / 'app' / 'override.cfg'
    extra.write_text('[debug]\n')
    try:
        module['bundle'](install)
    except ValueError:
        checks.append('unmanifested loose settings refused')
    else:
        raise AssertionError('loose override accepted')
    extra.unlink()
    target = install / 'app' / 'Biogenesis-R.pck'
    target.write_bytes(b'changed')
    try:
        module['bundle'](install)
    except ValueError:
        checks.append('modified bundle payload refused')
    else:
        raise AssertionError('modified payload accepted')
    target.write_bytes(b'private-not-a-game-pack')

    descriptor = module['lock'](state)
    check(descriptor is not None, 'first launcher takes lock')
    code = "import sys;from pathlib import Path;n={'__name__':'fixture_import'};exec(compile(Path(sys.argv[1]).read_bytes(),sys.argv[1],'exec'),n);fd=n['lock'](Path(sys.argv[2]));assert fd is None"
    duplicate = subprocess.run([sys.executable, '-I', '-B', '-c', code, str(SOURCE), str(state)],
                               stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5)
    check(duplicate.returncode == 0 and not duplicate.stderr, 'independent duplicate launcher starts no child')
    os.close(descriptor)
    descriptor = module['lock'](state)
    check(descriptor is not None, 'lock released after owner closes')
    os.close(descriptor)
    lock_path = state / 'living-display.lock'
    lock_path.unlink()
    lock_path.symlink_to(state / 'not-created')
    try:
        module['lock'](state)
    except OSError:
        checks.append('symlink launch lock refused without target creation')
    else:
        raise AssertionError('symlink lock accepted')
    check(not (state / 'not-created').exists(), 'lock refusal leaves symlink target absent')
    lock_path.unlink()

    log = module['Log'](state / 'logs')
    body = b'x' * 65536
    for _ in range(101):
        log.write(body)
    log.close()
    first_sizes = {p.name: p.stat().st_size for p in (state / 'logs').iterdir()}
    check(set(first_sizes) == {'current.log', 'previous.log'} and max(first_sizes.values()) <= module['LOG_BYTES'], 'large output keeps only two bounded files')
    log = module['Log'](state / 'logs')
    for _ in range(80):
        log.write(body)
    log.close()
    check(sum(p.stat().st_size for p in (state / 'logs').iterdir()) <= 2 * module['LOG_BYTES'], 'log bound holds across launcher instances')

    inherited = {'DISPLAY': ':0', 'XAUTHORITY': '/private/authority', 'XDG_RUNTIME_DIR': '/run/user/1000', 'WAYLAND_DISPLAY': 'wayland-0'}
    env = module['environment'](profile, inherited)
    check(env['DISPLAY'] == ':0' and env['XAUTHORITY'] == '/private/authority' and env['HOME'] == str(profile / 'home'), 'desktop variables retained while profile remains isolated')
    begin = time.monotonic()
    try:
        module['display_ready']({}, 0.01)
    except ValueError:
        checks.append('missing display refused before native display calls')
    else:
        raise AssertionError('missing display accepted')
    check(time.monotonic() - begin < 1, 'missing-display readiness is finite')

    # Only internal supervision is called; the product CLI has no executable selection
    module['child_policy'] = low_child
    module['observer_policy'] = lambda: None
    module['STOP'] = 0
    counter = root / 'spawn-count.txt'
    fake = root / 'normal.py'
    fake.write_text("import os,time,pathlib,sys\np=pathlib.Path(sys.argv[1]);p.write_text(p.read_text()+'one\\n' if p.exists() else 'one\\n');os.write(1,b'normal stdout\\n');os.write(2,b'normal stderr\\n');time.sleep(.2)\n")
    log = module['Log'](state / 'logs')
    outcome = module['supervise']([sys.executable, '-I', '-B', str(fake), str(counter)], env, root, log, state)
    log.close()
    check(outcome['status'] == 'EXITED' and outcome['exit_code'] == 0 and outcome['cleanup']['group_absent'], 'normal child exits with owned group absent')
    time.sleep(0.1)
    check(counter.read_text() == 'one\n', 'normal exit does not restart child')

    # A descendant holding stdout must not keep a normally exited leader's pipe open forever
    forked = root / 'pipe-owner.py'
    forked.write_text("import os,time,signal\nchild=os.fork()\nif child==0:\n while True:time.sleep(.1)\ntime.sleep(.8)\n")
    log = module['Log'](state / 'logs')
    began = time.monotonic()
    descendant_outcome = module['supervise']([sys.executable, '-I', '-B', str(forked)], env, root, log, state)
    log.close()
    check(descendant_outcome['status'] == 'EXITED' and descendant_outcome['cleanup']['group_absent'], 'normal leader exit closes a known descendant holding its stdout pipe')
    check(time.monotonic() - began < 7, 'post-exit pipe drain and closure remain finite')

    # The fixture runner itself has a finite outer limit and remains low priority
    signals_root = folder(root / 'signals')
    state2 = folder(signals_root / 'state')
    logs2 = folder(state2 / 'logs')
    fake_signal = signals_root / 'child.py'
    births = signals_root / 'births.json'
    fake_signal.write_text("import json,os,signal,sys,time\nfrom pathlib import Path\nsignal.signal(signal.SIGTERM,signal.SIG_IGN)\nchild=os.fork()\nif child==0:\n while True:time.sleep(.1)\nPath(sys.argv[1]).write_text(json.dumps([os.getpid(),child]))\nwhile True:time.sleep(.1)\n")
    runner = signals_root / 'runner.py'
    runner.write_text("import json,os,pathlib,resource,signal,sys\nn={'__name__':'fixture_import'};exec(compile(pathlib.Path(sys.argv[1]).read_bytes(),sys.argv[1],'exec'),n)\nn['child_policy']=lambda:None;n['observer_policy']=lambda:None\ndef stop(number,frame):n['STOP']=number\nsignal.signal(signal.SIGTERM,stop)\nstate=pathlib.Path(sys.argv[2]);log=n['Log'](state/'logs')\nr=n['supervise']([sys.executable,'-I','-B',sys.argv[3],sys.argv[4]],dict(os.environ),state,log,state);log.close();raise SystemExit(0 if r['cleanup']['group_absent'] else 1)\n")
    watcher = subprocess.Popen([sys.executable, '-I', '-B', str(runner), str(SOURCE), str(state2), str(fake_signal), str(births)],
                               stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
    until = time.monotonic() + 5
    while not births.exists() and time.monotonic() < until:
        time.sleep(0.02)
    check(births.exists(), 'signal fixture published actual parent and child PIDs')
    time.sleep(0.7)
    watcher.send_signal(signal.SIGTERM)
    stdout, stderr = watcher.communicate(timeout=9)
    check(watcher.returncode == 0 and not stderr, 'signal cleanup returns within finite bound')
    signal_status = json.loads((state2 / 'status.json').read_text())
    check(signal_status['status'] == 'FAILED' and signal_status['cleanup']['group_absent'], 'cancelled run retains truthful failure and group closure')
    check({r['signal'] for r in signal_status['cleanup']['actions']} == {signal.SIGTERM, signal.SIGKILL}, 'unresponsive named group receives finite TERM then KILL')
    check(all(not Path('/proc/%d' % pid).exists() for pid in json.loads(births.read_text())), 'both actual signal fixture births are absent')

    # Abrupt launcher death must not permit a second engine while its recorded child survives
    orphan_root = folder(root / 'orphan')
    orphan_state = folder(orphan_root / 'state')
    folder(orphan_state / 'logs')
    orphan_child = orphan_root / 'child.py'
    orphan_born = orphan_root / 'born'
    orphan_child.write_text("import pathlib,sys,time\npathlib.Path(sys.argv[1]).write_text('running')\nwhile True:time.sleep(.1)\n")
    orphan_runner = orphan_root / 'runner.py'
    orphan_runner.write_text("import os,pathlib,sys\nn={'__name__':'fixture_import'};exec(compile(pathlib.Path(sys.argv[1]).read_bytes(),sys.argv[1],'exec'),n)\nn['child_policy']=lambda:None;n['observer_policy']=lambda:None\ns=pathlib.Path(sys.argv[2]);fd=n['lock'](s);log=n['Log'](s/'logs')\nn['supervise']([sys.executable,'-I','-B',sys.argv[3],sys.argv[4]],dict(os.environ),s,log,s)\n")
    libc = ctypes.CDLL(None)
    previous_subreaper = ctypes.c_int()
    check(libc.prctl(37, ctypes.byref(previous_subreaper), 0, 0, 0) == 0 and libc.prctl(36, 1, 0, 0, 0) == 0, 'orphan fixture has explicit temporary subreaper ownership')
    abandoned = subprocess.Popen([sys.executable, '-I', '-B', str(orphan_runner), str(SOURCE), str(orphan_state), str(orphan_child), str(orphan_born)],
                                 stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
    until = time.monotonic() + 5
    while not orphan_born.exists() and time.monotonic() < until:
        time.sleep(.02)
    check(orphan_born.exists(), 'child cannot publish running before the durable launch gate')
    record = json.loads((orphan_state / 'status.json').read_text())
    row = record['birth']
    abandoned.kill()
    abandoned.communicate(timeout=3)
    check(module['existing_application'](orphan_state), 'surviving same-boot child remains authoritative after launcher death')
    lock_fd = module['lock'](orphan_state)
    check(lock_fd is not None, 'parent death releases flock but not durable duplicate protection')
    os.close(lock_fd)
    module.update(STATE=orphan_state)
    fake_realm = type('Realm', (), {'machine': 'aarch64'})()
    before_record = (orphan_state / 'status.json').read_bytes()
    with patch.object(os, 'uname', return_value=fake_realm):
        duplicate_code = module['main']()
    check(duplicate_code == 0 and (orphan_state / 'status.json').read_bytes() == before_record and module['same_birth'](row), 'replacement launcher refuses a second child without changing the survivor')
    check(module['same_birth'](row) and row['pid'] == row['group'], 'fixture cleanup signals only its recorded child group')
    os.killpg(row['group'], signal.SIGTERM)
    until = time.monotonic() + 3
    while time.monotonic() < until:
        module['reap_adopted']()
        if not module['same_birth'](row):
            break
        time.sleep(.02)
    check(not module['same_birth'](row), 'orphan fixture group closes after its explicit test cleanup')
    check(libc.prctl(36, previous_subreaper.value, 0, 0, 0) == 0, 'fixture restores its original subreaper state')

    # Verify the notifier command and environment without starting a desktop target
    class Notice:
        pid = 999999
        def wait(self, timeout):
            check(timeout == 17, 'desktop notification has a finite wait')
            return 0
    notices = []
    original_system = module['system_file']
    original_birth = module['birth']
    module['ZENITY'] = str(SOURCE)
    module['system_file'] = lambda path, maximum: check(str(path) == str(SOURCE), 'notifier admission uses its fixed system path')
    module['birth'] = lambda pid: {'pid': pid, 'group': pid, 'starttime': 1, 'namespace': 'fixture'}
    with patch.object(subprocess, 'Popen', side_effect=lambda argv, **kwargs: notices.append((argv, kwargs)) or Notice()):
        notice = module['notify_failure']('Configuration failed', inherited, root / 'missing-profile')
    module['system_file'] = original_system
    module['birth'] = original_birth
    check(notice == 'exit_0' and notices[0][0][1:] == ['--error', '--no-markup', '--title=Biogenesis-R startup', '--text=Configuration failed', '--timeout=15'], 'notification uses only fixed error-dialog options')
    check(notices[0][1]['env']['DISPLAY'] == ':0' and notices[0][1]['env']['HOME'] == str(module['ACCOUNT_HOME']), 'profile admission failure does not disable desktop notification')

    # Exercise missing-display main orchestration without invoking any child or notifier
    host_python = Path(sys.executable).resolve()
    module.update(INSTALL=install, STATE=state, PROFILE=profile, STOP=0, ZENITY='/not-installed-fixture', PYTHON=str(host_python), ROOT_UID=host_python.stat().st_uid)
    original_display = module['display_ready']
    module['display_ready'] = lambda values: original_display(values, 0.01)
    fake_uname = type('Realm', (), {'machine': 'aarch64'})()
    with patch.object(os, 'uname', return_value=fake_uname), patch.object(sys, 'version_info', (3,13,0,'final',0)), patch.dict(os.environ, {}, clear=True):
        code = module['main']()
    failure = json.loads((state / 'status.json').read_text())
    check(code == 1 and failure['status'] == 'FAILED' and 'DISPLAY' in failure['error'], 'missing display produces bounded failure status without game')
    check(failure['notification'] == 'unavailable_without_desktop_environment', 'no false notification visibility without desktop')
    module['STOP'] = signal.SIGTERM
    with patch.object(os, 'uname', return_value=fake_uname), patch.object(sys, 'version_info', (3,13,0,'final',0)), patch.dict(os.environ, {}, clear=True):
        stopped = module['main']()
    stopped_status = json.loads((state / 'status.json').read_text())
    check(stopped == 1 and stopped_status['notification'] == 'suppressed_for_launcher_signal', 'intentional launcher signal does not open a new desktop dialog')
    module['STOP'] = 0
    check((state / 'previous-status.json').stat().st_size <= module['STATUS_BYTES'] and (state / 'status.json').stat().st_size <= module['STATUS_BYTES'], 'status history stays bounded across launches')

result = {'status': 'PASS_DISPOSABLE_LAUNCHER_FIXTURES_ONLY', 'checks': checks, 'count': len(checks),
          'source_sha256': hashlib.sha256(SOURCE.read_bytes()).hexdigest(), 'game_or_Pi_execution': False, 'host_main_fixture_seams': 'Main realm/version and system-file owner use explicit host fixture metadata. Product CLI has no such overrides. Actual Pi root ownership remains required.', 'policy_scope': 'Harmless children inherit CPU0/IDLE19/idleIO and finite fixture limits. Product AArch64 resource setup is source-reviewed, not executed here.',
          'product_CLI_executable_override': False, 'owned_signal_fixture': signal_status}
print(json.dumps(result, sort_keys=True))
