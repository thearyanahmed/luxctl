# lux


## Features

- [x] Authentication based on token
- [x] Store the credentials ~encrypted~ in a file somewhere. Need to add encryption down the line. Could be a good first issue or optional with GPG key
- [x] Implement get me endpoint
- [x] Run tests, individual
- [ ] Support for listing projects
- [ ] Setup CI pipeline for release

## Local Development

### API Token Setup

To test against the local API, create a `dev_token` file in the project root:

```bash
echo "YOUR_API_TOKEN_HERE" > dev_token
```

Get your token from the local API server (e.g., via login or from the database).

> **Note:** `dev_token` is gitignored and should never be committed.

### Local API Commands

```bash
# Get current user info (auth required)
make local:me

# Generic GET request to any endpoint
make local:get ENDPOINT=/ping
make local:get ENDPOINT=/projects
make local:get ENDPOINT=/projects/{slug}
make local:get ENDPOINT=/projects/{slug}/tasks
make local:get ENDPOINT=/tasks/{id}
```
