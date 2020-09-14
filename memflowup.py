#!/usr/bin/python3

import sys, shutil, io, tarfile, json, os, subprocess, urllib.request as request
from os.path import expanduser, join, basename

args = sys.argv[1:]

# Setup stdout in case the script was piped in on *nix
if os.path.exists('/dev/tty'):
    fd = os.open('/dev/tty', os.O_RDWR)
    if fd >= 0:
        sys.stdin = os.fdopen(fd, 'r')

connectors = ['memflow-qemu-procfs', 'memflow-coredump', 'memflow-kvm']

registry = 'https://crates.io/api/v1/crates'

DB_FILE = 'db.json'

def is_posix():
    return os.name == 'posix'

def is_root():
    if is_posix():
        return os.geteuid() == 0
    else:
        return False

def make_dirs(path, as_root):
    try:
        if as_root and not is_root():
            subprocess.check_output('sudo mkdir -p %s'%path, shell=True)
        else:
            os.makedirs(path)
    except:
        pass

def copy_file(path, output, as_root):
    try:
        if as_root and not is_root():
            subprocess.check_output('sudo cp %s %s'%(path, output), shell=True)
        else:
            shutil.copy2(path, output)
        return True
    except:
        print('failed to copy file!')
        return False

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

class Database:
    def update_installed_connectors(self):
        try:
            with open(self.db_path) as f:
                self.installed_connectors = json.load(f)
        except:
            self.installed_connectors = {}

    def save_connectors(self):
        try:
            with open(self.tmp_db_path, 'w') as f:
                json.dump(self.installed_connectors, f)
            if self.tmp_db_path != self.db_path:
                copy_file(self.tmp_db_path, self.db_path, self.as_root)
            prev_connectors = self.installed_connectors
            self.update_installed_connectors()
            if prev_connectors != self.installed_connectors:
                print(prev_connectors)
                print(self.installed_connectors)
                print('failed to successfully write the db')
        except:
            print('error saving connector db')


    def __init__(self, as_root):
        self.as_root = as_root

        self.output = expanduser('~/.local/lib/memflow') if not as_root else '/usr/local/lib/memflow'
        make_dirs(self.output, as_root)

        # building is always done as current user
        self.connector_cache = join(expanduser("~"), '.memflow', 'connectors') if not is_root() else '/var/memflowup/connectors'
        make_dirs(self.connector_cache, as_root)

        db_dir = join(expanduser("~"), '.memflow', 'connectors') if not as_root else '/etc/memflowup'
        make_dirs(db_dir, as_root)

        if as_root and not is_root():
            make_dirs('/tmp', False)

        self.db_path = join(db_dir, DB_FILE)
        self.tmp_db_path = join('/tmp', DB_FILE) if not is_root() and as_root else self.db_path

        self.update_installed_connectors()

    def cargo_build(self, connector):
        os.chdir(join(self.connector_cache, '%s-%s'%connector))
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

    def install_connector(self, connector):
        print('installing %s-%s'%connector)

        if connector[0] == DB_FILE:
            print('crate file can not be name the same as the database file!')
        else:
            r = request.urlopen("%s/%s/%s/download"%(registry, connector[0], connector[1])).read()
            untar(self.connector_cache, r)
            built_file = self.cargo_build(connector)
            base_name = basename(built_file)
            install_path = join(self.output, base_name)
            if copy_file(built_file, install_path, self.as_root):
                print("installed under: " + install_path)
                self.installed_connectors[connector[0]] = connector[1]
                self.save_connectors()

user_db = Database(is_root());

dbs = {
    False: user_db
}

if is_root() or not is_posix():
    dbs[True] = user_db
else:
    dbs[True] = None

print("Default Install directory: " + user_db.output)
print("Default DB path: " + user_db.db_path)
print("Default Cache directory: " + user_db.connector_cache)
print()

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

def install_new_connectors(db):
    global args
    crates = []
    key_crates = {}

    print('Available crates:')

    for i, c in enumerate(connectors):
        info = get_crate_info(c)
        if info[0] in db.installed_connectors:
            installed_str = ' [installed %s]'%db.installed_connectors[info[0]]
        else:
            installed_str = ''
        print('%d) %s %s%s' % (i, info[0], info[1], installed_str))
        crates.append(info)

    print('')

    if len(args) > 0:
        inp = args
        args = []
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
        db.install_connector(c)

    print("done")

def update_connectors(db):
    global args

    if '-f' in args and args[0] == '-f':
        force = True
        args = args[1:]
    else:
        force = False

    for k in db.installed_connectors:
        nc = get_crate_info(k)
        if force or nc[1] != db.installed_connectors[k]:
            db.install_connector(nc)
        else:
            print('%s is already up to date (%s)'%(nc[0], nc[1]))

    print("done")

memflow_crate = get_crate_info('memflow')

try:
    print('Latest memflow version: %s\n'%memflow_crate[1])

    mode = False

    has_sysargs = len(args) > 0

    while True:

        if not dbs[mode]:
            dbs[mode] = Database(mode)

        db = dbs[mode]

        if not has_sysargs and len(args) == 0:

            print('Installed %s connectors:'%('system' if is_root() or mode else 'user'))

            for k in db.installed_connectors:
                print('%s %s'%(db.installed_connectors[k], k))

            print('')

            args = input('select operation (install, update, sys, user): ').split()

        op = args[0]
        args = args[1:]

        print(op)

        if op == "install":
            install_new_connectors(db)
        elif op == "update":
            update_connectors(db)
        elif op == "sys":
            mode = True
        elif op == "user":
            mode = False
        elif op == "q":
            break
        else:
            print('invalid op, use install, or update')

        if has_sysargs and len(args) == 0:
            break
except KeyboardInterrupt:
    pass
