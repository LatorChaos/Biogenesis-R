"""Launch one approved living display without resetting its owned profile"""
import grp,pwd
import ctypes
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import selectors
import signal
import stat
import subprocess
import sys
import time

ACCOUNT_HOME = Path(pwd.getpwuid(os.getuid()).pw_dir)
INSTALL = ACCOUNT_HOME / '.local/share/biogenesis-r'
STATE = ACCOUNT_HOME / '.local/state/biogenesis-r'
PROFILE = STATE / 'profile'
LOG_BYTES = 2 * 1024**2
STATUS_BYTES = 32768
BUNDLE_BYTES = 256 * 1024**2
STOP = 0
CHILD_GATE = ("import os,sys\n"
              "if sys.stdin.buffer.read(4) != b'GO\\n': raise SystemExit(125)\n"
              "fd=os.open('/dev/null',os.O_RDONLY);os.dup2(fd,0);os.close(fd)\n"
              "os.execve(sys.argv[1],sys.argv[1:],dict(os.environ))\n")
PYTHON = '/usr/bin/python3.13'
X11 = '/usr/lib/aarch64-linux-gnu/libX11.so.6'
ZENITY = '/usr/bin/zenity'
ROOT_UID = 0
OBSERVED_SYSTEM = {}


class StartupBlocked(ValueError):
    """Retain unresolved same-boot diagnostics without overwriting their authority"""



def require(value, message):
    if not value:
        raise ValueError(message)


def generation(info):
    return tuple(getattr(info, key) for key in ('st_dev', 'st_ino', 'st_mode', 'st_uid',
                 'st_gid', 'st_nlink', 'st_size', 'st_mtime_ns', 'st_ctime_ns'))


def owned_directory(path):
    for parent in [path, *path.parents]:
        info = parent.lstat()
        require(stat.S_ISDIR(info.st_mode), 'Directory symlink or type differs: ' + str(parent))
    info = path.lstat()
    require(info.st_uid == os.getuid() and info.st_mode & 0o022 == 0, 'Unsafe owned directory: ' + str(path))


def regular(path, maximum, owner=None):
    for parent in [path.parent, *path.parent.parents]:
        require(stat.S_ISDIR(parent.lstat().st_mode), 'File ancestor differs: ' + str(parent))
    info = path.lstat()
    require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1 and info.st_size <= maximum,
            'Regular file bound or identity differs: ' + str(path))
    require(info.st_uid == (os.getuid() if owner is None else owner) and info.st_mode & 0o022 == 0,
            'Regular file ownership or mode differs: ' + str(path))
    return info


def read_file(path, maximum, owner=None):
    before = regular(path, maximum, owner)
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    try:
        require(generation(os.fstat(fd)) == generation(before), 'Opened file identity changed')
        remaining = before.st_size
        parts = []
        while remaining:
            chunk = os.read(fd, min(remaining, 1024**2))
            require(chunk, 'File was truncated')
            parts.append(chunk)
            remaining -= len(chunk)
        require(not os.read(fd, 1), 'File grew beyond admission')
        require(generation(os.fstat(fd)) == generation(before)
                and generation(path.lstat()) == generation(before), 'File generation changed')
        return b''.join(parts)
    finally:
        os.close(fd)


def pinned(path, row, maximum=128 * 1024**2, owner=None):
    body = read_file(path, maximum, owner)
    require(len(body) == row['bytes'] and hashlib.sha256(body).hexdigest() == row['sha256'],
            'Pinned file differs: ' + str(path))
    if 'mode' in row:
        require(stat.S_IMODE(path.lstat().st_mode) == row['mode'], 'Pinned file mode differs')
    return body


def sync_directory(path):
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)



def current_boot():
    with Path('/proc/sys/kernel/random/boot_id').open('rb') as source:
        body = source.read(38)
    require(re.fullmatch(rb'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\n', body), 'Invalid kernel boot identity')
    return body.decode('ascii').strip()


def diagnostic_file(path):
    info = regular(path, STATUS_BYTES)
    require(stat.S_IMODE(info.st_mode) == 0o600, 'Diagnostic mode differs')
    return info


