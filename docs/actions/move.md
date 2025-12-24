---
title: move
description: 
---
Move files


## Example

```yaml

{% raw %}
targets:
    extraction_example:
        steps:
            - description: Retrieve and move url folder
              move:
                from: 
                    - https://github.com/pchakour/easymake/archive/refs/heads/main.zip
                to: "{{ EMAKE_OUT_DIR }}/easymake_moved"
{% endraw %}

```

### Configuration options

| Name | Description | Type | Required |
| ---- | ----------- | -- | -- |
| from | A list of source files to move | Vec<[InFile](../types.md#infile)> | true |
| to | The destination to move source files. Can be a folder or a filename if the from property contains only one file. The folder will be automatically created if doesn't exist | String | true |
