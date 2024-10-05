# memflowup - memflow setup tool

This tool is meant to provide a really quick way to setup various memflow components (connectors, OS plugins, utilities).

The recommended way is to install it through our automated script:
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.memflow.io | bash
```

Alternatively you can manually install it via cargo as well:
```sh
cargo install memflowup --force
```

## Basic Usage

See the help:
```sh
memflowup help
```

Pull all plugins:
```sh
memflowup pull --all
```

List all locally installed plugins:
```sh
memflowup plugins ls
```

List all available plugins in the default registry (http://registry.memflow.io):
```sh
memflowup plugins ls
```

Delete a plugin locally:
```sh
memflowup plugins remove coredump
```

Cleanup old versions of locally installed plugins:
```
memflowup plugins clean
```

Install a plugin from a github repo:
```
memflowup build https://github.com/memflow/memflow-coredump
```

Install a plugin from a folder:
```
cd memflow-coredump
memflowup build -p .
```

All commands additionally have a help (append `--help`) associated with them.


## Interacting with other registries

Memflowup features a configuration system that allows overriding some default properties.

To override the default registry run:
```
memflowup config set registry http://my-registry.io
memflowup config set pub_key_file /home/user/key_file.pub
```
All plugins in the memflow-registry are signed and the signature is checked by memflowup during the download process. Downloading from a custom registry requires setting up the according public key that was used for signing the files in the registry.

If you want to push to your own registry you also have to provide a token and the private key file which is used to sign plugins locally before publishing them.


## Migrate from memflowup 0.1

- Delete all system-wide installed plugins in `/usr/lib/memflow`
- Delete all installed plugins for the current user in `~/.local/lib/memflow`
- Delete the `/etc/memflowup` folder
- Reinstall all plugins via `memflowup pull --all`


## Troubleshooting:

- In case you are using Mac OS and encounter an error building proc-macro2 run `xcode-select --install`

