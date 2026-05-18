import sys
import os
import shutil


def _native_library_filename(base_name: str) -> str:
    if sys.platform.startswith("win"):
        return f"{base_name}.dll"
    if sys.platform == "darwin":
        return f"lib{base_name}.dylib"
    return f"lib{base_name}.so"


# Resolve paths relative to this file
_HERE = os.path.dirname(os.path.abspath(__file__))
_REPO_ROOT = os.path.normpath(os.path.join(_HERE, "..", "..", ".."))
_BINDINGS_DIR = os.path.join(_REPO_ROOT, "tests", "e2e", "bindings-generator", "src")
_LIBRARY_FILENAME = _native_library_filename("rust_lib")

# Copy the platform native library next to Generated.py so ctypes.CDLL can find it
_DLL_CANDIDATES = [
    os.path.join(_REPO_ROOT, "target", "debug", _LIBRARY_FILENAME),
    os.path.join(_REPO_ROOT, "target", "release", _LIBRARY_FILENAME),
]
_DLL_DEST = os.path.join(_BINDINGS_DIR, _LIBRARY_FILENAME)

for _candidate in _DLL_CANDIDATES:
    if os.path.exists(_candidate):
        shutil.copy2(_candidate, _DLL_DEST)
        break
else:
    raise RuntimeError(
        f"{_LIBRARY_FILENAME} not found. Run 'cargo build -p rust_lib' first."
    )

# Add bindings directory to sys.path so `import Generated` works
if _BINDINGS_DIR not in sys.path:
    sys.path.insert(0, _BINDINGS_DIR)
