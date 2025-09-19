# Easymake

## Types

| Name | Description |
| ---- | ---------- |
| InFile | An input file definition |


### InFile

An input file definition

Type: String | { file: String, credentials: String }
An input file can be a local file or a file from an url.
If you need to specify credentials to get an url file, you can use the field file and credentials.

**Note**

If you use the variable in_files inside the shell action to target an url file, the value will be automatically replaced by
the donwloaded path. 

**Examples**

```yaml
{% raw %}
secrets:
    my_deep_secret:
      type: plain
      username: My_username
      password: My_password

targets:
    getting_local_file:
        steps:
            - description: Getting a local file
              shell:
                in_files:
                  - "{{ EMAKE_CWD_DIR }}/path_to_my_local_path"
                cmd: ls {{ in_files[0] }} # or ls {{ in_files }}
    getting_from_url:
        steps:
            - description: Getting from url
              shell:
                in_files:
                  - https://github.com/pchakour/easymake/archive/refs/heads/main.zip
                cmd: ls {{ in_files }}
    getting_from_url_with_credentials:
        steps:
            - description: Getting from url with credentials
              shell:
                in_files:
                  - file: https://github.com/pchakour/easymake/archive/refs/heads/main.zip  
                    credentials: {{ secrets:my_deep_secret }}
                cmd: ls {{ in_files }}
{% endraw %}
```


