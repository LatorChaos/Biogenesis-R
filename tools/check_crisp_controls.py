"""Exercise actual crisp GUI focus plus the existing native control journey"""

import argparse
import hashlib
import json
import os
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.dont_write_bytecode = True

from build import ROOT, executable, godot_errors, local_path
from check_native_controls import NativeInput, inside as regular_inside
from run_reference_oracles import run_command


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def focused_inside(engine, run, quit_only=False):
    require(
        os.environ.get('BGR_TEST_ROOT') == str(run) and os.environ.get('DISPLAY', '').startswith(':'),
        'Owned display is required',
    )
    log = run / 'engine.log'
    actions = []
    checks = []
    native = None
    with log.open('wb') as output:
        process = subprocess.Popen(
            [
                str(engine),
                '--audio-driver',
                'Dummy',
                '--path',
                str(ROOT),
                '--script',
                'res://ui/crisp_check.gd',
                '--resolution',
                '1200x900',
            ],
            stdout=output,
            stderr=subprocess.STDOUT,
        )

        def lines(prefix):
            text = log.read_text(errors='replace')
            require(
                'BGR_GUI_FAIL' not in text and (not godot_errors(text)),
                'Actual frontend error: ' + str(log),
            )
            return [
                json.loads(line[len(prefix) + 1:])
                for line in text.splitlines()
                if line.startswith(prefix + ' ')
            ]

        def latest():
            records = lines('BGR_CRISP_OBS')
            return records[-1] if records else None

        def wait(description, predicate, timeout=12):
            until = time.monotonic() + timeout
            while time.monotonic() < until:
                observation = latest()
                if observation and predicate(observation):
                    return observation
                require(process.poll() is None, 'Frontend exited before ' + description)
                time.sleep(0.01)
            raise ValueError('Timed out ' + description + '; latest=' + repr(latest()))

        def checkpoint(label):
            pending = run / 'checkpoint-request.pending'
            save(pending, {'label': label})
            pending.replace(run / 'checkpoint-request.json')
            until = time.monotonic() + 12
            while time.monotonic() < until:
                for observation in lines('BGR_CRISP_CHECKPOINT'):
                    if observation['label'] == label:
                        checks.append(observation)
                        return observation
                require(process.poll() is None, 'Frontend exited before checkpoint ' + label)
                time.sleep(0.01)
            raise ValueError('Checkpoint absent ' + label)

        def key(name):
            native.key(name)
            actions.append({'kind': 'key', 'key': name, 'monotonic_ns': time.monotonic_ns()})

        def focus(name):
            for _ in range(40):
                observed = latest()
                if observed['focus'] == name:
                    return observed
                key('Tab')
                time.sleep(0.035)
            raise ValueError('Tab could not focus ' + name + '; actual=' + repr(latest()))

        def click(name):
            observed = latest()
            native.click(observed, name)
            actions.append({'kind': 'click', 'control': name, 'monotonic_ns': time.monotonic_ns()})

        def unchanged(observed):
            require(
                observed['paused'] and observed['tick'] == 0 and observed['model_sha256'] == expected_model,
                'GUI action changed the authoritative paused model',
            )

        try:
            ready = wait(
                'initial model',
                lambda row: row['same_initial'] and row['paused'] and (row['tick'] == 0),
            )
            expected_model = ready['model_sha256']
            native = NativeInput()
            window = native.find_window('Biogenesis-R')
            native.focus(native.display, window, 2, 0)
            native.flush(native.display)
            require(
                not ready['checkbox'] and (not ready['canvas_crisp']),
                'Smooth is not the unchecked startup default',
            )
            checkpoint('initial')
            if quit_only:
                focus('QuitButton')
                checkpoint('focused_quit_space')
                key('space')
                wait('actual exit confirmation', lambda row: row['decision_visible'] and row['decision_controls_ready'] and row['decision'] == 'quit')
                click('ConfirmWorldDecision')
                status = process.wait(timeout=5)
                require(status == 0, 'Focused Quit Space failed to exit')
                require(
                    not godot_errors(log.read_text(errors='replace')),
                    'Late focused-Quit error',
                )
                save(
                    run / 'summary.json',
                    {
                        'result': 'PASS',
                        'kind': 'focused_quit_space',
                        'engine_exit_code': status,
                        'checks': checks,
                        'actions': actions,
                        'world_state_injection': False,
                    },
                )
                print(json.dumps({'result': 'PASS', 'focused_quit_space': True}), flush=True)
                return
            click('CrispLines')
            wait('mouse crisp', lambda row: row['checkbox'] and row['canvas_crisp'])
            observed = checkpoint('mouse_crisp')
            unchanged(observed)
            require(observed['focus'] == 'CrispLines', 'Mouse did not focus checkbox')
            key('space')
            time.sleep(0.15)
            observed = checkpoint('focused_checkbox_space')
            require(
                (
                    not observed['checkbox']
                    and not observed['canvas_crisp']
                    and observed['paused']
                    and observed['tick'] == 0
                    and observed['same_initial']
                ),
                'Focused checkbox Space must toggle quality without pausing or advancing',
            )
            focus('StepButton')
            key('space')
            wait('focused step', lambda row: row['paused'] and row['tick'] == 1)
            checkpoint('focused_step_space')
            focus('PauseButton')
            key('space')
            wait('focused resume', lambda row: not row['paused'] and row['tick'] > 1)
            checkpoint('focused_pause_space')
            click('CrispLines')
            wait('running mouse crisp', lambda row: row['checkbox'] and row['canvas_crisp'])
            observed = checkpoint('running_mouse_crisp')
            require(not observed['paused'], 'Quality mouse toggle paused a running world')
            key('space')
            time.sleep(0.1)
            observed = checkpoint('running_checkbox_space')
            require(
                not observed['checkbox'] and (not observed['canvas_crisp']) and (not observed['paused']),
                'Quality Space paused a running world',
            )
            click('WorldCanvas')
            time.sleep(0.05)
            key('space')
            wait('world pause', lambda row: row['paused'])
            observed = checkpoint('world_pause')
            tick = observed['tick']
            time.sleep(0.25)
            require(latest()['tick'] == tick, 'Paused world advanced')
            focus('ResetButton')
            key('space')
            wait('actual reset confirmation', lambda row: row['decision_visible'] and row['decision_controls_ready'] and row['decision'] == 'reset')
            click('ConfirmWorldDecision')
            wait('focused reset', lambda row: row['same_initial'])
            checkpoint('focused_reset_space')
            zoom = latest()['zoom']
            focus('ZoomInButton')
            key('space')
            wait('focused zoom in', lambda row: row['zoom'] > zoom)
            checkpoint('focused_zoom_in_space')
            zoom = latest()['zoom']
            focus('ZoomOutButton')
            key('space')
            wait('focused zoom out', lambda row: row['zoom'] < zoom)
            checkpoint('focused_zoom_out_space')
            focus('FitButton')
            key('space')
            wait('focused fit', lambda row: row['fit'])
            checkpoint('focused_fit_space')
            click('WorldCanvas')
            key('n')
            wait('world N', lambda row: row['paused'] and row['tick'] == 1)
            checkpoint('world_step_key')
            key('r')
            wait('actual reset confirmation', lambda row: row['decision_visible'] and row['decision_controls_ready'] and row['decision'] == 'reset')
            click('ConfirmWorldDecision')
            wait('world R', lambda row: row['same_initial'])
            checkpoint('world_reset_key')
            zoom = latest()['zoom']
            key('equal')
            wait('world plus', lambda row: row['zoom'] > zoom)
            checkpoint('world_plus_key')
            zoom = latest()['zoom']
            key('minus')
            wait('world minus', lambda row: row['zoom'] < zoom)
            checkpoint('world_minus_key')
            key('f')
            wait('world fit', lambda row: row['fit'])
            checkpoint('world_fit_key')
            focus('CrispLines')
            key('space')
            wait('keyboard crisp', lambda row: row['checkbox'] and row['canvas_crisp'])
            unchanged(checkpoint('keyboard_crisp'))
            key('n')
            wait('crisp step', lambda row: row['tick'] == 1 and row['paused'])
            checkpoint('crisp_step')
            key('r')
            wait('actual reset confirmation', lambda row: row['decision_visible'] and row['decision_controls_ready'] and row['decision'] == 'reset')
            click('ConfirmWorldDecision')
            wait('crisp reset', lambda row: row['same_initial'])
            observed = checkpoint('crisp_reset')
            require(
                observed['checkbox'] and observed['canvas_crisp'],
                'Reset lost session quality selection',
            )
            key('n')
            wait('pre create step', lambda row: row['tick'] == 1 and row['paused'])
            focus('CreateWorldButton')
            key('space')
            wait('actual replacement confirmation', lambda row: row['decision_visible'] and row['decision_controls_ready'] and row['decision'] == 'create')
            click('ConfirmWorldDecision')
            created = wait('focused create', lambda row: row['paused'] and row['tick'] == 0 and not row['same_initial'] and not row['decision_visible'])
            require(created['configuration'] == ready['configuration'], 'Create changed the requested configuration')
            expected_model = created['model_sha256']
            observed = checkpoint('focused_create_space')
            require(
                observed['checkbox'] and observed['canvas_crisp'],
                'Create lost session quality selection',
            )
            observed = latest()
            native.resize(native.display, window, 800, 660)
            native.flush(native.display)
            wait('minimum viewport', lambda row: row['window_size'] == [800, 660])
            unchanged(checkpoint('minimum_crisp'))
            observed = latest()
            native.replace_text(observed, 'SeedInput', 'nrfq')
            actions.append({
                'kind': 'replace_text',
                'control': 'SeedInput',
                'value': 'nrfq',
                'monotonic_ns': time.monotonic_ns(),
            })
            key('space')
            wait('text focus', lambda row: row['texts']['SeedInput'] == 'nrfq ')
            unchanged(checkpoint('line_edit_shortcuts_are_text'))
            observed = latest()
            native.replace_text(observed, 'SeedInput', '827364')
            actions.append({
                'kind': 'replace_text',
                'control': 'SeedInput',
                'value': '827364',
                'monotonic_ns': time.monotonic_ns(),
            })
            wait('restored text', lambda row: row['texts']['SeedInput'] == '827364')
            unchanged(checkpoint('line_edit_restored'))
            require(
                latest()['focus'] == 'SeedInput',
                'Final Escape does not have real LineEdit focus',
            )
            key('Escape')
            wait('actual exit confirmation', lambda row: row['decision_visible'] and row['decision_controls_ready'] and row['decision'] == 'quit')
            click('ConfirmWorldDecision')
            status = process.wait(timeout=5)
            require(status == 0, 'Focused LineEdit Escape failed to quit')
            require(not godot_errors(log.read_text(errors='replace')), 'Late frontend error')
            result = {
                'result': 'PASS',
                'kind': 'focused_crisp_controls',
                'engine_exit_code': status,
                'checks': checks,
                'actions': actions,
                'input': (
                    'Actual XTest keys, clicks and Tab focus; '
                    'observer only reads state and requests rendered screenshots'
                ),
                'escape_focus': 'SeedInput',
                'world_state_injection': False,
            }
            save(run / 'summary.json', result)
            print(json.dumps({'result': 'PASS', 'focused_checks': len(checks)}), flush=True)
        finally:
            save(run / 'actions.json', actions)
            save(run / 'checkpoints.json', checks)
            if native:
                native.close(native.display)
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--godot')
    parser.add_argument('--inside', type=Path)
    parser.add_argument('--kind', choices=['regular', 'focused', 'quit', 'player', 'laboratory', 'image', 'laboratory-extra'], default='focused')
    parser.add_argument('--focus-only', action='store_true')
    parser.add_argument('--player-only', action='store_true')
    parser.add_argument('--laboratory-only', action='store_true')
    parser.add_argument('--image-only', action='store_true')
    parser.add_argument('--laboratory-extra-only', action='store_true')
    args = parser.parse_args()
    require(sum([args.focus_only, args.player_only, args.laboratory_only, args.image_only, args.laboratory_extra_only]) <= 1, 'Select only one focused journey')
    engine = executable(args.godot, 'godot')
    if args.inside:
        run = local_path(ROOT, args.inside)
        if args.kind == 'regular':
            regular_inside(engine, run)
        elif args.kind == 'player':
            from check_player_controls import player_inside
            player_inside(engine, run)
        elif args.kind == 'laboratory':
            from check_laboratory_controls import laboratory_inside
            laboratory_inside(engine, run)
        elif args.kind == 'image':
            from check_image_export_controls import image_inside
            image_inside(engine, run)
        elif args.kind == 'laboratory-extra':
            from check_laboratory_extra_controls import laboratory_extra_inside
            laboratory_extra_inside(engine, run)
        else:
            focused_inside(engine, run, args.kind == 'quit')
        return
    runs = local_path(ROOT, ROOT / '.workflow-local/crisp-controls')
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='journey-', dir=runs))
    print(json.dumps({'run': str(run)}), flush=True)
    files = [
        'project.godot',
        'bgr.gdextension',
        'ui/main.gd',
        'ui/line_path_presentation.gd',
        'ui/organism_controls.gd',
        'ui/laboratory_window.gd',
        'ui/genome_preview.gd',
        'ui/player_controls_check.gd',
        'tools/check_player_controls.py',
        'tools/check_native_controls.py',
        'tools/build.py',
        'tools/run_reference_oracles.py',
        'ui/world_canvas.gd',
        'ui/gui_check.gd',
        'ui/crisp_check.gd',
        'scenes/main.tscn',
        'bin/libbiogenesis_godot.so',
        '.godot/extension_list.cfg',
    ]
    if args.laboratory_only:
        files.extend(['tools/check_laboratory_controls.py', 'ui/laboratory_controls_check.gd', 'tests/fixtures/java/core.jsonl'])
    if args.image_only:
        files.extend(['tools/check_image_export_controls.py', 'ui/image_export_controls_check.gd', 'ui/laboratory_controls_check.gd', 'crates/godot/src/image_export.rs', 'crates/core/src/atomic_file.rs'])
    if args.laboratory_extra_only:
        files.extend(['tools/check_laboratory_extra_controls.py', 'ui/laboratory_extra_controls_check.gd', 'ui/laboratory_controls_check.gd'])
    pins = {name: sha(ROOT / name) for name in files}
    save(
        run / 'input-manifest.json',
        {
            'runtime_files': pins,
            'engine_sha256': sha(engine),
            'companion_sha256': sha(Path(__file__)),
            'model_injection': False,
        },
    )
    receipts = []
    kinds = ['image'] if args.image_only else (['laboratory'] if args.laboratory_only else (['player'] if args.player_only else (['focused'] if args.focus_only else ['regular', 'focused', 'quit'])))
    if args.laboratory_extra_only:
        kinds = ['laboratory-extra']
    for kind in kinds:
        child = run / kind
        child.mkdir()
        env = {key: os.environ[key] for key in ['LANG', 'LC_ALL', 'TZ'] if key in os.environ}
        for key, name in [
            ('HOME', 'home'),
            ('XDG_CONFIG_HOME', 'config'),
            ('XDG_CACHE_HOME', 'cache'),
            ('XDG_DATA_HOME', 'data'),
            ('TMPDIR', 'tmp'),
            ('XDG_RUNTIME_DIR', 'runtime'),
        ]:
            path = child / name
            path.mkdir(mode=0o700)
            env[key] = str(path)
        env.update(
            PATH='/usr/bin:/bin',
            PYTHONDONTWRITEBYTECODE='1',
            BGR_TEST_ROOT=str(child),
            BGR_GUI_OUTPUT=str(child),
        )
        try:
            run_command(
                [
                    '/usr/bin/xvfb-run',
                    '-a',
                    '-n',
                    {'regular': '480', 'focused': '481', 'quit': '482', 'player': '483', 'laboratory': '484', 'image': '485', 'laboratory-extra': '486'}[kind],
                    '-e',
                    str(child / 'xvfb.log'),
                    '-s',
                    '-screen 0 1280x960x24 -nolisten tcp',
                    sys.executable,
                    str(Path(__file__).resolve()),
                    '--godot',
                    str(engine),
                    '--inside',
                    str(child),
                    '--kind',
                    kind,
                ],
                run,
                receipts,
                ROOT,
                env,
                400 if kind == 'laboratory' else (220 if kind in ['image', 'laboratory-extra'] else 110),
            )
        finally:
            if receipts:
                group = receipts[-1]['process_group']
                for attempt in range(21):
                    try:
                        os.killpg(group, 0)
                    except ProcessLookupError:
                        state = 'absent'
                    except PermissionError:
                        state = 'unknown-permission'
                    else:
                        state = 'present-or-reused'
                    if state == 'absent' or attempt == 20:
                        break
                    time.sleep(0.05)
                save(
                    child / 'owned-group-readback.json',
                    {
                        'process_group': group,
                        'same_native_context': True,
                        'state': state,
                        'bounded_poll_attempts': attempt + 1,
                    },
                )
                require(state == 'absent', 'Owned phase group cleanup is unverified')
    require(
        pins == {name: sha(ROOT / name) for name in files},
        'Runtime changed during native controls',
    )
    groups = []
    for receipt in receipts:
        try:
            os.killpg(receipt['process_group'], 0)
        except ProcessLookupError:
            state = 'absent'
        except PermissionError:
            state = 'unknown-permission'
        else:
            state = 'present-or-reused'
        groups.append(
            {
                'process_group': receipt['process_group'],
                'same_native_context': True,
                'state': state,
            },
        )
    save(run / 'owned-group-readback.json', groups)
    require(all((row['state'] == 'absent' for row in groups)), 'Owned group cleanup is unverified')
    save(
        run / 'summary.json',
        {
            'result': 'PASS',
            'kinds': kinds,
            'runtime_files_preserved': True,
            'groups': groups,
        },
    )
    print(json.dumps({'result': 'PASS', 'run': str(run)}), flush=True)


if __name__ == '__main__':
    signal.signal(signal.SIGTERM, lambda signum, frame: sys.exit(128 + signum))
    try:
        main()
    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
