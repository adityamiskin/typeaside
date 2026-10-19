"""Guest-only E2E driver. Refuses to inject outside our KVM initramfs."""
import json
import os
from pathlib import Path
import re
import shutil
import statistics
import subprocess
import time
import traceback

ARTIFACTS = Path('/mnt')
processes = []
results = {'passed': False, 'assertions': {}, 'runs': []}
isolated = False


def wait_for(predicate, description, timeout=15):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(.02)
    raise TimeoutError(description)


def run(*args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, timeout=10, **kwargs)


def launch(args, name, env=None):
    log = Path(f'/tmp/{name}.log').open('w')
    process = subprocess.Popen(args, stdout=log, stderr=log, env=env)
    processes.append(process)
    return process


def niri(*args):
    return json.loads(run('niri', 'msg', '--json', *args).stdout)


def clipboard():
    return run('wl-paste', '--no-newline', '--type', 'text').stdout.decode()


def log_text():
    return Path('/tmp/app.log').read_text(errors='replace')


def trace(text):
    return [(int(ns), stage) for ns, stage in re.findall(r'^E2E (\d+) (\w+)$', text, re.M)]


def main():
    global isolated
    mounts = Path('/proc/mounts').read_text()
    if not (
        Path('/proc/1/cmdline').read_bytes().startswith(b'/bin/bash\x00/mnt/guest-init.sh')
        and 'rdinit=/init' in Path('/proc/cmdline').read_text()
        and 'QEMU' in Path('/sys/class/dmi/id/sys_vendor').read_text()
        and 'host / 9p ro,' in mounts
        and 'artifacts /mnt 9p rw,' in mounts
        and os.environ.get('XDG_RUNTIME_DIR') == '/run/capture-e2e'
        and not os.environ.get('NIRI_SOCKET')
    ):
        raise RuntimeError('Guest kernel/runtime isolation checks failed')
    isolated = True
    (ARTIFACTS / 'guest-isolation.json').write_text(json.dumps({
        'pid1': Path('/proc/1/cmdline').read_bytes().decode().replace('\0', ' '),
        'cmdline': Path('/proc/cmdline').read_text(), 'mounts': mounts,
        'dmi': Path('/sys/class/dmi/id/sys_vendor').read_text(),
        'boot_id': Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
        'clock': {'realtime_ns': time.time_ns(), 'monotonic_ns': time.monotonic_ns()},
    }, indent=2))

    # Device creation is allowed only after proving a separate kernel and private mounts.
    from evdev import UInput, ecodes
    keyboard = UInput({ecodes.EV_KEY: list(range(1, 249))}, name='Capture E2E keyboard')
    event_reader = keyboard.device
    launch(['udevadm', 'monitor', '--udev', '--property', '--subsystem-match=input'], 'udev-monitor')
    run('udevadm', 'settle')
    (ARTIFACTS / 'niri.kdl').write_text('''
output "Virtual-1" { mode "1280x800"; scale 1; }
prefer-no-csd
hotkey-overlay { skip-at-startup; }
animations { off; }
layout { background-color "#000000"; }
''')
    launch(['niri', '-c', '/mnt/niri.kdl'], 'niri')
    runtime = Path(os.environ['XDG_RUNTIME_DIR'])
    socket = wait_for(lambda: next(runtime.glob('niri.*.sock'), None), 'niri IPC socket')
    os.environ['NIRI_SOCKET'] = str(socket)
    os.environ['WAYLAND_DISPLAY'] = wait_for(lambda: next((p.name for p in runtime.glob('wayland-*') if not p.name.endswith('.lock')), None), 'Wayland socket')
    outputs = niri('outputs')
    assert outputs and any(output['current_mode'] is not None for output in outputs.values()), outputs
    (ARTIFACTS / 'outputs.json').write_text(json.dumps(outputs, indent=2))
    (ARTIFACTS / 'input-devices.txt').write_text(Path('/proc/bus/input/devices').read_text())

    config = Path(os.environ['XDG_CONFIG_HOME']) / 'typeaside'
    config.mkdir(parents=True)
    (config / 'config.json').write_text(json.dumps({
        'onboarded': True, 'launch_at_login': False, 'share_usage': False,
        'sounds': False, 'duck_media': False, 'auto_learn': False,
        'screen_context': False, 'theme': 'dark', 'note_hotkey': 'LEFTSHIFT*2',
    }))
    applications = Path(os.environ['XDG_DATA_HOME']) / 'applications'
    applications.mkdir(parents=True)
    (applications / 'org.typeaside.CaptureFixture.desktop').write_text('[Desktop Entry]\nType=Application\nName=Capture Fixture\nExec=python3 /mnt/fixture.py\n')
    launch(['python3', '/mnt/fixture.py'], 'fixture', dict(os.environ, GSK_RENDERER='cairo', GDK_BACKEND='wayland'))
    window = wait_for(lambda: next((w for w in niri('windows') if w['app_id'] == 'org.typeaside.CaptureFixture'), None), 'GTK fixture window')

    def focus_fixture(selected, index):
        run('niri', 'msg', 'action', 'focus-window', '--id', str(window['id']))
        command = {'selected': selected, 'index': index}
        Path('/tmp/fixture-command.json').write_text(json.dumps(command))
        wait_for(lambda: Path('/tmp/fixture-state.json').exists() and json.loads(Path('/tmp/fixture-state.json').read_text())['command'] == command, 'GTK selection acknowledgement')
        state = json.loads(Path('/tmp/fixture-state.json').read_text())
        assert state['selected'] == selected
        assert niri('focused-window')['id'] == window['id']
        (ARTIFACTS / f'fixture-{index}.json').write_text(json.dumps(state, indent=2))
        time.sleep(.15)
        return state

    focus_fixture(True, -1)
    run('niri', 'msg', 'action', 'screenshot-screen', '--show-pointer', 'false', '--path', '/mnt/fixture-before.png')
    launch(['/mnt/typeaside', '--hidden'], 'app', dict(os.environ, TYPEASIDE_TRACE='1', WAYLAND_DEBUG='client', GALLIUM_DRIVER='llvmpipe', LIBGL_ALWAYS_SOFTWARE='1'))
    wait_for(lambda: 'keyboards_ready' in log_text(), 'app hotkey service', timeout=45)
    time.sleep(1)

    def key(code, value):
        keyboard.write(ecodes.EV_KEY, code, value)
        keyboard.syn()

    def tap(code):
        key(code, 1)
        time.sleep(.035)
        key(code, 0)

    for index, selected in enumerate([True, False, True, False, True, False, True, False]):
        state = focus_fixture(selected, index)
        previous = f'unrelated clipboard sentinel {index}: Ω\nsecond line'
        subprocess.run(['wl-copy', '--type', 'text/plain;charset=utf-8'], input=previous.encode(), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10, check=True)
        assert clipboard() == previous
        offset = len(log_text())
        tap(ecodes.KEY_LEFTSHIFT)
        time.sleep(.075)
        key(ecodes.KEY_LEFTSHIFT, 1)
        time.sleep(.035)
        released = time.time_ns()
        key(ecodes.KEY_LEFTSHIFT, 0)
        written = time.time_ns()
        releases = [e for e in event_reader.read() if e.type == ecodes.EV_KEY and e.code == ecodes.KEY_LEFTSHIFT and e.value == 0]
        release_event = releases[-1]
        release_kernel = release_event.sec * 1_000_000_000 + release_event.usec * 1000
        assert abs(release_kernel - released) < 10_000_000
        release_before = released
        released = release_kernel
        wait_for(lambda: 'E2E_STATE ' in log_text()[offset:], 'capture render after double Left Shift')
        wait_for(lambda: any(stage == 'present' for _, stage in trace(log_text()[offset:])), 'capture surface presentation')
        time.sleep(.4)
        segment = log_text()[offset:]
        observed = json.loads(re.findall(r'^E2E_STATE (.+)$', segment, re.M)[0])
        expected = f'"{state["text"]}"\n' if selected else ''
        phases = trace(segment)
        first = {}
        for ns, stage in phases:
            if stage not in first:
                first[stage] = (ns - released) / 1e6
        render_ns = next(ns for ns, stage in phases if stage == 'render')
        first['present'] = next((ns - released) / 1e6 for ns, stage in phases if stage == 'present' and ns >= render_ns)
        restored = clipboard()
        record = {
            'index': index, 'cold': index == 0, 'selected': selected,
            'release_ns': released, 'state': observed, 'expected': expected,
            'release_write_bounds_ns': [release_before, written],
            'prefill_pass': observed['value'] == expected,
            'header_pass': observed['source'] == ('Capture Fixture' if selected else None),
            'clipboard_before': previous, 'clipboard_after': restored,
            'clipboard_pass': restored == previous,
            'rows_pass': observed['rows'] == 4 and observed['input_height'] == 88,
            'phases_ms': first,
        }
        results['runs'].append(record)
        (ARTIFACTS / f'run-{index}.json').write_text(json.dumps(record, indent=2))
        # This changes only the guest clipboard, after verifying clipboard restoration.
        run('niri', 'msg', 'action', 'screenshot-screen', '--show-pointer', 'false', '--path', f'/mnt/capture-{index}.png')
        wait_for(lambda: (ARTIFACTS / f'capture-{index}.png').exists(), 'guest screenshot')
        # Clear the draft through the real textarea, keeping the same app/injector alive.
        if selected:
            key(ecodes.KEY_LEFTCTRL, 1)
            tap(ecodes.KEY_A)
            key(ecodes.KEY_LEFTCTRL, 0)
            tap(ecodes.KEY_BACKSPACE)
            wait_for(lambda: json.loads(re.findall(r'^E2E_STATE (.+)$', log_text()[offset:], re.M)[-1])['value'] == '', 'cleared capture draft')
        tap(ecodes.KEY_ESC)
        time.sleep(.3)
        (ARTIFACTS / 'app-progress.log').write_text(log_text())

    results['assertions'] = {
        'double_shift_opens': len(results['runs']) == 8,
        'exact_selection_and_readable_header': all(r['prefill_pass'] and r['header_pass'] for r in results['runs'] if r['selected']),
        'no_selection_is_empty': all(r['prefill_pass'] for r in results['runs'] if not r['selected']),
        'clipboard_preserved': all(r['clipboard_pass'] for r in results['runs']),
        'four_empty_rows': all(r['rows_pass'] for r in results['runs'] if not r['selected']),
        'latency_measured': all('present' in r['phases_ms'] for r in results['runs']),
    }
    results['warm_median_ms'] = statistics.median(r['phases_ms']['present'] for r in results['runs'][1:])
    results['passed'] = all(results['assertions'].values())
    (ARTIFACTS / 'guest-input-devices-after.txt').write_text(Path('/proc/bus/input/devices').read_text())
    keyboard.close()


try:
    main()
except Exception:
    if isolated and os.environ.get('NIRI_SOCKET'):
        for command in ('layers', 'windows'):
            (ARTIFACTS / f'failure-{command}.json').write_text(json.dumps(niri(command), indent=2))
        run('niri', 'msg', 'action', 'screenshot-screen', '--show-pointer', 'false', '--path', '/mnt/failure.png')
        (ARTIFACTS / 'libinput-devices.txt').write_text(run('libinput', 'list-devices').stdout.decode())
        (ARTIFACTS / 'udev-input.txt').write_text(run('udevadm', 'info', '--export-db').stdout.decode())
        time.sleep(.2)
    results['error'] = traceback.format_exc()
    traceback.print_exc()
finally:
    if not isolated:
        raise SystemExit('Refusing to run outside the disposable capture VM')
    for process in reversed(processes):
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
    for name in ('niri', 'app', 'fixture', 'udev-monitor'):
        if Path(f'/tmp/{name}.log').exists():
            shutil.copyfile(f'/tmp/{name}.log', ARTIFACTS / f'{name}.log')
    (ARTIFACTS / 'results.json').write_text(json.dumps(results, indent=2))
    print(json.dumps(results, indent=2), flush=True)
