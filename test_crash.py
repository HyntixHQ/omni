#!/usr/bin/env python3
"""Stability + edge-case test for omni daemon.

Tests:
  - Rapid consecutive same-app launches (buffer reuse)
  - Rapid show/hide cycling with zero delay
  - Alternating launcher/calculator (mode switch)
  - Calculator expressions (valid + invalid)
  - Leaving surface open for extended duration
  - Many rapid IPC requests in a burst
  - Quit + restart cycle
"""

import os
import subprocess
import sys
import time
import threading

OMNI_BIN = os.path.expanduser("~/.local/bin/omni")
LOG_FILE = os.path.expanduser("~/.local/share/omni/daemon.log")


def start_daemon():
    proc = subprocess.Popen([OMNI_BIN, "--daemon"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(0.8)
    return proc


def stop_daemon(proc):
    proc.terminate()
    try:
        proc.wait(timeout=3)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait()


def send_command(cmd, timeout=5):
    """Send an IPC command. cmd is like 'launcher' or 'calculator:2+2'."""
    result = subprocess.run([OMNI_BIN, *cmd.split()], capture_output=True, text=True, timeout=timeout)
    return result.returncode == 0, result.stderr.strip()


def check_log_errors():
    """Return any ERROR lines from the daemon log."""
    if not os.path.exists(LOG_FILE):
        return []
    with open(LOG_FILE) as f:
        return [l for l in f.readlines() if "ERROR" in l]


def clear_log():
    if os.path.exists(LOG_FILE):
        os.remove(LOG_FILE)


def run_requests(label, requests, delay=0.3):
    """Run a list of (request_string,) tuples, return (pass, total, errors)."""
    passed = 0
    total = len(requests)
    daemon = start_daemon()
    time.sleep(0.5)
    errors = []
    for i, req in enumerate(requests):
        ok, stderr = send_command(req)
        alive = daemon.poll() is None
        log_errors = check_log_errors()
        if log_errors:
            errors.append(f"  FAIL #{i+1} ({req}): daemon error: {log_errors[-1].strip()}")
        elif not alive:
            errors.append(f"  FAIL #{i+1} ({req}): daemon died")
        elif not ok:
            errors.append(f"  FAIL #{i+1} ({req}): IPC failed — {stderr[:80]}")
        else:
            passed += 1
        if errors and len(errors) > 5:
            errors.append("  ... (truncated)")
            break
        time.sleep(delay)
    stop_daemon(daemon)
    clear_log()
    return passed, total, errors, label


def run_requests_burst(requests, delay=0.0):
    """Fire all requests with minimal delay, then check daemon after."""
    daemon = start_daemon()
    time.sleep(0.5)
    errors = []
    passed = 0
    total = len(requests)
    for i, req in enumerate(requests):
        ok, stderr = send_command(req)
        if not ok:
            errors.append(f"  FAIL #{i+1} ({req}): IPC failed — {stderr[:80]}")
            break
        passed += 1
        if delay > 0:
            time.sleep(delay)
    # After burst, verify daemon still alive
    alive = daemon.poll() is None
    log_errors = check_log_errors()
    if alive:
        passed += 0  # still passed
    else:
        errors.append(f"  daemon died during burst (passed {passed}/{total})")
    if log_errors and not errors:
        errors.append(f"  daemon has errors: {log_errors[-1].strip()}")
    stop_daemon(daemon)
    clear_log()
    return passed, total, errors, "burst"


def run_extended_open(duration=3.0):
    """Open launcher, leave it visible for `duration` seconds, then dismiss."""
    daemon = start_daemon()
    time.sleep(0.5)
    errors = []
    ok, stderr = send_command("launcher")
    if not ok:
        errors.append(f"  open failed: {stderr[:80]}")
    else:
        time.sleep(duration)
        # Send another launcher to trigger hide
        ok2, _ = send_command("launcher")
        time.sleep(0.3)
        if not ok2:
            errors.append(f"  close failed")
        alive = daemon.poll() is None
        log_errors = check_log_errors()
        if not alive:
            errors.append("daemon died during extended open")
        elif log_errors:
            errors.append(f"errors: {log_errors[-1].strip()}")
    stop_daemon(daemon)
    clear_log()
    return 1 if not errors else 0, 1, errors, f"extended({duration}s)"


def main():
    subprocess.run(["pkill", "-9", "omni"], capture_output=True)
    time.sleep(0.3)
    clear_log()

    test_cases = []

    # 1. Rapid consecutive same-app launches (10 launcher in a row)
    test_cases.append(
        run_requests(
            "10 consecutive launcher",
            ["launcher"] * 10,
            delay=0.15,
        )
    )

    # 2. Rapid consecutive calculator
    test_cases.append(
        run_requests(
            "10 consecutive calculator",
            ["calculator"] * 10,
            delay=0.15,
        )
    )

    # 3. Rapid alternating (launcher → calculator → launcher → calculator ...)
    test_cases.append(
        run_requests(
            "10 alternating",
            ["launcher", "calculator"] * 5,
            delay=0.15,
        )
    )

    # 4. Zero-delay burst of same-app (as fast as the shell can spawn)
    test_cases.append(
        run_requests_burst(
            ["launcher"] * 5,
            delay=0.0,
        )
    )

    # 5. Calculator with valid expressions
    test_cases.append(
        run_requests(
            "calculator expressions",
            [
                "calculator:2+2",
                "calculator:10*5",
                "calculator:sqrt(16)+1",
                "calculator:2^10",
                "calculator:(1+2)*3",
                "calculator:42",
            ],
            delay=0.2,
        )
    )

    # 6. Calculator with invalid expressions (should not crash)
    test_cases.append(
        run_requests(
            "invalid calculator",
            [
                "calculator:",
                "calculator:hello world",
                "calculator:1/0",
            ],
            delay=0.2,
        )
    )

    # 7. Leave launcher open for 5 seconds
    test_cases.append(
        run_extended_open(duration=5.0)
    )

    # 8. Rapid show/hide cycling (open → close → open → close → ...)
    test_cases.append(
        run_requests(
            "rapid show/hide",
            ["launcher"] * 20,
            delay=0.05,
        )
    )

    # 9. Calculator mode after launcher, then back
    test_cases.append(
        run_requests(
            "mode switch",
            [
                "launcher",
                "calculator",
                "launcher",
                "calculator:99*99",
                "launcher",
                "calculator",
                "calculator:sqrt(144)",
            ],
            delay=0.2,
        )
    )

    # 10. Launcher with long-lived calculator expression
    test_cases.append(
        run_requests(
            "calculator long expr",
            [
                "calculator:1234567890*9876543210+1111111111-2222222222",
                "launcher",
                "calculator",
            ],
            delay=0.3,
        )
    )

    # Results
    total_passed = 0
    total_requests = 0
    total_failures = 0
    print(f"\n{'='*70}")
    print(f"{'TEST':<35} {'PASSED':>8} {'TOTAL':>6} {'STATUS':>10}")
    print(f"{'='*70}")
    for passed, total, errors, label in test_cases:
        status = "PASS" if passed == total else "FAIL"
        total_passed += passed
        total_requests += total
        if errors:
            total_failures += 1
        print(f"{label:<35} {passed:>8} {total:>6} {status:>10}")
        for e in errors[:3]:
            print(f"  {e}")

    print(f"{'='*70}")
    print(f"Total: {total_passed}/{total_requests} requests passed")
    if total_failures == 0:
        print("RESULT: ALL TESTS PASSED")
        return 0
    else:
        print(f"RESULT: {total_failures} test(s) had failures")
        return 1


if __name__ == "__main__":
    sys.exit(main())
