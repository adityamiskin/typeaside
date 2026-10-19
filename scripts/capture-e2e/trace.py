"""Instrument only the exported build tree, never the checked-out product files."""
from pathlib import Path
import difflib


def instrument(source, artifacts):
    patches = []

    def edit(name, replacements):
        path = source / name
        before = path.read_text()
        after = before
        for old, new in replacements:
            if after.count(old) != 1:
                raise RuntimeError(f"Trace anchor must occur exactly once: {name}: {old}")
            after = after.replace(old, new)
        path.write_text(after)
        patches.extend(difflib.unified_diff(before.splitlines(True), after.splitlines(True), f'a/{name}', f'b/{name}'))

    def trace(stage):
        return f'typeaside::capture_e2e_trace("{stage}");'

    edit('src/lib.rs', [('pub mod engine;', '''pub mod engine;
pub fn capture_e2e_trace(stage: &str) {
    eprintln!("E2E {} {}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(), stage);
}
''')])
    modern = 'Event::ShowCapture' in (source / 'src/engine.rs').read_text()
    if modern:
        edit('src/engine.rs', [
            ('capture_open = true;\n                        #[cfg(target_os = "linux")]', trace('hotkey').replace('typeaside::', 'crate::') + '\ncapture_open = true;\n                        #[cfg(target_os = "linux")]'),
            ('let focused = focused_app();', 'let focused = focused_app();\n' + trace('focused_app').replace('typeaside::', 'crate::')),
            ('let app = focused.and_then(|app| app.name);', trace('selected_text').replace('typeaside::', 'crate::') + '\nlet app = focused.and_then(|app| app.name);'),
        ])
    else:
        edit('src/engine.rs', [
            ('let app = focused_app().and_then(|app| app.name);', trace('hotkey').replace('typeaside::', 'crate::') + '\nlet app = focused_app().and_then(|app| app.name);\n' + trace('focused_app').replace('typeaside::', 'crate::')),
            ('let selection = crate::text_injector::selected_text();', 'let selection = crate::text_injector::selected_text();\n' + trace('selected_text').replace('typeaside::', 'crate::')),
        ])
    # Library modules use crate::, the binary UI uses typeaside::.
    def local(stage):
        return trace(stage).replace('typeaside::', 'crate::')

    edit('src/hotkeys/linux.rs', [('let (wake, reader) = UnixStream::pair()?;', local('keyboards_ready') + '\nlet (wake, reader) = UnixStream::pair()?;')])
    edit('src/text_injector/linux.rs', [
        (('fn copy_selection(app_id: Option<&str>) -> Result<String> {' if modern else 'fn copy_selection() -> Result<String> {'), ('fn copy_selection(app_id: Option<&str>) -> Result<String> {' if modern else 'fn copy_selection() -> Result<String> {') + '\n' + local('copy_begin')),
        (('let terminal = app_id.is_some_and(is_terminal);' if modern else 'let terminal = focused_window()'), local('injector_ready') + '\n' + ('let terminal = app_id.is_some_and(is_terminal);' if modern else 'let terminal = focused_window()')),
        ('let previous = read_clipboard()?;\n        // Clearing', local('copy_focus') + '\nlet previous = read_clipboard()?;\n' + local('clipboard_read') + '\n        // Clearing'),
        ('if terminal {\n                injector.combo(', local('watch_ready') + '\nif terminal {\n                injector.combo('),
        ('if changed.recv_timeout(Duration::from_millis(120)).is_err() {', local('combo_done') + '\nif changed.recv_timeout(Duration::from_millis(120)).is_err() {\n' + local('copy_timeout')),
        ('let text = read_clipboard()?.unwrap_or_default();', local('copy_changed') + '\nlet text = read_clipboard()?.unwrap_or_default();\n' + local('selection_read')),
        ('Ok(text)\n        })();', local('clipboard_restored') + '\nOk(text)\n        })();'),
    ])
    edit('src/ui/mod.rs', [('Event::Capture { selection, app } => capture::toggle(&state, selection, app, cx),', 'Event::Capture { selection, app } => { ' + trace('ui_event') + ' capture::toggle(&state, selection, app, cx); ' + trace('window_opened') + ' },')])
    if modern:
        edit('src/ui/mod.rs', [('Event::ShowCapture => capture::show(&state, cx),', 'Event::ShowCapture => { ' + trace('show_event') + ' capture::show(&state, cx); },')])
    edit('src/ui/capture.rs', [
        ('cx.observe_window_appearance(window,', '''cx.observe_window_activation(window, |_: &mut Capture, window, _| {
            if window.is_window_active() { typeaside::capture_e2e_trace("keyboard_focus"); }
        }).detach();
        cx.observe_window_appearance(window,'''),
        ('.capture_action(cx.listener(|this, action: &Enter, window, cx| {', '''.capture_key_down(|event, _, _| {
            eprintln!("E2E_KEY {}", serde_json::json!({"key": event.keystroke.key, "control": event.keystroke.modifiers.control}));
        })
        .capture_action(cx.listener(|this, action: &Enter, window, cx| {'''),
        ('let p = palette(cx);', ('if self.visible { ' + trace('render') + ' }' if modern else trace('render')) + '\nlet p = palette(cx);'),
        ('let height = 80. + lines as f32 * 22.;', '''let height = 80. + lines as f32 * 22.;
        eprintln!("E2E_STATE {}", serde_json::json!({"value": value.to_string(), "source": self.state.read(cx).note_source, "rows": lines, "input_height": lines as f32 * 22., "panel_height": height}));'''),
        ('Err(error) => state.update(cx, |s, cx| {', 'Err(error) => state.update(cx, |s, cx| { eprintln!("E2E_CAPTURE_ERROR {error:#}");'),
    ])
    edit('vendor/gpui-pre-wgpu/src/wgpu_renderer.rs', [
        ('pub fn draw(&mut self, scene: &Scene) -> bool {', '''pub fn draw(&mut self, scene: &Scene) -> bool {
        eprintln!("E2E_SCENE quads={} glyphs={} shadows={}", scene.quads.len(), scene.monochrome_sprites.len(), scene.shadows.len());
        eprintln!("E2E {} draw", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());'''),
        ('frame.present();', '''frame.present();
        eprintln!("E2E {} present", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());''')])
    (artifacts / 'trace.patch').write_text(''.join(patches))
