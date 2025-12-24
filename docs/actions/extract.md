---
title: extract
description: Support archive are: zip, tar.gz and tar.xz
---
Extract archive
Support archive are: zip, tar.gz and tar.xz

## Example

```yaml

{% raw %}
targets:
    extraction_example:
        steps:
            - description: Retrieve and extract archive from url
              extract:
                from: 
                    - https://github.com/pchakour/easymake/archive/refs/heads/main.zip
                to:
                    - "{{ EMAKE_OUT_DIR }}"
                out_files:
                    - "{{ glob('${EMAKE_OUT_DIR}/main/**/*') }}"
{% endraw %}

```

### Configuration options

| Name | Description | Type | Required |
| ---- | ----------- | -- | -- |
| from | Archive to extract, can be an url | [InFile](../types.md#infile) | true |
| to | Folder in which extract the archive | String | true |
| out_files | To register extracted file in the cache. Allow to execute again the extraction if a file from out_files change | Option<Vec<String>> | false |
