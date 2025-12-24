---
title: 🛠 Command line
description: Command line options
---

## Build a target
```sh
emake build [TARGET_PATH]
```
Use `--cwd [PATH]` to specify a project directory if not in the current folder.

## Clean
Removes the `.emake` folder.
```sh
emake clean
```

## Generate a dependency graph
```sh
emake graph [TARGET_PATH]
```
