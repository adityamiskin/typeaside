#!/usr/bin/env python3
"""Quick Capture E2E in a disposable VM. See scripts/capture-e2e/README.md."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import sys

sys.dont_write_bytecode = True


def load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).parent / 'capture-e2e' / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ref', default='HEAD', help='Committed product revision to build')
    parser.add_argument('--artifacts', type=Path)
    parser.add_argument('--target-dir', type=Path, default=Path('/home/aditya/Projects/typeaside/target'))
    parser.add_argument('--lavapipe', type=Path, default=Path('/tmp/typeaside-e2e/vulkan/usr/lib/libvulkan_lvp.so'))
    args = parser.parse_args()
    artifacts = (args.artifacts or Path(tempfile.mkdtemp(prefix='capture-e2e-'))).resolve()
    artifacts.mkdir(parents=True, exist_ok=True)
    if not artifacts.is_relative_to(Path('/tmp').resolve()):
        raise RuntimeError('Artifacts must be stored under /tmp')
    print(f'Artifacts: {artifacts}', flush=True)
    revision = subprocess.check_output(['git', 'rev-parse', args.ref], text=True).strip()
    (artifacts / 'revision.txt').write_text(revision + '\n')
    if not args.lavapipe.is_file():
        raise RuntimeError('Pass --lavapipe pointing to libvulkan_lvp.so (Mesa lavapipe)')
    source = artifacts / 'source'
    source.mkdir()
    with (artifacts / 'source.tar').open('wb') as output:
        subprocess.run(['git', 'archive', revision], stdout=output, check=True)
    subprocess.run(['tar', '-xf', artifacts / 'source.tar', '-C', source], check=True)
    load('trace').instrument(source, artifacts)
    environment = dict(os.environ, CARGO_BUILD_JOBS='6', CARGO_TARGET_DIR=str(args.target_dir))
    with (artifacts / 'build.log').open('w') as output:
        subprocess.run(['cargo', 'build', '--locked', '--offline'], cwd=source, env=environment, stdout=output, stderr=subprocess.STDOUT, check=True)
    shutil.copy2(args.target_dir / 'debug/typeaside', artifacts / 'typeaside')
    (artifacts / 'binary.sha256').write_text(hashlib.sha256((artifacts / 'typeaside').read_bytes()).hexdigest() + '\n')
    for name in ('guest-init.sh', 'guest.py', 'fixture.py'):
        shutil.copy2(Path(__file__).parent / 'capture-e2e' / name, artifacts / name)
    shutil.copy2(args.lavapipe, artifacts / 'libvulkan_lvp.so')
    (artifacts / 'lvp_icd.json').write_text(json.dumps({'ICD': {'api_version': '1.4.354', 'library_path': '/mnt/libvulkan_lvp.so'}, 'file_format_version': '1.0.0'}))
    command = load('vm').prepare(artifacts)
    (artifacts / 'qemu-command.json').write_text(json.dumps(command, indent=2))
    isolation = load('isolation')
    before = isolation.snapshot()
    with (artifacts / 'console.log').open('w') as output:
        process = subprocess.Popen(command, stdout=output, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 240
            while True:
                if 'reboot: Power down' in (artifacts / 'console.log').read_text(errors='replace'):
                    break
                if process.poll() is not None:
                    raise RuntimeError(f'QEMU exited unexpectedly: {process.returncode}')
                if time.monotonic() >= deadline:
                    raise TimeoutError('Guest did not finish within 240 seconds')
                time.sleep(.1)
        finally:
            process.kill()
            process.wait()
            (artifacts / 'live-isolation.json').write_text(json.dumps(isolation.report(before, isolation.snapshot()), indent=2))
    result = json.loads((artifacts / 'results.json').read_text())
    proof = json.loads((artifacts / 'live-isolation.json').read_text())
    result['isolation'] = {key: value for key, value in proof.items() if key not in ('before', 'after')}
    guest = json.loads((artifacts / 'guest-isolation.json').read_text())
    result['isolation']['different_kernel_boot_id'] = guest['boot_id'] != before['boot_id']
    report = load('report')
    report.verify_screenshots(result, artifacts)
    result['passed'] = result['passed'] and all(result['assertions'].values()) and all(result['isolation'].values())
    (artifacts / 'results.json').write_text(json.dumps(result, indent=2))
    report.write(result, artifacts, revision)
    print((artifacts / 'report.txt').read_text())
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
