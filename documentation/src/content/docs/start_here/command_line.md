---
title: 🛠 Command line
description: Command line options
---

## Global options

`--cwd`

`--log_level`

`--help`


## Help command

Display all command line options with this command.

```sh
emake help
```

Or you can get help for a specific command like this example with the `build` command:

```sh
emake build --help
```

## Init a project

Initialize your project with this command in order to create your first Emakefile.

```sh
emake init
```

## Build a target
```sh
emake build [TARGET_PATH]
```
Use `--cwd [PATH]` to specify a project directory if not in the current folder.

## Clean

This command is usefull to clean all generated files and the `.emake` folder.
You can use this command with the option `--dry_run` to see files will be deleted.

```sh
emake clean
```

## Generate a dependency graph

Generate the graph of a specific target to visualize all dependencies

```sh
emake graph [TARGET_PATH] [FOLDER_PATH_TO_GENERATE_GRAPH]
```

## Keyring

Manage your secrets inside the local secret manager.

Keyring needs two informations to store your secret: a service and a name for your secret.

For exemple, if you want to store a secret about github like your password, you can store it with `github` as service and `my_greet_password` as name.
Feel free to use the service and name you want, keep in mind that these informations will be needed to retrieve or delete the secret.
One you will enter the following command, emake will ask you the secret to store.

Store the secret:

```sh
emake keyring store github my_greet_password
```

Delete the secret:

```sh
emake keyring remove github my_greet_password
```

Refer to the section [Reference/secrets/keyring](../../reference/secrets/keyring/) to learn how to use it inside your Emakefile