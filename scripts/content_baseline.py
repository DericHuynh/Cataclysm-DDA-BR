#!/usr/bin/env python3
"""Print a deterministic SHA-256 manifest for the supplied source/content roots."""
import hashlib
import json
import pathlib
import sys

for argument in sys.argv[1:]:
    root = pathlib.Path(argument)
    if not root.exists():
        raise SystemExit(f"Missing baseline input: {root}")
    paths = [root] if root.is_file() else sorted(root.rglob('*.json'))
    digest = hashlib.sha256()
    size = 0
    for path in paths:
        content = path.read_bytes()
        name = path.name if root.is_file() else path.relative_to(root).as_posix()
        digest.update(name.encode() + b'\0' + hashlib.sha256(content).digest())
        size += len(content)
    print(json.dumps({'root': str(root), 'files': len(paths), 'bytes': size, 'sha256': digest.hexdigest()}))
