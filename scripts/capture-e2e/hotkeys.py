"""Guest-only hotkey listener E2E: idle wakeups, hot-plug, unplug. Refuses to run outside the KVM guest."""
import json
import os
from pathlib import Path
import re
import subprocess
import time
import traceback

ARTIFACTS = Path('/mnt')
processes = []
results = {'passed': False, 'assertions': {}}
isolated = False


def wait_for(predicate, description, timeout=15):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if result := predicate():
            return result
        time.sleep(.02)
    raise TimeoutError(description)


def launch(args, name, env=None):
    log = Path(f'/tmp/{name}.log').open('w')
    process = subprocess.Popen(args, stdout=log, stderr=log, env=env)
    processes.append(process)
    return process


def stages():
    return re.findall(r'^E2E \d+ (\w+)$', Path('/tmp/app.log').read_text(errors='replace'), re.M)


try:
    mounts = Path('/proc/mounts').read_text()
    if not (
        'rdinit=/init' in Path('/proc/cmdline').read_text()
        and 'QEMU' in Path('/sys/class/dmi/id/sys_vendor').read_text()
        and 'host / 9p ro,' in mounts
        and 'artifacts /mnt 9p rw,' in mounts
        and os.environ.get('XDG_RUNTIME_DIR') == '/run/capture-e2e'
    ):
        raise RuntimeError('Guest kernel/runtime isolation checks failed')
    isolated = True
    (ARTIFACTS / 'guest-isolation.json').write_text(json.dumps({
        'boot_id': Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
    }))

    from evdev import UInput, ecodes
    (ARTIFACTS / 'niri.kdl').write_text('hotkey-overlay { skip-at-startup; }\nanimations { off; }\n')
    launch(['niri', '-c', '/mnt/niri.kdl'], 'niri')
    runtime = Path(os.environ['XDG_RUNTIME_DIR'])
    os.environ['WAYLAND_DISPLAY'] = wait_for(lambda: next((p.name for p in runtime.glob('wayland-*') if not p.name.endswith('.lock')), None), 'Wayland socket')
    config = Path(os.environ['XDG_CONFIG_HOME']) / 'typeaside'
    config.mkdir(parents=True)
    (config / 'config.json').write_text(json.dumps({
        'onboarded': True, 'launch_at_login': False, 'share_usage': False, 'sounds': False,
        'duck_media': False, 'auto_learn': False, 'screen_context': False, 'note_hotkey': 'LEFTSHIFT*2',
    }))
    app = launch(['/mnt/typeaside', '--hidden'], 'app', dict(os.environ, GALLIUM_DRIVER='llvmpipe', LIBGL_ALWAYS_SOFTWARE='1'))
    wait_for(lambda: 'keyboards_ready' in stages(), 'app hotkey service', timeout=45)
    listener = wait_for(lambda: next((t for t in Path(f'/proc/{app.pid}/task').iterdir() if (t / 'comm').read_text().startswith('typeaside-hotke')), None), 'hotkey thread')

    def wakeups_per_second(seconds):
        def switches():
            status = (listener / 'status').read_text()
            return sum(int(n) for n in re.findall(r'^(?:non)?voluntary_ctxt_switches:\s+(\d+)', status, re.M))
        before = switches()
        time.sleep(seconds)
        return (switches() - before) / seconds

    def plug(name):
        keyboard = UInput({ecodes.EV_KEY: list(range(1, 249))}, name=name)
        subprocess.run(['udevadm', 'settle'], check=True, timeout=10)
        time.sleep(.3)
        return keyboard

    def double_tap(keyboard):
        for _ in range(2):
            keyboard.write(ecodes.EV_KEY, ecodes.KEY_LEFTSHIFT, 1)
            keyboard.syn()
            time.sleep(.035)
            keyboard.write(ecodes.EV_KEY, ecodes.KEY_LEFTSHIFT, 0)
            keyboard.syn()
            time.sleep(.075)

    def double_tap_opens_capture(keyboard):
        opened = stages().count('hotkey')
        double_tap(keyboard)
        try:
            wait_for(lambda: stages().count('hotkey') > opened, 'capture opened', timeout=3)
        except TimeoutError:
            return False
        # The same shortcut toggles capture closed; Escape only reaches hotkeys while recording.
        time.sleep(.5)
        double_tap(keyboard)
        time.sleep(1)
        return True

    time.sleep(2)
    results['idle_wakeups_per_s'] = wakeups_per_second(10)
    first = plug('Capture E2E keyboard hotplug 1')
    results['hotplugged_keyboard_works'] = double_tap_opens_capture(first)
    first.close()
    time.sleep(.5)
    results['after_unplug_wakeups_per_s'] = wakeups_per_second(5)
    second = plug('Capture E2E keyboard hotplug 2')
    results['replugged_keyboard_works'] = double_tap_opens_capture(second)
    second.close()
    results['assertions'] = {
        'idle_listener_sleeps': results['idle_wakeups_per_s'] < 0.5,
        'hotplug_detected': results['hotplugged_keyboard_works'],
        'unplug_does_not_spin': results['after_unplug_wakeups_per_s'] < 0.5,
        'replug_detected': results['replugged_keyboard_works'],
        'app_alive': app.poll() is None,
    }
    results['passed'] = all(results['assertions'].values())
except Exception:
    results['error'] = traceback.format_exc()
finally:
    if not isolated:
        raise SystemExit('Refusing to run outside the disposable capture VM')
    for process in reversed(processes):
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
    for name in ('niri', 'app'):
        if Path(f'/tmp/{name}.log').exists():
            (ARTIFACTS / f'{name}.log').write_bytes(Path(f'/tmp/{name}.log').read_bytes())
    (ARTIFACTS / 'results.json').write_text(json.dumps(results, indent=2))
