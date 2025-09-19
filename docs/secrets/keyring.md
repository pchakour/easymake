# Easymake

## Secret: keyring

Get secrets from the local keyring

Local keyring storage, see command emake keyring to store or clear password

Example:
```yaml

secrets:
  my_deep_secret:
    type: keyring
    service: service_name
    name: secret_name

```