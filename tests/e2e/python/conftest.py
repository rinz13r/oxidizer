import sys
import os
import shutil

# Resolve paths relative to this file
_HERE = os.path.dirname(os.path.abspath(__file__))
_REPO_ROOT = os.path.normpath(os.path.join(_HERE, "..", "..", ".."))
_BINDINGS_DIR = os.path.join(_REPO_ROOT, "tests", "e2e", "bindings-generator", "src")

# Copy rust_lib.dll next to Generated.py so ctypes.CDLL can find it
_DLL_CANDIDATES = [
    os.path.join(_REPO_ROOT, "target", "debug", "rust_lib.dll"),
    os.path.join(_REPO_ROOT, "target", "release", "rust_lib.dll"),
]
_DLL_DEST = os.path.join(_BINDINGS_DIR, "rust_lib.dll")

for _candidate in _DLL_CANDIDATES:
    if os.path.exists(_candidate):
        shutil.copy2(_candidate, _DLL_DEST)
        break
else:
    raise RuntimeError(
        "rust_lib.dll not found. Run 'cargo build -p rust_lib' first."
    )

# Add bindings directory to sys.path so `import Generated` works
if _BINDINGS_DIR not in sys.path:
    sys.path.insert(0, _BINDINGS_DIR)
