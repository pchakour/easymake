---
title: yaml
description: Create or edit yaml files
---
Create or edit yaml files


## Example

```yaml

targets:
    yaml:
        steps:
            - description: Testing yaml action
              yaml:
                to: "{{ EMAKE_OUT_DIR }}/yaml_action.yml"
                set:
                    version: 0.1
                    name: yaml
                    type: action 

```

## Configuration options

| Name | Description | Type | Required |
| ---- | ----------- | -- | -- |
| from | The path to the yaml file to edit. Not mandatory if the parameter `to` is specified | Option<String> | false |
| to | Specify a path to save the yaml file. Not mandatory if the parameter `from` is specified | Option<String> | false |
| set | Value to set inside the file. Use null if you want delete the key | Value | true |
