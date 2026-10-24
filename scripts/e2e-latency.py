#!/usr/bin/env python3
"""Release latency through the app, with delivery redirected in an exported build.

prepare exports HEAD and instruments it; run drives only that binary and runtime.
No production injection code is enabled in the instrumented delivery callback.
"""
import argparse
import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import time


SOURCES = {'local': 'local', 'key': 'ownkey'}


def command(*args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def prepare(args):
    source = args.scratch / 'source'
    source.mkdir(parents=True, exist_ok=False)
    archive = args.scratch / 'source.tar'
    with archive.open('wb') as out:
        command('git', 'archive', args.ref, stdout=out)
    command('tar', '-xf', str(archive), '-C', str(source))
    patches = []

    def edit(name, old, new):
        path = source / name
        before = path.read_text()
        if before.count(old) != 1:
            raise RuntimeError(f'Expected one instrumentation anchor: {name}: {old}')
        after = before.replace(old, new)
        path.write_text(after)
        patches.extend(difflib.unified_diff(before.splitlines(True), after.splitlines(True), name, name))

    path = source / 'src/engine.rs'
    text = path.read_text()
    start = text.index('                                if dictation.mode == Mode::File {')
    end = text.index('\n                            },', start)
    edit('src/engine.rs', text[start:end], '''                                let delivered_ns = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
                                let value = serde_json::json!({"delivered_ns": delivered_ns, "dictation": dictation});
                                let path = std::path::PathBuf::from(std::env::var("TYPEASIDE_LATENCY_OUTPUT").unwrap());
                                let temporary = path.with_extension("pending");
                                std::fs::write(&temporary, value.to_string()).unwrap();
                                std::fs::rename(temporary, path).unwrap();''')
    edit('src/engine.rs', 'release_tail =\n                            Some(ReleaseTail::new(Instant::now(), capture.sample_count()));', '''eprintln!("LATENCY release {}", SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
                        release_tail =
                            Some(ReleaseTail::new(Instant::now(), capture.sample_count()));''')
    edit('src/engine.rs', 'let held_samples = release_tail.take().unwrap().samples;', '''eprintln!("LATENCY tail_end {}", SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
                    let held_samples = release_tail.take().unwrap().samples;''')
    edit('src/engine.rs', '    let tx = tx.clone();\n    match HotkeyService::start', '''    if std::env::var_os("TYPEASIDE_LATENCY_OUTPUT").is_some() { return None; }
    let tx = tx.clone();
    match HotkeyService::start''')
    edit('src/local/mod.rs', '''        prewarm(config);
    } else {
        let _ = sender().send(Request::Unload(None));''', '''        if std::env::var("TYPEASIDE_LATENCY_COLD").as_deref() != Ok("1") {
            prewarm(config);
        }
    } else {
        let _ = sender().send(Request::Unload(None));''')
    edit('src/local/mod.rs', '''        let mut child = Command::new(executable)
            .arg("--local-worker")''', '''        let mut child = Command::new(executable)
            .envs(std::env::var_os("TYPEASIDE_LATENCY_WORKER_ICD").into_iter().flat_map(|icd| [
                ("VK_DRIVER_FILES", icd),
                ("VK_LOADER_DRIVERS_DISABLE", "".into()),
                ("VK_LOADER_DRIVERS_SELECT", "".into()),
            ]))
            .arg("--local-worker")''')
    if args.experiments:
        edit('src/local/cleanup.rs', 'llama_memory_clear(memory, true);', '''llama_memory_clear(memory, std::env::var("TA_CLEAR_DATA").as_deref() != Ok("0"));''')
        edit('src/local/mod.rs', 'mod vad;', 'pub(crate) mod vad;')
        edit('src/local/vad.rs', 'pub(super) struct Vad', 'pub(crate) struct Vad')
        edit('src/audio.rs', '    pub fn stop(mut self, keep_ready: bool)', '''    pub(crate) fn latency_snapshot(&self) -> Option<Vec<f32>> {
        let data = self.buffer.lock().ok()?.clone();
        Some(resample(&data, self.sample_rate))
    }

    pub fn stop(mut self, keep_ready: bool)''')
        edit('src/engine.rs', 'let mut release_tail: Option<ReleaseTail> = None;', '''let mut release_tail: Option<ReleaseTail> = None;
    let mut latency_vad: Option<crate::local::vad::Vad> = None;''')
        edit('src/engine.rs', 'Some(ReleaseTail::new(Instant::now(), capture.sample_count()));', '''Some({
                                let released = Instant::now();
                                let mut tail = ReleaseTail::new(released, capture.sample_count());
                                if std::env::var("TA_VAD_TAIL").as_deref() == Ok("1") {
                                    let started = Instant::now();
                                    if latency_vad.is_none() {
                                        latency_vad = config.local_model.directory().and_then(|dir| {
                                            crate::local::vad::Vad::load(&dir.join("silero-vad-v6.2-F32.gguf"))
                                        }).ok();
                                    }
                                    let quiet = capture.latency_snapshot().and_then(|samples| {
                                        let ranges = latency_vad.as_mut()?.chunks(&samples).ok()?;
                                        Some(ranges.last().is_some_and(|range| range.end < samples.len()))
                                    }).unwrap_or(false);
                                    if quiet { tail.deadline = released; }
                                    eprintln!("LATENCY vad_gate quiet={} elapsed_us={}", quiet, started.elapsed().as_micros());
                                }
                                tail
                            });''')
        edit('src/local/cleanup.rs', 'let input_chars = raw.chars().count().max(1) as f64;', '''let input_chars = if std::env::var("TA_LENGTH_GUARD").as_deref() == Ok("0") {
                f64::INFINITY
            } else { raw.chars().count().max(1) as f64 };''')
        edit('src/local/cleanup.rs', 'llama_log_set(Some(discard_log), std::ptr::null_mut());', '''if std::env::var("TA_LLAMA_LOG").as_deref() == Ok("1") {
                llama_log_set(None, std::ptr::null_mut());
            } else { llama_log_set(Some(discard_log), std::ptr::null_mut()); }''')
        edit('src/local/cleanup.rs', 'params.n_threads = 4;', '''params.n_threads = std::env::var("TA_THREADS").ok().and_then(|s| s.parse().ok()).unwrap_or(4);''')
        edit('src/local/cleanup.rs', 'params.n_threads_batch = 4;', '''params.n_threads_batch = std::env::var("TA_BATCH_THREADS").ok().and_then(|s| s.parse().ok()).unwrap_or(params.n_threads);''')
        edit('src/local/cleanup.rs', 'params.n_batch = CONTEXT as u32;', '''params.n_batch = std::env::var("TA_BATCH").ok().and_then(|s| s.parse().ok()).unwrap_or(CONTEXT as u32);''')
        edit('src/local/cleanup.rs', 'params.n_ubatch = 512;', '''params.n_ubatch = std::env::var("TA_UBATCH").ok().and_then(|s| s.parse().ok()).unwrap_or(512);
            if std::env::var("TA_FLASH").as_deref() == Ok("1") { params.flash_attn_type = 1; }''')
        edit('src/local/cleanup.rs', 'params.n_gpu_layers = if std::env::var("TYPEASIDE_LOCAL_DEVICE").as_deref() == Ok("cpu")', '''params.n_gpu_layers = if std::env::var("TA_CLEANUP_CPU").as_deref() == Ok("1") || std::env::var("TYPEASIDE_LOCAL_DEVICE").as_deref() == Ok("cpu")''')
        edit('src/local/cleanup.rs', 'let previous = std::mem::take(&mut self.prompt);', '''if std::env::var("TA_CACHE").as_deref() == Ok("0") { self.prompt.clear(); }
            let previous = std::mem::take(&mut self.prompt);''')
        edit('src/local/cleanup.rs', 'let status = llama_decode(ctx, batch);', '''let prefill = self.prompt.is_empty();
                let started = std::time::Instant::now();
                let status = llama_decode(ctx, batch);
                if prefill { eprintln!("PREFILL cached={} tokens={} ms={}", shared, tokens.len(), started.elapsed().as_micros() as f64 / 1000.0); }''')
        edit('src/local/mod.rs', '        Some(_) => None,', '''        Some(_) if std::env::var("TA_PRELOAD_CLEANUP").as_deref() != Ok("1") => None,''')
    (args.scratch / 'instrumentation.patch').write_text(''.join(patches))
    (args.scratch / 'revision.txt').write_bytes(subprocess.check_output(['git', 'rev-parse', args.ref]))
    print(source)


def run(args):
    binary = args.binary.resolve()
    binary_bytes = binary.read_bytes()
    if binary.name != 'typeaside-latency' or b'TYPEASIDE_LATENCY_OUTPUT' not in binary_bytes:
        raise RuntimeError('Use the instrumented release executable named typeaside-latency')
    if args.device == 'gpu' and b'TYPEASIDE_LATENCY_WORKER_ICD' not in binary_bytes:
        raise RuntimeError('Re-export the harness: dGPU timing requires separate UI/worker drivers')
    binary_sha256 = hashlib.sha256(binary_bytes).hexdigest()
    rows = args.scratch / (args.label + '.jsonl')
    if rows.exists():
        raise RuntimeError('This label already has samples; choose a fresh --label')
    home = args.scratch / 'home'
    if not (home / 'data/models').is_dir():
        raise RuntimeError(f'Install models through examples into TYPEASIDE_HOME={home}')
    runtime = args.scratch / 'runtime'
    runtime.mkdir(mode=0o700, exist_ok=True)
    if (runtime / 'typeaside.sock').exists():
        raise RuntimeError('Runtime already has an instance socket; refusing to contact it')
    output = args.scratch / 'delivered.json'
    original_runtime = os.environ['XDG_RUNTIME_DIR']
    wayland = Path(os.environ.get('WAYLAND_DISPLAY', 'wayland-1'))
    if not wayland.is_absolute():
        wayland = Path(original_runtime) / wayland
    env = dict(os.environ, TYPEASIDE_HOME=str(home), XDG_RUNTIME_DIR=str(runtime),
               WAYLAND_DISPLAY=str(wayland), PULSE_SERVER=f'unix:{original_runtime}/pulse/native',
               TYPEASIDE_LATENCY_OUTPUT=str(output), TYPEASIDE_TRACE='1',
               TYPEASIDE_LATENCY_COLD='1' if args.cold else '0',
               TYPEASIDE_LOCAL_DEVICE=args.device)
    if args.icd:
        env['TYPEASIDE_LATENCY_WORKER_ICD'] = str(args.icd.resolve())
        for name in ('VK_DRIVER_FILES', 'VK_ICD_FILENAMES', 'VK_LOADER_DRIVERS_DISABLE', 'VK_LOADER_DRIVERS_SELECT'):
            env.pop(name, None)
    sink = f'typeaside_latency_{os.getpid()}'
    module = subprocess.check_output(['pactl', 'load-module', 'module-null-sink',
                                     f'sink_name={sink}', 'rate=16000', 'channels=1'], text=True).strip()
    env['PULSE_SOURCE'] = sink + '.monitor'
    # ALSA's pulse plugin is scoped to this child environment; no system defaults change.
    alsa = args.scratch / 'alsa.conf'
    alsa.write_text(f'pcm.!default {{ type pulse server "unix:{original_runtime}/pulse/native" device "{sink}.monitor" }}\nctl.!default {{ type pulse }}\n')
    env['ALSA_CONFIG_PATH'] = str(alsa)
    config_dir = home / 'config'
    config_dir.mkdir(exist_ok=True)
    config = dict(hotkey='LEFTCTRL+LEFTSHIFT+RIGHTALT+F24',
                  transcription=SOURCES[args.speech], local_model=args.model, language='en',
                  input_devices=['default'],
                  cleanup=dict(enabled=args.editing, source=SOURCES[args.editing_source]),
                  sounds=False, media_control='off', auto_learn=False, screen_context=False,
                  auto_submit=False, send_command=False, onboarded=True, launch_at_login=False,
                  command_hotkey=None, translate_hotkey=None, note_hotkey=None,
                  pause_hotkey=None, paste_last_hotkey=None)
    (config_dir / 'config.json').write_text(json.dumps(config))
    process = None
    player = None
    bus = None
    log_path = args.scratch / (args.label + '.log')
    log = log_path.open('a')

    def stop():
        nonlocal process
        if process is not None:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            process = None
            (runtime / 'typeaside.sock').unlink(missing_ok=True)

    def start():
        nonlocal process
        process = subprocess.Popen([binary, '--hidden'], env=env, stdout=log, stderr=log)
        deadline = time.monotonic() + 60
        while not (runtime / 'typeaside.sock').exists():
            if process.poll() is not None:
                raise RuntimeError(f'App exited with {process.returncode}; see {log_path}')
            if time.monotonic() > deadline:
                raise TimeoutError('Instance socket not ready')
            time.sleep(.05)
        time.sleep(2)

    try:
        if args.speech == 'key' or (args.editing and args.editing_source == 'key'):
            # API keys live in the user's keyring; the proxy exposes only the Secret Service.
            proxy = runtime / 'bus'
            proxy.unlink(missing_ok=True)
            bus = subprocess.Popen(['xdg-dbus-proxy', os.environ['DBUS_SESSION_BUS_ADDRESS'],
                                    str(proxy), '--filter', '--talk=org.freedesktop.secrets'],
                                   stderr=log)
            deadline = time.monotonic() + 5
            while not proxy.exists():
                if bus.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError('Keyring D-Bus proxy did not start')
                time.sleep(.05)
            address = f'unix:path={proxy}'
        else:
            bus_config = args.scratch / 'bus.conf'
            bus_config.write_text(f'''<busconfig><type>session</type>
<listen>unix:tmpdir={runtime}</listen><auth>EXTERNAL</auth>
<policy context="default"><allow send_destination="*"/>
<allow receive_sender="*"/><allow own="*"/></policy></busconfig>''')
            bus = subprocess.Popen(['dbus-daemon', '--nofork', '--print-address=1',
                                    '--config-file=' + str(bus_config)], stdout=subprocess.PIPE, stderr=log)
            address = bus.stdout.readline().decode().strip()
            if not address:
                raise RuntimeError('Private D-Bus session did not start')
        env['DBUS_SESSION_BUS_ADDRESS'] = address
        for clip in args.clips:
            for iteration in range(args.runs + (not args.cold)):
                if args.cold or process is None:
                    stop()
                    start()
                output.unlink(missing_ok=True)
                log.flush()
                log_offset = log_path.stat().st_size
                command(str(binary), 'dictation', 'start', env=env, stdout=log, stderr=log)
                if args.settle_ms:
                    time.sleep(args.settle_ms / 1000)
                else:
                    deadline = time.monotonic() + 5
                    while b'typeaside overlay frame: listening' not in log_path.read_bytes()[log_offset:]:
                        if process.poll() is not None or time.monotonic() > deadline:
                            raise RuntimeError('Capture did not become ready')
                        time.sleep(.005)
                player = subprocess.Popen(['paplay', '--device=' + sink, str(clip)])
                if player.wait(timeout=90) != 0:
                    raise RuntimeError('Playback into the virtual sink failed')
                player = None
                gpu_power = {}
                if args.device == 'gpu':
                    for pci in Path('/sys/bus/pci/devices').iterdir():
                        if ((pci / 'vendor').read_text().strip() == '0x10de'
                                and (pci / 'class').read_text().strip().startswith('0x03')):
                            gpu_power[pci.name] = (pci / 'power/runtime_status').read_text().strip()
                # paplay drains the sink before release. No extra silent wait is inserted.
                stop_ns = time.time_ns()
                command(str(binary), 'dictation', 'stop', env=env, stdout=log, stderr=log)
                deadline = time.monotonic() + 60
                while not output.exists():
                    if process.poll() is not None or time.monotonic() > deadline:
                        raise RuntimeError(f'No delivery; see {log_path}')
                    time.sleep(.005)
                result = json.loads(output.read_text())
                with log_path.open('rb') as trace_file:
                    trace_file.seek(log_offset)
                    trace = trace_file.read().decode(errors='replace')
                stages = dict((stage, int(ns)) for stage, ns in re.findall(r'LATENCY (release|tail_end) (\d+)', trace))
                timing = re.findall(r'typeaside: stt=(\d+)ms llm=(\d+)ms', trace)
                if not timing or len(stages) != 2:
                    raise RuntimeError('Delivery lacks its pipeline/release trace')
                if timing:
                    result.update(stt_ms=int(timing[-1][0]), cleanup_ms=int(timing[-1][1]))
                if len(stages) == 2:
                    result.update(release_tail_ms=(stages['tail_end'] - stages['release']) / 1e6,
                                  cli_to_release_ms=(stages['release'] - stop_ns) / 1e6,
                                  release_to_delivery_ms=(result['delivered_ns'] - stages['release']) / 1e6)
                result['cleanup_fallbacks'] = re.findall(r'typeaside: cleanup fallback: (.+)', trace)
                result['vad_gate'] = re.findall(r'LATENCY vad_gate quiet=(true|false) elapsed_us=(\d+)', trace)
                result.update(clip=clip.name, iteration=iteration, cold=args.cold,
                              editing=args.editing, device=args.device, model=args.model,
                              speech=args.speech, editing_source=args.editing_source,
                              stop_ns=stop_ns, elapsed_ms=(result['delivered_ns'] - stop_ns) / 1e6,
                              warmup=not args.cold and iteration == 0,
                              settle_ms=args.settle_ms,
                              gpu_power_before_stop=gpu_power,
                              binary_sha256=binary_sha256)
                with rows.open('a') as out:
                    out.write(json.dumps(result) + '\n')
                print(json.dumps(result), flush=True)
                time.sleep(.3)
    finally:
        if player is not None and player.poll() is None:
            player.terminate()
            player.wait()
        stop()
        if bus is not None:
            bus.terminate()
            bus.wait(timeout=5)
        log.close()
        command('pactl', 'unload-module', module)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['prepare', 'run'])
    parser.add_argument('--scratch', type=Path, required=True)
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--label', default='latency')
    parser.add_argument('--ref', default='HEAD', help='Committed source revision to export')
    parser.add_argument('--experiments', action='store_true', help='Add scratch-only inference parameter overrides')
    parser.add_argument('--device', choices=['cpu', 'auto', 'gpu'], default='cpu')
    parser.add_argument('--model', default='parakeet-v3')
    parser.add_argument('--icd', type=Path, help='Scope worker Vulkan enumeration; the UI keeps its default GPU')
    parser.add_argument('--editing', action='store_true')
    parser.add_argument('--speech', choices=SOURCES, default='local')
    parser.add_argument('--editing-source', choices=SOURCES, default='local')
    parser.add_argument('--cold', action='store_true', help='Restart with models unloaded until key press')
    parser.add_argument('--runs', type=int, default=5)
    parser.add_argument('--settle-ms', type=int, default=600, help='Capture settle time; 0 waits for the listening trace')
    parser.add_argument('clips', type=Path, nargs='*')
    args = parser.parse_intermixed_args()
    if args.runs < 1 or args.settle_ms < 0:
        parser.error('runs must be positive and settle-ms must be nonnegative')
    args.scratch = args.scratch.resolve()
    if str(args.scratch).startswith('/tmp') or args.scratch == Path(os.environ['XDG_RUNTIME_DIR']):
        parser.error('Use a separate disk-backed scratch directory')
    args.scratch.mkdir(parents=True, exist_ok=True)
    if args.action == 'prepare':
        prepare(args)
    elif args.binary is None or not args.clips:
        parser.error('run requires --binary and clips')
    else:
        run(args)


def interrupted(*_):
    raise KeyboardInterrupt


if __name__ == '__main__':
    signal.signal(signal.SIGTERM, interrupted)
    main()
