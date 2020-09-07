#!/usr/bin/python3

import sys, shutil, io, tarfile, json, os, subprocess, urllib.request as request
from os.path import expanduser, join, basename

connectors = ['memflow-qemu-procfs', 'memflow-coredump', 'memflow-daemon-connector', 'memflow-kvm']

registry = 'https://crates.io/api/v1/crates'

DB_FILE = 'db.json'

def is_root():
    return os.geteuid() == 0

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
            print(data)
    except:
        return []

installed_connectors = get_installed_connectors();

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
        untar(connector_cache, r);
        built_file = cargo_build(connector)
        base_name = basename(built_file)
        shutil.copy2(built_file, join(output, base_name))
        print(built_file)

def install_new_connectors():
    crates = []

    print('Available crates:')
    
    for i, c in enumerate(connectors):
        info = get_crate_info(c)
        print('%d) %s %s' % (i, info[0], info[1]))
        crates.append(info)

    print('')
    inp = input('Select which crates to install (space separated list, or * for all): ')
    
    if inp.rstrip() == '*':
        selected = [i for i in range(len(crates))]
    else:
        selected = [int(i) for i in inp.split(' ')]
    
    print(selected)

    for i in selected:
        install_connector(crates[i])

    print("done")

def update_connectors():

    for c in installed_connectors:
        nc = get_crate_info(c[0])
        if nc[1] != c[1]:
            install_connector(nc)

    print("done")

memflow_crate = get_crate_info('memflow')

try:
    print('Latest memflow version: %s\n'%memflow_crate[1])
    
    if len(sys.argv) > 1:
        op = sys.argv[1]
    else:
        op = ""
    
    print(op)
    
    if op == "install":
        install_new_connectors()
    elif op == "update":
        update_connectors()
    else:
        print('invalid op, use install, or update')
except KeyboardInterrupt:
    pass
