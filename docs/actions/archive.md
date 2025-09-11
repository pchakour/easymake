# Easymake

## Action: archive

### Description

Compress your files as an archive


### Example

```yaml

targets:
  hello_world:
    steps:
        - description: 'Example files compression'
          archive:
            from:
                - from_path
            to: to_path

```

### Configuration options

| Name | Description | Type | Required |
| ---- | ----------- | -- | -- |
| from | Files to compress | Vec<[InFile](../types.md#infile)> | true |
| to | Destination | String | true |
| exclude | Exclude a list of file | Option<Vec<String>> | false |
