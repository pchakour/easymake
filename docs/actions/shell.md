# Action: shell

## Description

Execute shell command.
The cmd property allow to use `in_files` and `out_files` as variables.

## Example

```yaml

targets:
    pre_hello_world:
        steps:
            - description: Generate hello world file
              shell:
                in_files: []
                out_files: ["{{ EMAKE_WORKING_DIR }}/hello_world.txt"]
                cmd: touch {{ out_files }}
    hello_world:
        deps:
            - pre_hello_world
        steps:
            - description: Echo example
              shell:
                cmd: echo 'hello world' >> {{ in_files }}
                in_files: ["{{ EMAKE_WORKING_DIR }}/hello_world.txt"]
                out_files: ["{{ EMAKE_WORKING_DIR }}/hello_world.txt"]
    

```

## Configuration options

| Name | Description | Type | Required |
| ---- | ----------- | -- | -- |
| cmd |  | String | false |
| in_files |  | Option < Vec < InFile > > | false |
| out_files |  | Option < Vec < String > > | false |
| checksum |  | Option < String > | false |
| clean |  | Option < String > | false |
