import os
import shutil
import sys
from pathlib import Path


def _native_library_filename(base_name: str) -> str:
    if sys.platform.startswith("win"):
        return f"{base_name}.dll"
    if sys.platform == "darwin":
        return f"lib{base_name}.dylib"
    return f"lib{base_name}.so"


# Resolve paths relative to this file
_HERE = Path(__file__).resolve().parent
_REPO_ROOT = _HERE.parents[2]
_BINDINGS_DIR = _REPO_ROOT / "target" / "generated" / "e2e"
_LIBRARY_FILENAME = _native_library_filename("rust_lib")
_GENERATED_PY = _BINDINGS_DIR / "Generated.py"

_BINDINGS_DIR.mkdir(parents=True, exist_ok=True)

if not _GENERATED_PY.exists():
    raise RuntimeError(
        f"{_GENERATED_PY} not found. Run 'cargo xtask generate-bindings' first."
    )

# Copy the platform native library next to Generated.py so ctypes.CDLL can find it
_DLL_CANDIDATES = [
    _REPO_ROOT / "target" / "debug" / _LIBRARY_FILENAME,
    _REPO_ROOT / "target" / "release" / _LIBRARY_FILENAME,
]
_DLL_DEST = _BINDINGS_DIR / _LIBRARY_FILENAME

for _candidate in _DLL_CANDIDATES:
    if _candidate.exists():
        shutil.copy2(_candidate, _DLL_DEST)
        break
else:
    raise RuntimeError(
        f"{_LIBRARY_FILENAME} not found. Run 'cargo build -p rust_lib' first."
    )

# Add bindings directory to sys.path so `import Generated` works
if str(_BINDINGS_DIR) not in sys.path:
    sys.path.insert(0, str(_BINDINGS_DIR))
