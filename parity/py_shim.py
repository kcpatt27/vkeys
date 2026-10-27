"""Run the Python vkeys CLI with a stdin that getpass can read from a pipe.

CPython's Windows getpass() reads the console directly (msvcrt.getwch) and hangs
when stdin is a pipe, unless sys.stdin is not sys.__stdin__. This shim replaces
sys.stdin so getpass takes its documented fallback path. Harness-only: the
Python implementation is untouched.
"""

import io
import sys
import warnings

sys.stdin = io.TextIOWrapper(sys.stdin.buffer, encoding="utf-8", errors="replace")
warnings.simplefilter("ignore")

from vkeys.cli import main  # noqa: E402

sys.exit(main())
