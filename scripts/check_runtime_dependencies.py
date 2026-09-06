#!/usr/bin/env python3
"""Verify actual transitive normal-dependency boundaries (dev fixtures excluded)."""
import json
import pathlib
import subprocess

root = pathlib.Path(__file__).resolve().parents[1]
for crate in ('cdda_sim', 'cdda_catalog', 'cdda_ui'):
    tree = subprocess.check_output([
        'cargo', 'tree', '--offline', '-p', crate, '--edges', 'normal',
        '--prefix', 'none', '--format', '{p}'
    ], cwd=root, text=True)
    dependencies = {line.split()[0] for line in tree.splitlines() if line.strip()}
    forbidden = {'cdda_data', 'cdda_defs_raw', 'cdda_input', 'cdda_context', 'cdda_render', 'leafwing-input-manager'}
    if crate != 'cdda_ui':
        forbidden |= {'bevy_input', 'bevy_asset', 'bevy_render', 'bevy_ui', 'bevy_winit'}
    else:
        forbidden |= {name for name in dependencies if name.startswith('cdda_') and name != crate}
    violations = sorted(dependencies & forbidden)
    print(json.dumps({'crate': crate, 'normal_dependencies': len(dependencies), 'violations': violations}))
    if violations:
        raise SystemExit(1)
