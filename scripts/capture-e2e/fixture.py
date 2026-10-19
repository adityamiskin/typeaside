"""A real GTK text view: native selection, native Ctrl+Insert copy handling."""
import json
import os
import time
from pathlib import Path
import gi

gi.require_version('Gtk', '4.0')
from gi.repository import Gtk, GLib

TEXT = 'Capture exactly this: café, λ, "quotes".'
application = Gtk.Application(application_id='org.typeaside.CaptureFixture')


def activate(app):
    window = Gtk.ApplicationWindow(application=app, title='Quick Capture E2E Fixture')
    window.set_default_size(1000, 600)
    view = Gtk.TextView()
    controller = Gtk.EventControllerKey()
    controller.set_propagation_phase(Gtk.PropagationPhase.CAPTURE)

    def pressed(_controller, keyval, keycode, modifiers):
        print(json.dumps({'time_ns': time.time_ns(), 'keyval': keyval, 'keycode': keycode, 'modifiers': int(modifiers)}), flush=True)
        return False

    controller.connect('key-pressed', pressed)
    view.add_controller(controller)
    view.set_top_margin(80)
    view.set_left_margin(80)
    buffer = view.get_buffer()
    buffer.set_text(TEXT)
    window.set_child(view)
    window.present()
    view.grab_focus()
    last = None

    def commands():
        nonlocal last
        path = Path('/tmp/fixture-command.json')
        if not path.exists() or path.read_text() == last:
            return True
        last = path.read_text()
        command = json.loads(last)
        start, end = buffer.get_bounds()
        if command['selected']:
            buffer.select_range(start, end)
        else:
            buffer.place_cursor(end)
        Path('/tmp/fixture-state.json').write_text(json.dumps({
            'command': command, 'selected': bool(buffer.get_selection_bounds()),
            'text': buffer.get_text(start, end, True), 'pid': os.getpid(),
        }))
        return True

    GLib.timeout_add(20, commands)


application.connect('activate', activate)
application.run([])
