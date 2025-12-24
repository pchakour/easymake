---
title: remove
description: Remove a list of paths
---
Remove a list of paths


## Example

```yaml

targets:
    pre_remove:
        steps:
            - description: Creating a file to remove
              shell:
                out_files:
                    - "{{ EMAKE_OUT_DIR }}/hello.txt"
                cmd: echo 'hello' > {{ out_files }}
    remove:
        steps:
            - description: Remove file
              remove:
                paths:
                    - "{{ EMAKE_OUT_DIR }}/hello.txt"

```

## Configuration options

| Name | Description | Type | Required |
| ---- | ----------- | -- | -- |
| paths | List of path to remove. Could be folders or files | Vec<String> | true |
