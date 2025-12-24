---
title: archive
description: Compress your files as an archive
---
Compress your files as an archive


## Example

```yaml

targets:
  pre_archive:
    steps:
      - description: Creating file to archive
        shell:
          out_files: ["{{ EMAKE_WORKING_DIR }}/file_to_archive.txt"]
          cmd: echo 'Hello World !' > {{ out_files }}
  archive:
    deps:
        - pre_archive
    steps:
        - description: 'Example files compression'
          archive:
            from:
                - "{{ EMAKE_WORKING_DIR }}/file_to_archive.txt"
            to: "{{ EMAKE_OUT_DIR }}/archive.zip"

```

## Configuration options

| Name | Description | Type | Required |
| ---- | ----------- | -- | -- |
| from | Files to compress | Vec<[InFile](../types.md#infile)> | true |
| to | Destination | String | true |
| exclude | Exclude a list of file | Option<Vec<String>> | false |
