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
