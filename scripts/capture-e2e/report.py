"""Summarize the retained observations without hiding failed runs."""
import statistics
from PIL import Image


def verify_screenshots(result, artifacts):
    for run in result['runs']:
        if run['selected']:
            continue
        with Image.open(artifacts / f'capture-{run["index"]}.png') as image:
            rgb = image.convert('RGB')
            column = [rgb.getpixel((rgb.width // 2, y)) for y in range(rgb.height)]
        spans = []
        start = None
        for y, color in enumerate(column + [None]):
            if color == (24, 25, 27) and start is None:
                start = y
            elif color != (24, 25, 27) and start is not None:
                spans.append(y - start)
                start = None
        run['screenshot_panel_height_px'] = max(spans, default=0)
        run['screenshot_geometry_pass'] = run['screenshot_panel_height_px'] == 168
    result['assertions']['four_empty_rows'] = result['assertions'].get('four_empty_rows', False) and all(
        run.get('screenshot_geometry_pass', False) for run in result['runs'] if not run['selected']
    )


def breakdown(run):
    phase = run['phases_ms']
    return {
        'release to engine hotkey': phase['hotkey'],
        'first focused_app lookup': phase['focused_app'] - phase['hotkey'],
        'injector creation/reuse': phase['injector_ready'] - phase['copy_begin'],
        'additional focus lookup': phase['copy_focus'] - phase['injector_ready'],
        'clipboard snapshot + watcher startup': phase['watch_ready'] - phase['copy_focus'],
        'copy key injection': phase['combo_done'] - phase['watch_ready'],
        'copy wait': phase.get('copy_changed', phase.get('copy_timeout')) - phase['combo_done'],
        'selection read + restore + watcher cleanup': phase['selected_text'] - phase.get('copy_changed', phase.get('copy_timeout')),
        'engine to UI delivery': phase['ui_event'] - phase['selected_text'],
        'release to keyboard focus': phase.get('keyboard_focus', float('nan')),
        'UI show to first present': phase['present'] - phase.get('show_event', phase['ui_event']),
    }


def write(result, artifacts, revision):
    lines = [f'Revision: {revision}', f'Artifacts: {artifacts}', f'Overall: {"PASS" if result["passed"] else "FAIL"}', '']
    for name, passed in result['assertions'].items():
        lines.append(f'{"PASS" if passed else "FAIL"}: {name}')
    for name, passed in result['isolation'].items():
        lines.append(f'{"PASS" if passed else "FAIL"}: isolation {name}')
    if result.get('error'):
        lines.append(result['error'])
    runs = result['runs']
    if result.get('idle_rss_kib'):
        lines.append(f"Idle RSS KiB: {result['idle_rss_kib']}")
    if runs:
        lines.extend(['', f'Cold first frame: {runs[0]["phases_ms"]["present"]:.2f} ms'])
        for name, value in breakdown(runs[0]).items():
            lines.append(f'  {name}: {value:.2f} ms')
        for selected in (True, False):
            group = [r for r in runs[1:] if r['selected'] == selected]
            if group:
                lines.extend(['', f'Warm {"selected" if selected else "empty"} (n={len(group)}) median: {statistics.median(r["phases_ms"]["present"] for r in group):.2f} ms'])
                parts = [breakdown(r) for r in group]
                for name in parts[0]:
                    lines.append(f'  {name} median: {statistics.median(p[name] for p in parts):.2f} ms')
        lines.extend(['', 'Per-run latency and prefill/header/clipboard results:'])
        for run in runs:
            lines.append(f'  {run["index"]}: {run["phases_ms"]["present"]:.2f} ms; selected={run["selected"]}; prefill={run["prefill_pass"]}; header={run["header_pass"]}; clipboard={run["clipboard_pass"]}; focus={run['phases_ms'].get('keyboard_focus', float('nan')):.2f} ms; typing={run.get('typing_pass')}; copy_target={run.get('copy_target_pass')}')
    lines.extend(['', 'Latency ends at the first successful wgpu frame.present() after Capture::render.',
                  'It measures a submitted frame, not physical monitor scanout. This VM uses lavapipe and a debug build.',
                  'Screenshots and run-N.json retain the rendered result and exact text/geometry observations.'])
    (artifacts / 'report.txt').write_text('\n'.join(lines) + '\n')
