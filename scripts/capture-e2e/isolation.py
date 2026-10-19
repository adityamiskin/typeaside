"""Read-only checks of the live session. Never stores clipboard contents or window titles."""
import hashlib
import json
from pathlib import Path
import subprocess
import time


def digest(command):
    result = subprocess.run(command, capture_output=True, timeout=3)
    return {'status': result.returncode, 'sha256': hashlib.sha256(result.stdout).hexdigest()}


def snapshot():
    focus = subprocess.run(['niri', 'msg', '--json', 'focused-window'], capture_output=True, timeout=3)
    window = json.loads(focus.stdout) if focus.returncode == 0 else None
    services = {}
    for entry in Path('/proc').glob('[0-9]*'):
        try:
            if (entry / 'comm').read_text().strip() == 'typeaside':
                services[entry.name] = (entry / 'stat').read_text().split(') ')[1].split()[19]
        except (OSError, ProcessLookupError):
            pass
    return {
        'time_ns': time.time_ns(),
        'boot_id': Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
        'clipboard': digest(['wl-paste', '--no-newline', '--type', 'text']),
        'primary': digest(['wl-paste', '--primary', '--no-newline', '--type', 'text']),
        'focus': {'status': focus.returncode, 'id': window.get('id') if window else None, 'app_id': window.get('app_id') if window else None},
        'typeaside_process_start_ticks': services,
        'input_devices': Path('/proc/bus/input/devices').read_text(),
    }


def report(before, after):
    return {
        'before': before, 'after': after,
        'clipboard_unchanged': before['clipboard'] == after['clipboard'],
        'primary_unchanged': before['primary'] == after['primary'],
        'focus_unchanged': before['focus'] == after['focus'],
        'live_typeaside_processes_unchanged': before['typeaside_process_start_ticks'] == after['typeaside_process_start_ticks'],
        'host_input_devices_unchanged': before['input_devices'] == after['input_devices'],
        'no_test_keyboard_on_host': 'Capture E2E keyboard' not in after['input_devices'],
    }
