#!/usr/bin/python3

import sys, shutil, io, tarfile, json, os, subprocess, urllib.request as request
from os.path import expanduser, join, basename

# Setup stdout in case the script was piped in on *nix
if os.path.exists('/dev/tty'):
    fd = os.open('/dev/tty', os.O_RDWR)
    if fd >= 0:
        sys.stdin = os.fdopen(fd, 'r')

connectors = ['memflow-qemu-procfs', 'memflow-coredump', 'memflow-kvm']

registry = 'https://crates.io/api/v1/crates'

DB_FILE = 'db.json'

def is_root():
    try:
        return os.geteuid() == 0
    except:
        return False # this would occur on windows, but there we do not support system-wide connectors yet

output = expanduser('~/.local/lib/memflow') if not is_root() else '/opt/memflow'

try:
    os.makedirs(output)
except:
    pass

connector_cache = join(expanduser("~"), '.memflow', 'connectors')

try:
    os.makedirs(connector_cache)
except:
    pass

print(output)

def untar(dest_dir, buf):
    buf_file = io.BytesIO(buf)
    tar = tarfile.open(fileobj=buf_file, mode='r:gz')
    tar.extractall(dest_dir)

def get_crate_info(crate):
    r = request.urlopen(registry + '/' + crate).read()
    j = json.loads(r.decode('utf-8'))

    crate = j['crate']

    name = crate['id']
    version = crate['max_version']

    return (name, version)

def get_connector_db_path():
    return join(connector_cache, DB_FILE)

def get_installed_connectors():
    try:
        with open(get_connector_db_path()) as f:
            data = json.load(f)
            return data
    except:
        return {}

installed_connectors = get_installed_connectors()

def save_connectors():
    try:
        with open(get_connector_db_path(), 'w') as f:
            json.dump(installed_connectors, f)
    except:
        print('error saving connector db')

def cargo_build(connector):
    os.chdir(join(connector_cache, '%s-%s'%connector))
    print('executing cargo build')
    ret = subprocess.check_output('cargo build --release --all-features --message-format=json', shell=True)
    ret = ret.decode('utf-8')
    json_lines = [j for j in [json.loads(a) for a in ret.splitlines()] if j['reason'] == 'compiler-artifact' and j['target']['name'] == connector[0]]
    parsed = json_lines[-1]

    compiled_files = [a for a in parsed['filenames'] if not a.endswith('.rlib')]

    if len(compiled_files) > 1:
        print('multiple compiled outputs! Trying to match the right one, could break')
        compiled_files = [a for a in compiled_files if a.endswith('.so') or a.endswith('.dll') or a.endswith('.dyld')]

    return compiled_files[0]


def install_connector(connector):
    print('installing %s-%s'%connector)

    if connector[0] == DB_FILE:
        print('crate file can not be name the same as the database file!')
    else:
        r = request.urlopen("%s/%s/%s/download"%(registry, connector[0], connector[1])).read()
        untar(connector_cache, r)
        built_file = cargo_build(connector)
        base_name = basename(built_file)
        shutil.copy2(built_file, join(output, base_name))
        print(built_file)
        installed_connectors[connector[0]] = connector[1]
        save_connectors()

def select_crate(crates, val):
    if val.isdigit():
        i = int(val)
        if len(crates) > i:
            return crates[i]
        else:
            return None
    else:
        for c in crates:
            if c[0] == val:
                return c
        return None

def install_new_connectors():
    crates = []
    key_crates = {}

    print('Available crates:')
    
    for i, c in enumerate(connectors):
        info = get_crate_info(c)
        if info[0] in installed_connectors:
            installed_str = ' [installed %s]'%installed_connectors[info[0]]
        else:
            installed_str = ''
        print('%d) %s %s%s' % (i, info[0], info[1], installed_str))
        crates.append(info)
    
    print('')

    if len(sys.argv) > 2 and sys.argv[2] == '-n':
        inp = sys.argv[3:]
    else:
        inp = input('Select which crates to install (space separated list of numbers/names, or * for all): ')
        inp = inp.rstrip().split(' ')

    if len(inp) == 0:
        print('none selected!')
        return
    
    if inp[0] == '*':
        selected = [crates[i] for i in range(len(crates))]
    else:
        selected = [select_crate(crates, i) for i in inp]

    for c in selected:
        if c == None:
            print('invalid value entered!')
            return
    
    for c in selected:
        install_connector(c)

    print("done")

def update_connectors():

    if '-f' in sys.argv:
        force = True
    else:
        force = False

    for k in installed_connectors:
        nc = get_crate_info(k)
        if force or nc[1] != installed_connectors[k]:
            install_connector(nc)
        else:
            print('%s is already up to date (%s)'%(nc[0], nc[1]))

    print("done")

memflow_crate = get_crate_info('memflow')

try:
    print('Latest memflow version: %s\n'%memflow_crate[1])

    print('Installed connectors:')

    for k in installed_connectors:
        print('%s %s'%(installed_connectors[k], k))

    print('')
    
    if len(sys.argv) > 1:
        op = sys.argv[1]
    else:
        op = input('select operation (install, update): ')
    
    print(op)
    
    if op == "install":
        install_new_connectors()
    elif op == "update":
        update_connectors()
    else:
        print('invalid op, use install, or update')
except KeyboardInterrupt:
    pass
