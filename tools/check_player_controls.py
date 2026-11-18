"""Exercise real organism controls through the owned native frontend"""

import json
import math
import os
import subprocess
import time

from build import ROOT, godot_errors
from check_native_controls import NativeInput


def player_inside(engine, run):
    def require(value, message):
        if not value:
            raise ValueError(message)

    def save(path, value):
        path.write_text(json.dumps(value, indent=2) + '\n')

    require(os.environ.get('BGR_TEST_ROOT') == str(run) and os.environ.get('DISPLAY', '').startswith(':'),
            'Owned display is required')
    log = run / 'engine.log'
    actions, checks = [], []
    native = None
    with log.open('wb') as output:
        process = subprocess.Popen([str(engine), '--audio-driver', 'Dummy', '--path', str(ROOT),
                                    '--script', 'res://ui/player_controls_check.gd', '--resolution', '1200x900'],
                                   stdout=output, stderr=subprocess.STDOUT)

        def lines(prefix):
            text = log.read_text(errors='replace')
            require('BGR_GUI_FAIL' not in text and not godot_errors(text), 'Frontend error: ' + str(log))
            return [json.loads(line[len(prefix) + 1:]) for line in text.splitlines()
                    if line.startswith(prefix + ' ')]

        def latest():
            rows = lines('BGR_CRISP_OBS')
            return rows[-1] if rows else None

        def wait(description, predicate, timeout=10):
            until = time.monotonic() + timeout
            while time.monotonic() < until:
                observed = latest()
                if observed and predicate(observed):
                    return observed
                require(process.poll() is None, 'Frontend exited before ' + description)
                time.sleep(0.01)
            raise ValueError('Timed out ' + description + '; latest=' + repr(latest()))

        def checkpoint(label):
            request = run / 'checkpoint-request.pending'
            save(request, {'label': label})
            request.replace(run / 'checkpoint-request.json')
            until = time.monotonic() + 10
            while time.monotonic() < until:
                for observed in lines('BGR_CRISP_CHECKPOINT'):
                    if observed['label'] == label:
                        checks.append(observed)
                        return observed
                require(process.poll() is None, 'Frontend exited before ' + label)
                time.sleep(0.01)
            raise ValueError('Missing checkpoint ' + label)

        def key(name):
            native.key(name)
            actions.append({'key': name, 'monotonic_ns': time.monotonic_ns()})

        def click(name):
            native.click(latest(), name)
            actions.append({'control': name, 'monotonic_ns': time.monotonic_ns()})

        def point(position, button=1):
            observed = latest()
            cx, cy, _, _ = observed['controls']['WorldCanvas']
            wx, wy = observed['window_position']
            ox, oy = observed['offset']
            zoom = observed['zoom']
            x, y = wx + cx + ox + position[0] * zoom, wy + cy + oy + position[1] * zoom
            native.motion(native.display, -1, round(x), round(y), 0)
            native.button(native.display, button, 1, 0)
            native.button(native.display, button, 0, 0)
            native.flush(native.display)
            actions.append({'world_point': position, 'screen_point': [x, y], 'button': button,
                            'monotonic_ns': time.monotonic_ns()})

        def pick(identity=None, skip=None):
            for candidate in latest()['organism_points']:
                if candidate['id'] == skip or (identity is not None and candidate['id'] != identity):
                    continue
                position = [coordinate + 0.5 for coordinate in candidate['position']]
                if skip is not None or identity is not None:
                    for _ in range(12):
                        previous_zoom = latest()['zoom']
                        if previous_zoom >= 2.0:
                            break
                        point(position, 4)
                        wait('real pointer zoom for one-pixel organism', lambda row: row['zoom'] > previous_zoom)
                    require(latest()['zoom'] >= 2.0, 'Cannot magnify the selectable source pixel')
                point(position)
                return wait('actual organism selection', lambda row: row['selected'] and
                            row['selected']['id'] == candidate['id'])
            raise ValueError('Requested actual organism is absent')

        def empty_point(button=1):
            observed = latest()
            _, _, width, height = observed['controls']['WorldCanvas']
            offset = observed['offset']
            zoom = observed['zoom']
            for x, y in [(8, 8), (width - 8, 8), (8, height - 8), (width - 8, height - 8)]:
                if all((x - offset[0] - item['position'][0] * zoom) ** 2 +
                       (y - offset[1] - item['position'][1] * zoom) ** 2 > 4096
                       for item in observed['organism_points']):
                    point([(x - offset[0]) / zoom, (y - offset[1]) / zoom], button)
                    return
            raise ValueError('No bounded empty viewport corner was found')

        def world_action(command, predicate):
            click('GlobalActionsMenu')
            observed = wait('global actions popup', lambda row: row['global_actions_menu'])
            count = observed['global_actions_item_count']
            require(0 < count <= 16, 'Global actions menu exceeds bounded native traversal')
            for _ in range(count + 1):
                previous = latest()['global_actions_focused_id']
                if previous == command:
                    break
                key('Down')
                wait('actual menu focus', lambda row: row['global_actions_menu'] and row['global_actions_focused_id'] != previous)
            require(latest()['global_actions_focused_id'] == command, 'Cannot focus requested actual menu item')
            key('Return')
            return wait('world action ' + str(command), lambda row: not row['global_actions_menu'] and predicate(row))

        try:
            initial = wait('initial actual world', lambda row: row['paused'] and row['tick'] == 0 and row['same_initial'])
            native = NativeInput()
            window = native.find_window('Biogenesis-R')
            native.focus(native.display, window, 2, 0)
            native.flush(native.display)
            checkpoint('player_initial')
            selected = pick()
            parent_id = selected['selected']['id']
            checkpoint('player_selected')
            before = latest()
            amount = min(10.0, before['carbon_dioxide'])
            click('FeedOrganismButton')
            fed = wait('source feed', lambda row: row['selected']['energy'] == before['selected']['energy'] + amount)
            require(fed['carbon_dioxide'] == before['carbon_dioxide'] + amount and
                    fed['oxygen'] == before['oxygen'] + amount and fed['tick'] == 0, 'Feed source gas accounting differs')
            click('WeakenOrganismButton')
            half = fed['selected']['energy'] / 2.0
            weakened = wait('source weaken', lambda row: row['selected']['energy'] == fed['selected']['energy'] - half)
            require(weakened['oxygen'] == fed['oxygen'] - half and
                    weakened['carbon_dioxide'] == fed['carbon_dioxide'] + half, 'Weaken source gas accounting differs')
            feeds = max(0, math.ceil((weakened['selected']['reproduce_energy'] + 200.0 - weakened['selected']['energy']) / 10.0))
            require(feeds <= 256, 'Selected organism exceeds bounded native reproduction preparation')
            for _ in range(feeds):
                energy = latest()['selected']['energy']
                click('FeedOrganismButton')
                wait('real feed preparation', lambda row: row['selected']['energy'] == energy + 10.0)
            before = latest()
            click('ReproduceOrganismButton')
            children_before_reproduction = before['selected']['children']
            reproduced = wait('actual selected reproduction', lambda row: row['selected']['children'] > before['selected']['children'])
            require(reproduced['population'] > before['population'] and reproduced['tick'] == 0, 'Reproduction did not insert a real paused-world child')
            checkpoint('player_reproduced')
            click('TrackOrganismButton')
            tracked = wait('track selected identity', lambda row: row['tracking'] and row['selected_is_tracked'])
            require(tracked['offset'] == reproduced['offset'] and not tracked['tracking_path'], 'Paused tracking moved the camera')
            second = pick(skip=parent_id)
            require(second['tracking'] and not second['selected_is_tracked'], 'Selection changed tracked identity')
            second_id = second['selected']['id']
            empty_point()
            wait('empty left click clears only selection', lambda row: not row['selected'] and row['tracking'])
            pick(identity=second_id)
            empty_point(3)
            time.sleep(0.12)
            require(latest()['selected']['id'] == second_id and not latest()['context_menu'], 'Empty right click opened another organism menu')
            wait('empty-world genotype menu', lambda row: row['world_context'])
            checkpoint('player_world_context')
            key('Escape')
            wait('Escape closes empty-world menu', lambda row: not row['world_context'])
            require(process.poll() is None, 'World popup Escape quit the application')
            point([coordinate + 0.5 for coordinate in next(row['position'] for row in latest()['organism_points'] if row['id'] == second_id)], 3)
            wait('organism context popup', lambda row: row['context_menu'])
            checkpoint('player_context')
            key('Escape')
            wait('Escape closes popup', lambda row: not row['context_menu'])
            require(process.poll() is None, 'Popup Escape quit the application')
            click('CrispLines')
            wait('actual crisp selection rendering', lambda row: row['checkbox'] and row['canvas_crisp'])
            checkpoint('player_selected_crisp')
            click('CrispLines')
            wait('restore smooth rendering', lambda row: not row['checkbox'] and not row['canvas_crisp'])
            click('KillOrganismButton')
            killed = wait('kill selected', lambda row: not row['selected']['alive'])
            require(killed['tracking'], 'Killing another organism cleared independent tracking')
            checkpoint('player_corpse')
            click('ReviveOrganismButton')
            wait('revive selected', lambda row: row['selected']['alive'] and row['selected']['age_ticks'] == 1)
            click('RejuvenateOrganismButton')
            wait('rejuvenate selected', lambda row: row['selected']['age_ticks'] == 0)
            checkpoint('player_revived')
            click('KillOrganismButton')
            wait('second kill', lambda row: not row['selected']['alive'])
            click('DisperseOrganismButton')
            wait('disperse corpse', lambda row: row['selected']['energy'] == 0.0)
            click('StepButton')
            wait('actual frame removes selected corpse', lambda row: row['tick'] == 1 and not row['selected'])
            checkpoint('player_removed')
            before = latest()
            world_action(6, lambda row: row['carbon_dioxide'] == before['carbon_dioxide'] + 1000.0)
            world_action(8, lambda row: row['methane'] == before['methane'] + 1000.0)
            world_action(7, lambda row: row['carbon_dioxide'] == (before['carbon_dioxide'] + 1000.0) - 1000.0)
            world_action(9, lambda row: row['methane'] == (before['methane'] + 1000.0) - 1000.0)
            checkpoint('player_world_gases')
            native.resize(native.display, window, 800, 660)
            native.flush(native.display)
            wait('minimum window', lambda row: row['window_size'] == [800, 660])
            click('FitButton')
            wait('fit after resize', lambda row: row['fit'])
            selected = pick(identity=parent_id)
            require(selected['tracking'] and selected['selected_is_tracked'], 'Original tracked identity was lost')
            checkpoint('player_minimum_selected')
            click('KillOrganismButton')
            wait('tracked death clears camera', lambda row: not row['selected']['alive'] and not row['tracking'])
            checkpoint('player_minimum_corpse')
            world_action(4, lambda row: row['population'] == 0)
            world_action(5, lambda row: row['selected']['energy'] == 0.0)
            click('StepButton')
            wait('all corpses removed', lambda row: row['tick'] == 2 and not row['selected'] and not row['organism_points'])
            checkpoint('player_empty')
            key('r')
            wait('actual reset confirmation', lambda row: row['decision_visible'] and row['decision_controls_ready'] and row['decision'] == 'reset')
            click('ConfirmWorldDecision')
            wait('reset real world', lambda row: row['same_initial'] and not row['selected'] and not row['tracking'])
            checkpoint('player_reset')
            key('Escape')
            wait('actual exit confirmation', lambda row: row['decision_visible'] and row['decision_controls_ready'] and row['decision'] == 'quit')
            click('ConfirmWorldDecision')
            require(process.wait(timeout=5) == 0, 'Native exit failed')
            require(not godot_errors(log.read_text(errors='replace')), 'Late frontend error')
            save(run / 'summary.json', {'result': 'PASS', 'kind': 'player_controls', 'checks': checks,
                                       'actions': len(actions), 'real_feed_preparation': feeds,
                                       'real_reproduction_children': reproduced['selected']['children'] - children_before_reproduction,
                                       'world_state_injection': False})
            print(json.dumps({'result': 'PASS', 'player_checkpoints': len(checks)}), flush=True)
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
