#!/usr/bin/env python3
"""Boot a disposable KVM guest with a read-only host root and one writable artifact share."""
import os
from pathlib import Path
import shutil
import subprocess
import re


def prepare(artifacts):
    kernel = os.uname().release
    tree = artifacts / "initramfs"
    if tree.exists():
        shutil.rmtree(tree)
    for directory in ("bin", "lib64", "usr/lib", "dev", "proc", "sys", "newroot"):
        (tree / directory).mkdir(parents=True, exist_ok=True)
    shutil.copy2("/usr/lib/initcpio/busybox", tree / "bin/busybox")
    for library in ("/usr/lib/libcrypt.so.2", "/usr/lib/libc.so.6", "/lib64/ld-linux-x86-64.so.2"):
        shutil.copy2(library, tree / library.lstrip("/"))
    for program in ("mount", "kmod", "switch_root"):
        source = Path('/usr/bin') / program
        shutil.copy2(source, tree / 'bin' / program)
        for library in re.findall(r'(?:=> )?(/\S+) \(', subprocess.check_output(['ldd', source], text=True)):
            dest = tree / library.lstrip('/')
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(library, dest)
    (tree / 'bin/insmod').symlink_to('kmod') if not (tree / 'bin/insmod').exists() else None
    modules = []
    for name in ("9p", "9pnet_virtio", "virtio_gpu", "uinput", "virtio_input"):
        deps = subprocess.check_output(["modprobe", "--show-depends", name], text=True)
        for line in deps.splitlines():
            if line.startswith("insmod "):
                source = Path(line.split()[1])
                dest = tree / source.name.removesuffix(".zst")
                if dest.name not in modules:
                    dest.write_bytes(subprocess.check_output(["zstd", "-dq", "-c", source]))
                    modules.append(dest.name)
    (tree / "init").write_text("""#!/bin/busybox sh
export PATH=/bin
/bin/busybox --install -s /bin
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev
""" + "".join(f"insmod /{module}\n" for module in modules) + """
mount -t 9p -o trans=virtio,version=9p2000.L,ro,cache=loose host /newroot || exec sh
mount -t tmpfs tmpfs /newroot/run
mount -t tmpfs tmpfs /newroot/tmp
mount -t 9p -o trans=virtio,version=9p2000.L artifacts /newroot/mnt
mount --move /dev /newroot/dev
mount --move /proc /newroot/proc
mount --move /sys /newroot/sys
exec switch_root /newroot /bin/bash /mnt/guest-init.sh
""")
    (tree / "init").chmod(0o755)
    with (artifacts / "initramfs.cpio").open("wb") as output:
        subprocess.run(["bash", "-c", "find . -print0 | cpio --null -o -H newc --owner=0:0"], cwd=tree, stdout=output, check=True)
    return [
        "qemu-system-x86_64", "-enable-kvm", "-cpu", "host", "-m", "3072", "-smp", "4",
        "-kernel", f"/usr/lib/modules/{kernel}/vmlinuz", "-initrd", str(artifacts / "initramfs.cpio"),
        "-append", "console=ttyS0 rdinit=/init panic=-1", "-display", "egl-headless,rendernode=/dev/dri/renderD129", "-serial", "stdio",
        "-monitor", "none", "-no-reboot", "-no-shutdown", "-nic", "none", "-device", "virtio-vga-gl",
        "-virtfs", "local,path=/,mount_tag=host,security_model=none,readonly=on",
        "-virtfs", f"local,path={artifacts},mount_tag=artifacts,security_model=none",
    ]
