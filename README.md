# Zenoh CLI

A work-in-progress CLI for [Zenoh](https://zenoh.io).

Zenoh CLI provides interactive and non-interactive modes. Interactive mode is aimed for scripting, and non-interactive mode is aimed for learning and debugging Zenoh.

https://github.com/user-attachments/assets/485d5fd9-1f55-47b1-b3f1-4bfe70ec501f

## Usage

You can install Zenoh CLI using the following command:

```bash
$ curl --proto '=https' --tlsv1.2 -LsSf https://github.com/unexge/zenoh-cli/releases/download/v0.1.0/zenoh-cli-installer.sh | sh
```

If you start Zenoh CLI without any arguments, it will start in interactive mode:

```bash
$ zenoh-cli
Zenoh CLI v0.1.0
> put myhome/kitchen/temp 25
ok
> _
```

You can just provide commands while invoking Zenoh CLI if you don't want the interactive mode:

```bash
$ zenoh-cli put myhome/kitchen/temp 25
ok
$ zenoh-cli subscribe myhome/**
...
```

## Available Commands

Zenoh CLI provides the following commands:

- `get <keyexpr>`: Get values associated with the given key expression.
- `put <keyexpr> <value>`: Put a value associated with the given key expression.
- `delete <keyexpr>`: Delete values associated with the given key expression.
- `subscribe <keyexpr>`: Subscribe to values associated with the given key expression.
- `zid`: Print the ID of the local Zenoh instance.
- `peers`: Print the list of connected peers.
- `routers`: Print the list of connected routers.
- `quit`: Quit the Zenoh CLI.
