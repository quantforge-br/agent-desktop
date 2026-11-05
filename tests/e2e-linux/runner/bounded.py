from __future__ import annotations

import importlib.util
from pathlib import Path

_MODULE_PATH = Path(__file__).resolve().parent.parent.parent / "e2e" / "bounded_process.py"


def load_bounded_process():
    spec = importlib.util.spec_from_file_location("bounded_process", _MODULE_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module
