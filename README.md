# memflowup - memflow setup tool

This tool is meant to provide a really quick way to setup various memflow components (connectors, OS plugins, utilities).

The recommended way is to install it through our automated script:
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.memflow.io | sh
```

Alternatively you can manually install it via cargo as well:
```sh
cargo install memflowup --force
```

## Usage

See help:

```sh
memflowup -h
```

Run interactively:

```sh
memflowup interactive
```

Install a set of connectors non-interactively:

```sh
memflowup install memflow-qemu-procfs memflow-coredump
```

Same with development (0.2+) channel:

```sh
memflowup install -d memflow-qemu memflow-coredump
```

Update all connectors that are installed system-wide (`-s`) from development channel (`-d`):

```sh
memflowup update -s -d
```

Reinstall a connector:

```
memflowup install memflow-kvm -s -d --reinstall
```

## Migration from 0.1

- TODO: auto migration? -> ask user if he wants to run it
- Delete all system-wide installed plugins in /... -> ask
- Delete plugin database in /.. -> ask
- Purge all plugins that have no meta file -> ask
- Ask to redownload all plugins -> ask
- TODO: how do we know if migrations ran successfully and we do not have to run them again?

- Create a config file to store things like token / priv key file (additioanlly to cmdline switches)
  memflowup config token 123456
  memflowup config priv-key-file bla.pem -> will store fullpath in config
  memflowup config registry xyz.registry.io # overwrite default registry

- store current memflowup version in config file to see what migration steps need to be run


## Troubleshooting:

mac in case cc failed in proc-macro2
xcode-select --install
