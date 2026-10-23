#!/usr/bin/env python3
"""Exercise the curl installer with real signatures and isolated downloads."""

import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile


def run(*args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, text=True, **kwargs)


with tempfile.TemporaryDirectory(prefix="typeaside-install-test-") as temporary:
    root = Path(temporary)
    archive = Path(sys.argv[1]).resolve()
    with tarfile.open(archive) as bundle:
        bundle.extract("minisign-linux/x86_64/minisign", root, filter="data")
    minisign = root / "minisign-linux/x86_64/minisign"
    secret, public = root / "test.key", root / "test.pub"
    run(str(minisign), "-G", "-W", "-s", str(secret), "-p", str(public))
    artifact = root / "TypeAside.AppImage"
    artifact.write_text("#!/bin/sh\nprintf 'fixture app\\n'\n")
    run(str(minisign), "-S", "-s", str(secret), "-m", str(artifact))
    installer = root / "install.sh"
    source = Path("apps/web/public/install.sh").read_text()
    release_key = Path("packaging/minisign.pub").read_text().splitlines()[1]
    installer.write_text(source.replace(release_key, public.read_text().splitlines()[1]))
    fixtures = root / "downloads"
    fixtures.mkdir()
    shutil.copy(artifact, fixtures)
    shutil.copy(str(artifact) + ".minisig", fixtures)
    shutil.copy(archive, fixtures / "minisign-0.12-linux.tar.gz")
    commands = root / "commands"
    commands.mkdir()
    curl = commands / "curl"
    curl.write_text(
        f"#!{sys.executable}\n"
        "import os, pathlib, shutil, sys\n"
        "args = sys.argv[1:]\n"
        "url = next(arg for arg in args if arg.startswith('https://'))\n"
        "name = url.rsplit('/', 1)[1]\n"
        "if name == os.environ.get('FAIL_DOWNLOAD'): sys.exit(22)\n"
        "shutil.copy(pathlib.Path(os.environ['FIXTURES']) / name, args[args.index('--output') + 1])\n"
    )
    curl.chmod(0o755)
    uname = commands / "uname"
    uname.write_text('#!/bin/sh\ncase "$1" in -s) echo "${TEST_OS:-Linux}";; -m) echo "${TEST_ARCH:-x86_64}";; esac\n')
    uname.chmod(0o755)

    def install(name, **overrides):
        home = root / name / "home with spaces"
        home.mkdir(parents=True, exist_ok=True)
        temporary = root / name / "tmp"
        temporary.mkdir(exist_ok=True)
        environment = {
            **os.environ,
            "HOME": str(home),
            "XDG_DATA_HOME": str(home / "custom data"),
            "TMPDIR": str(temporary),
            "PATH": f"{commands}:/usr/bin:/bin",
            "FIXTURES": str(fixtures),
            **overrides,
        }
        result = subprocess.run(
            ["sh"], input=installer.read_text(), text=True,
            capture_output=True, env=environment,
        )
        assert not list(temporary.iterdir()), result.stderr
        return home, result

    home, result = install("success")
    assert result.returncode == 0, result.stderr
    app = home / "custom data/typeaside/TypeAside.AppImage"
    assert app.read_bytes() == artifact.read_bytes()
    assert os.access(app, os.X_OK)
    launcher = home / ".local/bin/typeaside"
    assert run(str(launcher)).stdout == "fixture app\n"
    desktop = home / "custom data/applications/typeaside.desktop"
    assert f'Exec="{launcher}"' in desktop.read_text()
    assert (home / "custom data/icons/hicolor/scalable/apps/typeaside.svg").is_file()
    _, result = install("success")
    assert result.returncode == 0, result.stderr
    print("PASS: signed install, launcher, desktop entry, spaces, XDG paths and reinstall")

    for overrides in ({"TEST_OS": "Darwin"}, {"TEST_ARCH": "aarch64"}):
        home, result = install("unsupported", **overrides)
        assert result.returncode != 0
        assert not list(home.iterdir())
    print("PASS: unsupported OS and architecture leave the install untouched")

    original = app.read_bytes()
    (fixtures / "TypeAside.AppImage").write_text("tampered\n")
    _, result = install("success")
    assert result.returncode != 0
    assert app.read_bytes() == original
    shutil.copy(artifact, fixtures)
    print("PASS: invalid signature preserves the existing install")

    _, result = install("success", FAIL_DOWNLOAD="TypeAside.AppImage")
    assert result.returncode != 0
    assert app.read_bytes() == original
    print("PASS: failed download preserves the existing install")

    (commands / "minisign").symlink_to(minisign)
    _, result = install("existing-verifier", FAIL_DOWNLOAD="minisign-0.12-linux.tar.gz")
    assert result.returncode == 0, result.stderr
    (commands / "minisign").unlink()
    print("PASS: existing Minisign avoids downloading a verifier")

    (fixtures / "minisign-0.12-linux.tar.gz").write_text("tampered verifier\n")
    home, result = install("bad-verifier")
    assert result.returncode != 0
    assert not list(home.iterdir())
    print("PASS: verifier checksum mismatch prevents installation")
