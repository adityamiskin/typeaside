#!/usr/bin/env python3
"""Compare prefix caching with interleaved warm workers, reversing order per pair."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('--artifacts', type=Path, required=True)
    parser.add_argument('--runs', type=int, default=5)
    args = parser.parse_args()
    if not os.environ.get('TYPEASIDE_HOME'):
        parser.error('Set an isolated TYPEASIDE_HOME')
    args.artifacts.mkdir(parents=True, exist_ok=True)
    source = Path('examples/local_cleanup.rs').read_text().split('fn main()')[0]
    cases = re.findall(r'"([^"\n]+)"\s*,\s*Tone::(\w+)\s*,\s*(None|Some\("[^"]+"\))', source)
    assert len(cases) == 12
    workers = {}
    logs = []

    def clean(worker, text, styling='semi-formal', email=False):
        payload = json.dumps([text, dict(styling=styling, email=email)]).encode()
        worker.stdin.write(bytes([5]) + len(payload).to_bytes(8, 'little') + payload)
        worker.stdin.flush()
        return json.loads(worker.stdout.readline())

    try:
        for name, cache in [('baseline', '0'), ('cache', '1')]:
            log = (args.artifacts / (name + '.log')).open('w')
            logs.append(log)
            worker = subprocess.Popen([args.binary.resolve(), '--local-worker', '--local-model', 'parakeet-v3'],
                                      env=dict(os.environ, TA_CACHE=cache), stdin=subprocess.PIPE,
                                      stdout=subprocess.PIPE, stderr=log)
            workers[name] = worker
            assert json.loads(worker.stdout.readline()) == {'Ok': 'ready'}
            assert 'Ok' in clean(worker, cases[11][0])
        time.sleep(1)
        for iteration in range(args.runs):
            for index, (text, tone, app) in enumerate(cases):
                styling = {'Casual': 'semi-formal', 'VeryCasual': 'casual', 'Formal': 'formal'}[tone]
                row = dict(case=index + 1, iteration=iteration, raw=text)
                order = ['baseline', 'cache'] if (index + iteration) % 2 == 0 else ['cache', 'baseline']
                for name in order:
                    started = time.perf_counter_ns()
                    row[name] = clean(workers[name], text, styling, app != 'None')
                    row[name + '_ms'] = (time.perf_counter_ns() - started) / 1e6
                print(json.dumps(row), flush=True)
    finally:
        for worker in workers.values():
            worker.stdin.close()
        for worker in workers.values():
            try:
                worker.wait(timeout=10)
            except subprocess.TimeoutExpired:
                worker.kill()
                worker.wait()
        for log in logs:
            log.close()


if __name__ == '__main__':
    main()
