---
title: copy
description: Copy files or folders to a specific destination
---
Copy files or folders to a specific destination


## Example

```yaml

targets:
    pre_copy:
        steps:
            - description: Generate hello world file
              shell:
                out_files: ["{{ EMAKE_WORKING_DIR }}/hello_world.txt"]
                cmd: touch {{ out_files }}
    copy:
        deps:
            - pre_copy
        steps:
            - description: Copy hello world file
              copy:
                from: 
                    - "{{ EMAKE_WORKING_DIR }}/hello_world.txt"
                to: "{{ EMAKE_OUT_DIR }}/hello_world.txt"

```

## Configuration options

| Name | Description | Type | Required |
| ---- | ----------- | -- | -- |
| from | A list of source files to copy | Vec<String> | true |
| to | A list of destination files. The number of destinations must be one to copy all sources in the destination or must match the number of destination | String | true |
