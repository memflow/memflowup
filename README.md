# memflow connector setup script

Requires Python.

This script is meant to provide a really quick way to setup various memflow connectors. Currently, it only supports installing connectors, and lacks features such as kernel module support, but it is to be expanded upon.

Run `curl -L https://raw.githubusercontent.com/memflow/memflowup/master/memflowup.py | python3` for interactive installation.

## Usage

Run interactively:

```
./memflowup.py
```

Install a set of connectors non-interactively:

```
./memflowup.py install memflow-qemu-procfs memflow-coredump
```

Update all connectors:

```
./memflowup.py update
```

Reinstall all connectors:

```
./memflowup.py update -f
```

Update all connectors and install a system-wide connector (only Unix supports system-wide installation):

```
./memflowup.py sys update install memflow-kvm
```
