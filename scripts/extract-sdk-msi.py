"""Read MSI tables and unpack CAB payloads without invoking Windows Installer."""
import ctypes
import os
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MSI = ctypes.WinDLL('msi')
HANDLE = ctypes.c_uint
MSI.MsiOpenDatabaseW.argtypes = [ctypes.c_wchar_p, ctypes.c_wchar_p, ctypes.POINTER(HANDLE)]
MSI.MsiDatabaseOpenViewW.argtypes = [HANDLE, ctypes.c_wchar_p, ctypes.POINTER(HANDLE)]
MSI.MsiViewExecute.argtypes = [HANDLE, HANDLE]
MSI.MsiViewFetch.argtypes = [HANDLE, ctypes.POINTER(HANDLE)]
MSI.MsiRecordGetStringW.argtypes = [HANDLE, ctypes.c_uint, ctypes.c_wchar_p, ctypes.POINTER(ctypes.c_uint)]
MSI.MsiCloseHandle.argtypes = [HANDLE]

def safe_child(root, relative):
    root = root.resolve()
    target = (root / relative).resolve()
    if target != root and root not in target.parents:
        raise ValueError(f'Path escapes output: {relative}')
    return target

def query(db, sql, count):
    view = HANDLE()
    rc = MSI.MsiDatabaseOpenViewW(db, sql, ctypes.byref(view))
    if rc:
        raise OSError(rc, sql)
    rows = []
    try:
        rc = MSI.MsiViewExecute(view, 0)
        if rc:
            raise OSError(rc, sql)
        while True:
            record = HANDLE()
            rc = MSI.MsiViewFetch(view, ctypes.byref(record))
            if rc == 259:
                break
            if rc:
                raise OSError(rc, sql)
            try:
                row = []
                for index in range(1, count + 1):
                    value = ctypes.create_unicode_buffer(32768)
                    size = ctypes.c_uint(32767)
                    rc = MSI.MsiRecordGetStringW(record, index, value, ctypes.byref(size))
                    if rc:
                        raise OSError(rc, sql)
                    row.append(value.value)
                rows.append(row)
            finally:
                MSI.MsiCloseHandle(record)
    finally:
        MSI.MsiCloseHandle(view)
    return rows

def extract_msi(msi_path, destination):
    msi_path, destination = Path(msi_path).resolve(), Path(destination).resolve()
    safe_child(ROOT, destination.relative_to(ROOT))
    db = HANDLE()
    rc = MSI.MsiOpenDatabaseW(str(msi_path), None, ctypes.byref(db))
    if rc:
        raise OSError(rc, f'Opening {msi_path}')
    try:
        directories = {row[0]: row[1:] for row in query(db, 'SELECT `Directory`,`Directory_Parent`,`DefaultDir` FROM `Directory`', 3)}
        components = dict(query(db, 'SELECT `Component`,`Directory_` FROM `Component`', 2))
        files = query(db, 'SELECT `File`,`Component_`,`FileName` FROM `File`', 3)
        cabinets = [row[0] for row in query(db, 'SELECT `Cabinet` FROM `Media`', 1) if row[0]]
    finally:
        MSI.MsiCloseHandle(db)
    def folder(key):
        if key in ('TARGETDIR', 'ProgramFilesFolder', 'ProgramFiles64Folder', 'ProgramFilesFolder.643890BFEF154C1E93DEB8B8F52879D8'):
            return Path()
        parent, name = directories[key]
        name = name.split(':')[0].split('|')[-1]
        if name in ('.', 'SourceDir'):
            name = ''
        return (folder(parent) if parent else Path()) / name
    staging = safe_child(ROOT, '.tmp/sdk-cabs')
    staging.mkdir(parents=True, exist_ok=True)
    expand = Path(os.environ.get('SystemRoot', r'C:\Windows')) / 'System32/expand.exe'
    for cabinet in cabinets:
        if cabinet.startswith('#'):
            raise ValueError('Embedded cabinet is not supported')
        print(f'Extracting {cabinet}', flush=True)
        subprocess.run([str(expand), '-F:*', str(msi_path.parent / cabinet), str(staging)], check=True, stdout=subprocess.DEVNULL)
    copied = 0
    for file_id, component, filename in files:
        source = staging / file_id
        if not source.is_file():
            matches = list(staging.rglob(file_id))
            if not matches:
                raise FileNotFoundError(f'Missing CAB payload {file_id}')
            source = matches[0]
        target = safe_child(destination, folder(components[component]) / filename.split('|')[-1])
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        copied += 1
    print(f'{msi_path.name}: {copied} files extracted', flush=True)
