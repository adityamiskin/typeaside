#!/usr/bin/env python3
"""Focused release worker experiments; use e2e-latency.py for app latency.

TYPEASIDE_HOME, TYPEASIDE_LOCAL_DEVICE and VK_DRIVER_FILES scope the worker.
Writes every raw result, including model errors, to JSONL on stdout.
"""
import argparse
import json
import os
from pathlib import Path
import struct
import subprocess
import time
import wave


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('clips', type=Path, nargs='+')
    parser.add_argument('--model', default='parakeet-v3')
    parser.add_argument('--runs', type=int, default=5)
    parser.add_argument('--stream', action='store_true')
    parser.add_argument('--idle', type=float, default=0)
    parser.add_argument('--gap', type=float, default=0, help='Pause between complete dictations')
    parser.add_argument('--cases', action='store_true')
    parser.add_argument('--probes', action='store_true', help='Challenge the already-punctuated skip heuristic')
    args = parser.parse_args()
    if not os.environ.get('TYPEASIDE_HOME'):
        parser.error('Set an isolated TYPEASIDE_HOME')
    started = time.perf_counter_ns()
    worker = subprocess.Popen([args.binary.resolve(), '--local-worker', '--local-model', args.model],
                              stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    try:
        ready = json.loads(worker.stdout.readline())
        assert ready == {'Ok': 'ready'}, ready
        print(json.dumps(dict(stage='load', ms=(time.perf_counter_ns() - started) / 1e6)), flush=True)

        def request(payload):
            worker.stdin.write(payload)
            worker.stdin.flush()
            result = json.loads(worker.stdout.readline())
            return result

        def clean(text, styling='semi-formal', email=False):
            data = json.dumps([text, dict(styling=styling, email=email)]).encode()
            return request(bytes([5]) + struct.pack('<Q', len(data)) + data)

        for clip in args.clips:
            with wave.open(str(clip)) as audio:
                assert audio.getframerate() == 16000 and audio.getnchannels() == 1
                pcm = audio.readframes(audio.getnframes())
            floats = struct.pack(f'<{len(pcm)//2}f', *(v / 32768 for (v,) in struct.iter_unpack('<h', pcm)))
            payload = bytes([0]) + struct.pack('<Q', len(floats) // 4) + floats
            for iteration in range(args.runs + 1):
                if args.idle:
                    time.sleep(args.idle)
                gpu_power = {}
                if os.environ.get('TYPEASIDE_LOCAL_DEVICE') == 'gpu':
                    for pci in Path('/sys/bus/pci/devices').iterdir():
                        if ((pci / 'vendor').read_text().strip() == '0x10de'
                                and (pci / 'class').read_text().strip().startswith('0x03')):
                            gpu_power[pci.name] = (pci / 'power/runtime_status').read_text().strip()
                start = time.perf_counter_ns()
                if args.stream:
                    worker.stdin.write(bytes([1]))
                    worker.stdin.flush()
                    for offset in range(0, len(floats), 2560 * 4):
                        block = floats[offset:offset + 2560 * 4]
                        worker.stdin.write(bytes([2]) + struct.pack('<Q', len(block) // 4) + block)
                        worker.stdin.flush()
                        time.sleep(len(block) / 4 / 16000)
                    start = time.perf_counter_ns()
                    result = request(bytes([3]))
                else:
                    result = request(payload)
                stt_ms = (time.perf_counter_ns() - start) / 1e6
                text = result.get('Ok')
                start = time.perf_counter_ns()
                cleaned = clean(text) if text else None
                cleanup_ms = (time.perf_counter_ns() - start) / 1e6
                print(json.dumps(dict(clip=clip.name, iteration=iteration, stt_ms=stt_ms,
                                      cleanup_ms=cleanup_ms, speech=result, cleanup=cleaned,
                                      gpu_power_before_request=gpu_power)), flush=True)
                if args.gap:
                    time.sleep(args.gap)
        if args.cases:
            # Keep the inputs and styles identical to the production example.
            import re
            source = Path('examples/local_cleanup.rs').read_text().split('fn main()')[0]
            cases = re.findall(r'"([^"\n]+)"\s*,\s*Tone::(\w+)\s*,\s*(None|Some\("[^"]+"\))', source)
            assert len(cases) == 12, len(cases)
            for iteration in range(args.runs):
                for index, (text, tone, app) in enumerate(cases):
                    styling = {'Casual': 'semi-formal', 'VeryCasual': 'casual', 'Formal': 'formal'}[tone]
                    start = time.perf_counter_ns()
                    result = clean(text, styling, app != 'None')
                    print(json.dumps(dict(case=index + 1, iteration=iteration, raw=text,
                                          cleanup=result, ms=(time.perf_counter_ns() - start) / 1e6)), flush=True)
        if args.probes:
            for iteration in range(args.runs):
                for text in ['Please send it to john.', 'He go home.', 'We was late.',
                             'I can can do it.', 'Monday is monday.']:
                    start = time.perf_counter_ns()
                    result = clean(text)
                    print(json.dumps(dict(probe=text, iteration=iteration, cleanup=result,
                                          ms=(time.perf_counter_ns() - start) / 1e6)), flush=True)
    finally:
        worker.stdin.close()
        try:
            worker.wait(timeout=10)
        except subprocess.TimeoutExpired:
            worker.kill()
            worker.wait()


if __name__ == '__main__':
    main()
