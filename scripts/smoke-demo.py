#!/usr/bin/env python3
"""Launch only the demo, wait for a painted main frame, retain log and screenshot."""
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

artifacts = Path("smoke-artifacts")
artifacts.mkdir(exist_ok=True)
log = artifacts / "demo.log"
env = dict(os.environ, TYPEASIDE_TRACE="1")
with log.open("w") as output:
    process = subprocess.Popen(
        [sys.argv[1], "--demo"], stdout=output, stderr=subprocess.STDOUT,
        env=env, start_new_session=sys.platform != "win32",
    )
    try:
        deadline = time.monotonic() + 30
        while "typeaside first main frame:" not in log.read_text(errors="replace"):
            if process.poll() is not None:
                raise RuntimeError(f"demo exited with {process.returncode}")
            if time.monotonic() >= deadline:
                raise TimeoutError("no first main frame within 30 seconds")
            time.sleep(0.1)
        print(log.read_text(errors="replace"))
        screenshot = str((artifacts / "demo.png").resolve())
        if sys.platform == "darwin":
            command = ["screencapture", "-x", screenshot]
        elif sys.platform == "win32":
            command = ["powershell", "-NoProfile", "-Command", r"""
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$image = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$graphics = [System.Drawing.Graphics]::FromImage($image)
$graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
$image.Save($args[0], [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$image.Dispose()
""".replace("$args[0]", "'" + screenshot.replace("'", "''") + "'")]
        else:
            command = ["import", "-window", "root", screenshot]
        try:
            subprocess.run(command, timeout=10, check=True)
        except (OSError, subprocess.SubprocessError) as error:
            print(f"Screenshot unavailable on this runner: {error}")
    finally:
        if process.poll() is None:
            if sys.platform == "win32":
                process.terminate()
            else:
                os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
