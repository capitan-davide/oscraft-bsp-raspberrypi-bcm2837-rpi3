#!/usr/bin/env python3
"""Boot an image in QEMU and check that QNX comes up on all CPUs.

Usage: test.py <image> [<cpus>]

Starts tools/qemu/run.sh with the mini-UART on stdio, waits for the boot
script's banner and the shell prompt, runs a few commands and checks
their output: the number of CPUs, a thread running on each of them,
uname. Exits 0 if all checks pass. The console output goes to stdout.
"""

import os
import selectors
import subprocess
import sys
import time

TIMEOUT = 60  # seconds, for each step

image = sys.argv[1]
cpus = int(sys.argv[2]) if len(sys.argv) > 2 else 4
run = os.path.join(os.path.dirname(os.path.abspath(__file__)), "run.sh")
qemu = subprocess.Popen([run, image, "-no-reboot"], stdin=subprocess.PIPE,
                        stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                        env=dict(os.environ, SERIAL1="stdio"))
sel = selectors.DefaultSelector()
sel.register(qemu.stdout, selectors.EVENT_READ)
output = b""


def expect(text):
    """Read the console until `text` appears; return what came before it."""
    global output
    start = len(output)
    deadline = time.monotonic() + TIMEOUT
    while text.encode() not in output[start:]:
        left = deadline - time.monotonic()
        if left <= 0 or not sel.select(left):
            fail(f"timed out waiting for {text!r}")
        chunk = os.read(qemu.stdout.fileno(), 4096)
        if not chunk:
            fail(f"QEMU exited while waiting for {text!r}")
        sys.stdout.write(chunk.decode(errors="replace").replace("\r", ""))
        sys.stdout.flush()
        output += chunk
    at = output.index(text.encode(), start)
    return output[start:at].decode(errors="replace").replace("\r", "")


def send(line):
    qemu.stdin.write(line.encode() + b"\r")
    qemu.stdin.flush()


def fail(why):
    print(f"\nFAIL: {why}")
    qemu.kill()
    sys.exit(1)


expect("Welcome to QNX")
expect(f"{cpus} CPUs")
expect("# ")
send("pidin info")
info = expect("# ")
found = info.count("Processor")
if found != cpus:
    fail(f"pidin info lists {found} processors, not {cpus}")
# The kernel's idle threads, one per CPU, are procnto's first threads.
send("pidin -p 1 -f bl")
threads = expect("# ")
idle = {line.split()[1] for line in threads.splitlines()[2:2 + cpus] if len(line.split()) == 2}
if idle != {str(c) for c in range(cpus)}:
    fail(f"idle threads on CPUs {sorted(idle)}, not on all {cpus}")
send("uname -m")
if "RaspberryPi3B" not in expect("# "):
    fail("uname -m is not RaspberryPi3B")
qemu.kill()
print(f"\nPASS: QNX on {cpus} CPUs")
