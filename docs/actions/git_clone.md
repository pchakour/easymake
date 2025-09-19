# Easymake

## Action: git_clone

### Description

Clone a git repository


### Example

```yaml

{% raw %}
targets:
    clone:
        steps:
            - description: Cloning a repository
              git_clone:
                url: https://github.com/githubtraining/training-manual.git
                destination: "{{ EMAKE_OUT_DIR }}/training"
{% endraw %}

```

### Configuration options

| Name | Description | Type | Required |
| ---- | ----------- | -- | -- |
| url | Url of the repository to clone | String | true |
| destination | Clone destination | String | true |
| commit | Commit to checkout. Could be a sha, a tag or a branch | Option<String> | false |
| username | Auth username when cloning with https | Option<String> | false |
| password | Auth password when cloning with https | Option<String> | false |
| ssh_key | Path to ssh key when cloning with ssh | Option<String> | false |