def reconcile_boot(state):
    boot = current_boot()
    fences = []
    with os.scandir(state) as entries:
        count = 0
        for entry in entries:
            count += 1
            require(count <= 128, 'Unexpected launcher state entry count')
            if entry.name.startswith('boot-') and entry.name.endswith('.fence'):
                value = entry.name[5:-6]
                require(re.fullmatch(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', value), 'Malformed boot fence name')
                info = regular(Path(entry.path), 0)
                require(stat.S_IMODE(info.st_mode) == 0o600, 'Boot fence mode differs')
                fences.append((Path(entry.path), value, generation(info)))
    require(len(fences) <= 1, 'Multiple boot fences cannot establish launch ordering')
    diagnostic_names = ['status.json', 'previous-status.json', 'interrupted-status.json', 'status.pending.json']
    present = {}
    for name in diagnostic_names:
        path = state / name
        if os.path.lexists(path):
            present[name] = generation(diagnostic_file(path))
            if name in ['status.json', 'previous-status.json']:
                read_status(path)
    target = state / ('boot-' + boot + '.fence')
    if not fences:
        if present:
            raise StartupBlocked('Unfenced diagnostic records require explicit inspection')
        fd = os.open(target, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
        sync_directory(state)
        return {'boot_id': boot, 'transition': 'initial_fence', 'pending_archived': False}
    previous, prior_boot, prior_generation = fences[0]
    if prior_boot == boot:
        if 'status.pending.json' in present:
            raise StartupBlocked('Same-boot interrupted diagnostics cannot prove child closure')
        return {'boot_id': boot, 'transition': 'same_boot', 'pending_archived': False}
    # The child gate cannot release an engine before this fence becomes durable
    archived = False
    if 'status.pending.json' in present:
        pending, archive = state / 'status.pending.json', state / 'interrupted-status.json'
        require(generation(pending.lstat()) == present['status.pending.json'], 'Pending diagnostic generation changed')
        if 'interrupted-status.json' in present:
            require(generation(archive.lstat()) == present['interrupted-status.json'], 'Interrupted archive generation changed')
        os.replace(pending, archive)
        sync_directory(state)
        archived = True
    require(generation(previous.lstat()) == prior_generation and not os.path.lexists(target), 'Boot fence changed during transition')
    os.rename(previous, target)
    sync_directory(state)
    return {'boot_id': boot, 'prior_boot_id': prior_boot, 'transition': 'different_verified_boot', 'pending_archived': archived}


def system_file(path, maximum=64 * 1024**2):
    path = Path(path)
    require(path.is_absolute(), 'System path must be absolute')
    for item in [path, *path.parents]:
        info = item.lstat()
        require(info.st_uid == ROOT_UID and (stat.S_ISLNK(info.st_mode) or info.st_mode & 0o022 == 0), 'Untrusted system path')
    resolved = path.resolve(strict=True)
    for parent in resolved.parents:
        info = parent.lstat()
        require(stat.S_ISDIR(info.st_mode) and info.st_uid == ROOT_UID and info.st_mode & 0o022 == 0, 'Untrusted resolved system ancestor')
    before = resolved.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_uid == ROOT_UID and before.st_mode & 0o022 == 0
            and 0 < before.st_size <= maximum, 'Untrusted system file')
    fd = os.open(resolved, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        require(generation(os.fstat(fd)) == generation(before), 'System file descriptor changed')
        remaining, digest = before.st_size, hashlib.sha256()
        while remaining:
            data = os.read(fd, min(remaining, 1024**2))
            require(data, 'System file truncated')
            digest.update(data)
            remaining -= len(data)
        require(not os.read(fd, 1) and generation(os.fstat(fd)) == generation(before)
                and generation(resolved.lstat()) == generation(before) and path.resolve(strict=True) == resolved,
                'System file generation changed')
    finally:
        os.close(fd)
    observed = {'path': str(path), 'resolved': str(resolved), 'bytes': before.st_size, 'sha256': digest.hexdigest()}
    OBSERVED_SYSTEM[str(path)] = observed
    return observed


def interpreter():
    require(sys.version_info[:2] == (3, 13), 'Supported Python3.13 interpreter required')
    observed = system_file(PYTHON)
    require(Path(sys.executable).resolve() == Path(observed['resolved']), 'Installed interpreter path differs')
    return observed


def status_write(state, value):
    value = dict(value)
    value['boot_id'] = current_boot()
    value['observed_system'] = dict(OBSERVED_SYSTEM)
    body = (json.dumps(value, sort_keys=True) + '\n').encode('ascii')
    require(len(body) <= STATUS_BYTES, 'Status exceeds its bound')
    target, pending = state / 'status.json', state / 'status.pending.json'
    if target.exists() or target.is_symlink():
        regular(target, STATUS_BYTES)
    fd = os.open(pending, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        with os.fdopen(fd, 'wb') as stream:
            require(stream.write(body) == len(body), 'Incomplete status write')
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(pending, target)
        sync_directory(state)
    except BaseException:
        # Same-boot interruption stays blocked; a verified different boot can archive diagnostics
        raise


def lock(state):
    owned_directory(state)
    path = state / 'living-display.lock'
    fd = os.open(path, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC, 0o600)
    try:
        info = os.fstat(fd)
        require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and info.st_nlink == 1
                and stat.S_IMODE(info.st_mode) == 0o600 and info.st_size == 0, 'Unsafe launch lock')
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        os.close(fd)
        return None
    except BaseException:
        os.close(fd)
        raise
    return fd


class Log:
    """Keep only two fixed-size diagnostic files, including across launches"""
    def __init__(self, directory):
        owned_directory(directory)
        self.directory = directory
        require(set(p.name for p in directory.iterdir()) <= {'current.log', 'previous.log'}, 'Unknown diagnostic entry')
        for path in directory.iterdir():
            regular(path, LOG_BYTES)
        self.fd = None
        self.total = 0
        self.rotations = 0
        self.open()

    def open(self):
        path = self.directory / 'current.log'
        self.fd = os.open(path, os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_NOFOLLOW, 0o600)
        info = os.fstat(self.fd)
        require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid()
                and info.st_nlink == 1 and stat.S_IMODE(info.st_mode) == 0o600 and info.st_size <= LOG_BYTES,
                'Unsafe current diagnostic file')
        self.size = info.st_size

    def write(self, body):
        require(len(body) <= 65536, 'One diagnostic read exceeds its bound')
        while body:
            if self.size == LOG_BYTES:
                os.fsync(self.fd)
                os.close(self.fd)
                self.fd = None
                current, previous = self.directory / 'current.log', self.directory / 'previous.log'
                regular(current, LOG_BYTES)
                if previous.exists() or previous.is_symlink():
                    regular(previous, LOG_BYTES)
                os.replace(current, previous)
                self.open()
                sync_directory(self.directory)
                self.rotations += 1
            part, body = body[:LOG_BYTES - self.size], body[LOG_BYTES - self.size:]
            remaining = memoryview(part)
            while remaining:
                count = os.write(self.fd, remaining)
                require(count > 0, 'Incomplete diagnostic write')
                remaining = remaining[count:]
                self.size += count
                self.total += count

    def close(self):
        if self.fd is not None:
            os.fsync(self.fd)
            os.close(self.fd)
            self.fd = None


def bundle(install):
    owned_directory(install)
    app = install / 'app'
    owned_directory(app)
    manifest = json.loads(read_file(install / 'bundle-manifest.json', 65536))
    require(manifest['schema'] == 1 and manifest['ready'] is True
            and manifest['startup_interlock_qualified'] is True
            and manifest['file_logging_disabled_qualified'] is True, 'Installed bundle is not qualified')
    rows = manifest['files']
    required = {'Biogenesis-R.arm64', 'Biogenesis-R.pck', 'bin/libbiogenesis_godot.arm64.so'}
    require(isinstance(rows, dict) and required <= set(rows) and 3 <= len(rows) <= 128, 'Bundle manifest schema differs')
    total = 0
    for name, row in rows.items():
        relative = Path(name)
        require(not relative.is_absolute() and str(relative) == name and '..' not in relative.parts
                and name != '.' and len(name) <= 512, 'Unsafe bundle-relative path')
        body = pinned(app / name, row)
        total += len(body)
        require(total <= BUNDLE_BYTES, 'Bundle exceeds aggregate bound')
        if name == 'Biogenesis-R.arm64':
            require(body[:6] == b'\x7fELF\x02\x01' and int.from_bytes(body[18:20], 'little') == 183
                    and row['mode'] == 0o755, 'Expected executable AArch64 engine')
    present = set()
    directories = 0
    for current, dirs, names in os.walk(app, followlinks=False):
        directories += len(dirs)
        require(directories <= 128, 'Bundle directory count exceeds bound')
        for name in dirs:
            owned_directory(Path(current) / name)
        for name in names:
            present.add(str((Path(current) / name).relative_to(app)))
            require(len(present) <= 128, 'Bundle file count exceeds bound')
    require(present == set(rows), 'Unmanifested bundle files differ')
    # Loose settings or code must not bypass the approved pack
    for forbidden in ['override.cfg', 'project.godot', 'project.binary']:
        require(not os.path.lexists(app / forbidden), 'Unexpected loose project override')
    return app, manifest


def readiness_policy():
    resource.setrlimit(resource.RLIMIT_AS, (512 * 1024**2, 512 * 1024**2))
    resource.setrlimit(resource.RLIMIT_FSIZE, (65536, 65536))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    os.sched_setaffinity(0, {0})
    os.sched_setscheduler(0, os.SCHED_IDLE, os.sched_param(0))
    os.setpriority(os.PRIO_PROCESS, 0, 19)


def display_probe_source():
    return ("import ctypes,sys\n"
            "lib=ctypes.CDLL(" + repr(X11) + ")\n"
            "lib.XOpenDisplay.argtypes=[ctypes.c_char_p];lib.XOpenDisplay.restype=ctypes.c_void_p\n"
            "lib.XCloseDisplay.argtypes=[ctypes.c_void_p];lib.XCloseDisplay.restype=ctypes.c_int\n"
            "d=lib.XOpenDisplay(sys.argv[1].encode('ascii'))\n"
            "if not d:raise SystemExit(2)\n"
            "lib.XCloseDisplay(d)\n")


def bounded_display_probe(display, seconds, authority):
    process = subprocess.Popen([sys.executable, '-I', '-B', '-c', display_probe_source(), display],
                               stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                               start_new_session=True, close_fds=True, preexec_fn=readiness_policy,
                               env={'PATH': '/usr/bin:/bin', 'DISPLAY': display,
                                    'XAUTHORITY': authority})
    row = birth(process.pid)
    try:
        return process.wait(timeout=seconds) == 0
    except subprocess.TimeoutExpired:
        for signum in [signal.SIGTERM, signal.SIGKILL]:
            if same_birth(row):
                try:
                    os.killpg(row['group'], signum)
                except ProcessLookupError:
                    pass
            try:
                process.wait(timeout=0.25)
                break
            except subprocess.TimeoutExpired:
                pass
        require(process.poll() is not None, 'Readiness helper cleanup exceeded its bound')
        return False
    finally:
        if process.poll() is not None:
            try:
                os.killpg(row['group'], 0)
            except ProcessLookupError:
                pass
            else:
                raise RuntimeError('Readiness helper group is not closed')


def display_runtime_directory(path):
    require(path == Path('/run/user/1000'), 'Inherited runtime directory differs')
    for parent in [path, *path.parents]:
        require(stat.S_ISDIR(parent.lstat().st_mode), 'Runtime directory symlink or type differs')
    info = path.lstat()
    if info.st_mode & 0o022 == 0:
        owned_directory(path)
        return
    require(os.getuid() == os.getgid() == info.st_uid == info.st_gid == 1000
            and stat.S_IMODE(info.st_mode) == 0o770, 'Runtime directory owner or mode differs')
    account = pwd.getpwuid(1000)
    group = grp.getgrgid(1000)
    require(account.pw_gid == group.gr_gid == 1000
            and all(name == account.pw_name for name in group.gr_mem)
            and all(row.pw_name == account.pw_name and row.pw_uid == 1000
                    for row in pwd.getpwall() if row.pw_gid == 1000),
            'Runtime directory primary group is not private')


def display_ready(inherited, seconds=30):
    end = time.monotonic() + seconds
    error = 'Desktop display environment is incomplete'
    while time.monotonic() < end and not STOP:
        try:
            display = inherited.get('DISPLAY', '')
            require(re.fullmatch(r':[0-9]+(?:\.[0-9]+)?', display), 'Inherited local DISPLAY is required')
            runtime = Path(inherited.get('XDG_RUNTIME_DIR', ''))
            require(runtime == Path('/run/user/1000'), 'Inherited runtime directory differs')
            display_runtime_directory(runtime)
            authority = Path(inherited.get('XAUTHORITY', ''))
            require(authority.is_absolute(), 'Inherited authority path is required')
            regular(authority, 65536)
            number = display[1:].split('.')[0]
            require(stat.S_ISSOCK(Path('/tmp/.X11-unix/X' + number).stat().st_mode), 'Display socket is not ready')
            system_file(X11, 16 * 1024**2)
            remaining = end - time.monotonic() - 0.5
            require(remaining > 0, 'Display readiness deadline expired')
            require(bounded_display_probe(display, min(3, remaining), str(authority)), 'Display authentication probe failed or timed out')
            return
        except (ValueError, OSError) as failure:
            error = str(failure)
        time.sleep(min(0.2, max(0, end - time.monotonic())))
    raise ValueError(error if not STOP else 'Startup cancelled before display readiness')


def environment(profile, inherited):
    for part in ['home', 'config', 'cache', 'data', 'tmp']:
        owned_directory(profile / part)
    result = {'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8', 'LC_ALL': 'C.UTF-8',
              'HOME': str(profile / 'home'), 'TMPDIR': str(profile / 'tmp'),
              'XDG_CONFIG_HOME': str(profile / 'config'), 'XDG_CACHE_HOME': str(profile / 'cache'),
              'XDG_DATA_HOME': str(profile / 'data'), 'DBUS_SESSION_BUS_ADDRESS': 'unix:path=' + str(profile / 'absent-bus')}
    for name in ['DISPLAY', 'XAUTHORITY', 'XDG_RUNTIME_DIR', 'WAYLAND_DISPLAY']:
        if name in inherited:
            require('\0' not in inherited[name] and len(inherited[name]) <= 4096, 'Desktop variable exceeds bound')
            result[name] = inherited[name]
    return result


def child_policy():
    resource.setrlimit(resource.RLIMIT_AS, (8 * 1024**3, 8 * 1024**3))
    resource.setrlimit(resource.RLIMIT_FSIZE, (1024**3, 1024**3))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_NICE, (0, 0))
    resource.setrlimit(resource.RLIMIT_RTPRIO, (0, 0))
    os.sched_setaffinity(0, {0, 1, 2, 3})
    os.sched_setscheduler(0, os.SCHED_OTHER, os.sched_param(0))
    os.setpriority(os.PRIO_PROCESS, 0, 0)
    libc = ctypes.CDLL(None, use_errno=True)
    require(libc.prctl(38, 1, 0, 0, 0) == 0, 'Cannot set no-new-privileges')
    require(libc.syscall(30, 1, 0, 3 << 13) == 0, 'Cannot set AArch64 idle I/O priority')
    fields = dict(line.split(':', 1) for line in Path('/proc/self/status').read_text().splitlines() if ':' in line)
    require(fields['NoNewPrivs'].strip() == '1' and all(int(fields[k].strip(), 16) == 0
            for k in ['CapEff', 'CapPrm', 'CapInh', 'CapAmb']), 'Runtime privilege context differs')


def birth(pid):
    with Path('/proc/%d/stat' % pid).open('rb') as source:
        raw = source.read(8193)
    require(len(raw) <= 8192, 'Named stat exceeds bound')
    fields = raw[raw.rfind(b')') + 2:].split()
    return {'pid': pid, 'group': int(fields[2]), 'starttime': int(fields[19]),
            'namespace': os.readlink('/proc/%d/ns/pid' % pid)}


def same_birth(expected):
    try:
        actual = birth(expected['pid'])
        return actual == expected and Path('/proc/%d' % expected['pid']).stat().st_uid == os.getuid()
    except (FileNotFoundError, ProcessLookupError):
        return False


def valid_boot(value):
    return isinstance(value, str) and re.fullmatch(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', value) is not None


def read_status(path):
    try:
        value = json.loads(read_file(path, STATUS_BYTES))
        require(isinstance(value, dict), 'Status must be an object')
        game = value.get('game')
        require(game is None or isinstance(game, dict), 'Recorded game must be an object or null')
        for record in [value] + ([game] if game is not None else []):
            require(valid_boot(record.get('boot_id')), 'Recorded boot identity is missing or malformed')
            require(record.get('status') in ['ADMITTING', 'STARTING', 'RUNNING', 'EXITED', 'FAILED'], 'Recorded status kind differs')
            row = record.get('birth')
            if row is not None:
                require(isinstance(row, dict) and set(row) == {'pid', 'group', 'starttime', 'namespace'}, 'Recorded birth schema differs')
                require(all(type(row[key]) is int and row[key] > 0 for key in ['pid', 'group', 'starttime'])
                        and row['pid'] == row['group'], 'Recorded birth integers differ')
                require(isinstance(row['namespace'], str) and re.fullmatch(r'pid:\[[1-9][0-9]*\]', row['namespace']), 'Recorded namespace differs')
            require(record['status'] != 'RUNNING' or row is not None, 'RUNNING requires a recorded birth')
        return value
    except (ValueError, TypeError, KeyError, RecursionError) as error:
        raise StartupBlocked('Corrupt complete launcher status: ' + str(error)[:512]) from error


def recorded_game(state):
    path = state / 'status.json'
    if not os.path.lexists(path):
        return None
    value = read_status(path)
    return value['game'] if isinstance(value.get('game'), dict) else value


def existing_application(state):
    current = recorded_game(state)
    if not current or current.get('boot_id') != current_boot():
        return False
    if current.get('cleanup_error'):
        raise StartupBlocked('Same-boot child closure is unresolved; no second engine may start')
    row = current.get('birth')
    if not isinstance(row, dict):
        return False
    if same_birth(row):
        return True
    cleanup = current.get('cleanup')
    if current.get('cleanup_error') or not isinstance(cleanup, dict) or cleanup.get('group_absent') is not True:
        raise StartupBlocked('Same-boot child closure is unresolved; no second engine may start')
    return False


def children(pid):
    pending, seen, rows = [pid], set(), []
    while pending:
        current = pending.pop()
        if current in seen:
            continue
        seen.add(current)
        require(len(seen) <= 32, 'Named child tree exceeds bound')
        try:
            rows.append(birth(current))
            tasks = list(Path('/proc/%d/task' % current).iterdir())
            require(len(tasks) <= 256, 'Named task count exceeds bound')
            for task in tasks:
                with (task / 'children').open('rb') as source:
                    raw = source.read(4097)
                require(len(raw) <= 4096, 'Named children exceed read bound')
                pending.extend(int(value) for value in raw.split())
        except (FileNotFoundError, ProcessLookupError):
            continue
    return rows


def reap_adopted():
    while True:
        try:
            pid, _ = os.waitpid(-1, os.WNOHANG)
        except ChildProcessError:
            return
        if not pid:
            return


def close_child(process, root, known):
    actions = []
    for signum, seconds in [(signal.SIGTERM, 2), (signal.SIGKILL, 2)]:
        if same_birth(root):
            require(root['pid'] == root['group'] and root['group'] != os.getpgrp(), 'Unowned signal group')
            try:
                os.killpg(root['group'], signum)
            except ProcessLookupError:
                pass
            actions.append({'group': root['group'], 'signal': signum})
        else:
            for row in known.values():
                if same_birth(row) and row['group'] == root['group']:
                    try:
                        os.kill(row['pid'], signum)
                    except ProcessLookupError:
                        pass
                    actions.append({'pid': row['pid'], 'signal': signum})
        until = time.monotonic() + seconds
        while time.monotonic() < until:
            if process.poll() is not None:
                reap_adopted()
                try:
                    os.killpg(root['group'], 0)
                except ProcessLookupError:
                    return {'group_absent': True, 'actions': actions}
            time.sleep(0.02)
    raise RuntimeError('Owned child group closure is incomplete')


def observer_policy():
    os.sched_setaffinity(0, {0})
    os.sched_setscheduler(0, os.SCHED_IDLE, os.sched_param(0))
    os.setpriority(os.PRIO_PROCESS, 0, 19)
    libc = ctypes.CDLL(None, use_errno=True)
    require(libc.syscall(30, 1, 0, 3 << 13) == 0, 'Cannot set observer idle I/O')


def supervise(argv, env, cwd, log, state):
    selector = selectors.DefaultSelector()
    process = None
    root = None
    known = {}
    outcome = {'status': 'STARTING', 'exit_code': None, 'cleanup': None, 'boot_id': current_boot()}
    libc = ctypes.CDLL(None, use_errno=True)
    previous = ctypes.c_int()
    require(libc.prctl(37, ctypes.byref(previous), 0, 0, 0) == 0, 'Cannot read subreaper state')
    require(libc.prctl(36, 1, 0, 0, 0) == 0, 'Cannot set subreaper state')
    try:
        reconcile_boot(state)
        require(not STOP, 'Startup was cancelled')
        gated = [sys.executable, '-I', '-B', '-c', CHILD_GATE, *argv]
        process = subprocess.Popen(gated, cwd=cwd, env=env, stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                   start_new_session=True, close_fds=True, preexec_fn=child_policy)
        root = birth(process.pid)
        require(root['group'] == process.pid, 'Child did not create its own group')
        known[(root['pid'], root['starttime'])] = root
        outcome.update(status='RUNNING', birth=root, boot_id=current_boot())
        status_write(state, outcome)
        require(not STOP, 'Startup cancelled before engine admission')
        process.stdin.write(b'GO\n')
        process.stdin.close()
        observer_policy()
        os.set_blocking(process.stdout.fileno(), False)
        selector.register(process.stdout, selectors.EVENT_READ)
        last_tree = 0.0
        exit_drain_deadline = None
        while selector.get_map() or process.poll() is None:
            if STOP:
                raise InterruptedError('Launcher received signal ' + str(STOP))
            if process.poll() is None and time.monotonic() - last_tree >= 0.5:
                live = [row for row in children(process.pid) if row['group'] == root['group']]
                # Keep current ancestry only; long sessions do not accumulate a history
                known = {(row['pid'], row['starttime']): row for row in live}
                known[(root['pid'], root['starttime'])] = root
                last_tree = time.monotonic()
            for key, _ in selector.select(0.2):
                chunk = os.read(key.fileobj.fileno(), 65536)
                if chunk:
                    log.write(chunk)
                else:
                    selector.unregister(key.fileobj)
            if process.poll() is not None:
                if exit_drain_deadline is None:
                    exit_drain_deadline = time.monotonic() + 1
                if not selector.get_map() or time.monotonic() >= exit_drain_deadline:
                    break
        outcome.update(status='EXITED' if process.returncode == 0 else 'FAILED', exit_code=process.returncode)
    except BaseException as error:
        outcome.update(status='FAILED', error=(type(error).__name__ + ': ' + str(error))[:2048])
    finally:
        try:
            if process is not None:
                if process.stdin is not None and not process.stdin.closed:
                    process.stdin.close()
                require(root is not None, 'Child birth was not observed; no group signal authority')
                if process.poll() is None or any(same_birth(row) for row in known.values()):
                    outcome['cleanup'] = close_child(process, root, known)
                else:
                    reap_adopted()
                    try:
                        os.killpg(root['group'], 0)
                    except ProcessLookupError:
                        outcome['cleanup'] = {'group_absent': True, 'actions': []}
                    else:
                        raise RuntimeError('Child exited with unqualified remaining group members')
                process.wait(timeout=1)
                outcome['exit_code'] = process.returncode
                process.stdout.close()
        except BaseException as error:
            outcome.update(status='FAILED', cleanup_error=str(error)[:2048])
        selector.close()
        if libc.prctl(36, previous.value, 0, 0, 0) != 0:
            outcome.update(status='FAILED', subreaper_restore_error='Cannot restore subreaper state')
        outcome.update(log_bytes_received=log.total, log_rotations=log.rotations)
        try:
            status_write(state, outcome)
        except BaseException as error:
            outcome.update(status='FAILED', status_publication_error=(type(error).__name__ + ': ' + str(error))[:2048])
    return outcome


def notify_failure(message, inherited, profile):
    if not inherited.get('DISPLAY') and not inherited.get('WAYLAND_DISPLAY'):
        return 'unavailable_without_desktop_environment'
    path = Path(ZENITY)
    if not path.exists():
        return 'not_installed'
    system_file(path, 16 * 1024**2)
    # Notification remains usable when the application's profile admission failed
    env = {'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8', 'LC_ALL': 'C.UTF-8', 'HOME': str(ACCOUNT_HOME)}
    for key in ['DISPLAY', 'WAYLAND_DISPLAY', 'XAUTHORITY', 'XDG_RUNTIME_DIR', 'DBUS_SESSION_BUS_ADDRESS']:
        if key in inherited:
            value = inherited[key]
            require('\0' not in value and len(value) <= 4096, 'Notification environment exceeds bound')
            if key == 'DBUS_SESSION_BUS_ADDRESS':
                require(value.startswith('unix:'), 'Notification requires a local session bus')
            env[key] = value
    process = subprocess.Popen([str(path), '--error', '--no-markup', '--title=Biogenesis-R startup',
                               '--text=' + message[:1024], '--timeout=15'], env=env,
                               stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                               start_new_session=True, close_fds=True)
    row = birth(process.pid)
    try:
        return 'exit_' + str(process.wait(timeout=17))
    except subprocess.TimeoutExpired:
        if same_birth(row):
            os.killpg(row['group'], signal.SIGTERM)
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            if same_birth(row):
                os.killpg(row['group'], signal.SIGKILL)
            process.wait(timeout=2)
        return 'timed_out'


def main():
    require(len(sys.argv) == 1, 'This launcher accepts no runtime or executable arguments')
    require(ACCOUNT_HOME.is_absolute(), 'Account home must be absolute')
    require(os.getuid() == os.getgid() == 1000 and os.uname().machine == 'aarch64', 'Wrong installed runtime realm')
    owned_directory(STATE)
    descriptor = lock(STATE)
    if descriptor is None:
        print('Biogenesis-R already owns the launch lock', file=sys.stderr)
        return 0
    inherited = {key: os.environ[key] for key in ['DISPLAY', 'WAYLAND_DISPLAY', 'XAUTHORITY',
                 'XDG_RUNTIME_DIR', 'DBUS_SESSION_BUS_ADDRESS'] if key in os.environ}
    log = None
    result = None
    try:
        try:
            reconcile_boot(STATE)
            result = recorded_game(STATE)
            if existing_application(STATE):
                print('An existing recorded Biogenesis-R child remains alive', file=sys.stderr)
                return 0
        except (OSError, ValueError, TypeError, KeyError, RecursionError) as error:
            raise StartupBlocked('Launcher diagnostic admission refused: ' + str(error)[:512]) from error
        result = None
        for name in ['status.json', 'previous-status.json']:
            path = STATE / name
            if path.exists() or path.is_symlink():
                regular(path, STATUS_BYTES)
        if (STATE / 'status.json').exists():
            os.replace(STATE / 'status.json', STATE / 'previous-status.json')
        log = Log(STATE / 'logs')
        status_write(STATE, {'status': 'ADMITTING'})
        interpreter()
        app, manifest = bundle(INSTALL)
        env = environment(PROFILE, inherited)
        display_ready(inherited)
        available = next(line for line in Path('/proc/meminfo').read_text().splitlines() if line.startswith('MemAvailable:'))
        require(int(available.split()[1]) * 1024 >= 1024**3, 'Available memory below one GiB')
        argv = [str(app / 'Biogenesis-R.arm64'), '--audio-driver', 'Dummy', '--display-driver', 'wayland',
                '--rendering-method', 'gl_compatibility', '--rendering-driver', 'opengl3_es',
                '--disable-vsync', '--max-fps', '30', '--', '--safe-graphics', '--resume-managed', '827364']
        result = supervise(argv, env, app, log, STATE)
        if result['status'] != 'EXITED':
            raise RuntimeError(result.get('error', 'Application exited unexpectedly; inspect bounded logs'))
        return 0
    except BaseException as error:
        message = (type(error).__name__ + ': ' + str(error))[:2048]
        notification = 'not_attempted'
        if STOP:
            notification = 'suppressed_for_launcher_signal'
        else:
            try:
                notification = notify_failure(message, inherited, PROFILE)
            except BaseException as notice_error:
                notification = 'failed: ' + str(notice_error)[:512]
        if isinstance(error, StartupBlocked):
            print('Notification outcome: ' + notification[:512], file=sys.stderr)
        if not isinstance(error, StartupBlocked):
            try:
                status_write(STATE, {'status': 'FAILED', 'error': message, 'notification': notification, 'game': result})
            except BaseException as status_error:
                print('Primary failure retained; status publication failed: ' + str(status_error)[:512], file=sys.stderr)
        print(message, file=sys.stderr)
        return 1
    finally:
        if log is not None:
            log.close()
        os.close(descriptor)


if __name__ == '__main__':
    def stop(number, frame):
        global STOP
        STOP = number
    for number in [signal.SIGTERM, signal.SIGINT, signal.SIGHUP]:
        signal.signal(number, stop)
    raise SystemExit(main())
