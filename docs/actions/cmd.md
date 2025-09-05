# Action: cmd

## Description

Execute shell command.
The cmd property allow to use `in_files` and `out_files` as variables.

## Example

```yaml

targets:
    pre_hello_world:
        steps:
            - description: Generate hello world file
              in_files: []
              out_files: ["{{ EMAKE_WORKING_DIR }}/hello_world.txt"]
              cmd: touch {{ out_files }}
    hello_world:
        deps:
            - pre_hello_world
        steps:
            - description: Echo example
              in_files: ["{{ EMAKE_WORKING_DIR }}/hello_world.txt"]
              out_files: ["{{ EMAKE_WORKING_DIR }}/hello_world.txt"]
              cmd: echo 'hello world' >> {{ in_files }}
    

```

## Configuration options

| Name | Description | Type | Required |
| ---- | ----------- | -- | -- |
