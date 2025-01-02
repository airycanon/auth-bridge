# Auth-Bridge

Auth-Bridge is an open-source project that provides proxy capabilities for Kubernetes Pods, enabling secure access to
tools such as GitLab and Harbor while automatically injecting specified credentials based on predefined policies.

## Features

- Dual-mode proxy support (forward/reverse) with automatic endpoint generation
- Multiple authentication methods support (Basic Auth, Bearer Token, Generic and Dynamic Auth)
- Policy-based credential injection
- Flexible authentication injection positions (header, query, body)
- Fine-grained access control with OPA policies
- Seamless Kubernetes Secret integration

## Quick Start

This guide will help you set up Auth-Bridge to proxy requests to an Nginx server that requires basic authentication.

### Requirements:

* A kubernetes cluster, you can use the following lightweight Kubernetes clusters:
    * [OrbStack](https://orbstack.dev)
    * [Kind](https://kind.sigs.k8s.io)

* Install cert-manager in the cluster:
    ```shell
    kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml
    ```

* Install skaffold cli to build and install auth-bridge:
    ```shell
    curl -Lo skaffold https://storage.googleapis.com/skaffold/releases/latest/skaffold-linux-amd64 && \
    sudo install skaffold /usr/local/bin/
    ```

### Deploy
* Deploy a test [Nginx server](examples/common/server.yaml) with basic auth.
* Deploy a [Test Client](examples/common/client.yaml) to access nginx.
* Deploy Auth-Bridge:
  ```shell
  skaffold run
  ```

### Configure Auth-Bridge: 

* Apply the basic example to the cluster:
  ```shell
  kubectl apply -f examples/basic/
  ```
  It should create the following Auth-Bridge resources:
    * [Secret](examples/basic/secret.yaml) - Store credentials that match the nginx's basic authentication.
    * [Policy](examples/basic/script.yaml) - Define an allow-all policy for proxy access.
    * [Proxy](examples/basic/proxy.yaml) - Configure proxy with the above secret and policy.  
  
  Now any pod using the Auth-Bridge proxy will automatically have basic authentication injected when accessing the Nginx
    server.  

### Test the Proxy Access

* Using forward proxy:
  ```shell
  # Direct access to target service with proxy setting
  kubectl exec -it test-client -- curl -x http://forward-proxy.auth-bridge:80 http://nginx.auth-bridge-example/auth
  ```

* Using reverse proxy (access through proxy endpoint):
  ```shell
  # Get reverse proxy endpoint
  ENDPOINT=$(kubectl get proxy -n auth-bridge-example basic-auth -o jsonpath='{.status.address.reverse.endpoint}')
  
  # Replace target host with reverse proxy endpoint
  kubectl exec -it test-client -- curl http://${ENDPOINT}/auth
  ```

Both methods should return `Hello from auth path` message.

## Configuration

### Secret Storage

Auth-Bridge supports two types of credential storage:

**Kubernetes Secrets**

```yaml
storage:
  secretRef:
    name: auth-secret
    namespace: default
```

**Raw Values**

```yaml
storage:
  raw:
    Authorization: "Bearer token123"
```

### Authentication Methods

Auth-Bridge supports four types of authentication:

**Basic Authentication**  
Automatically generates Basic Auth header from secret. The secret needs to provide `username` and `password` fields.

Example:

```yaml
auth:
  method:
    basicAuth: { }
  storage:
    secretRef:
      name: basic-auth
```

**Bearer Authentication**  
Automatically generates the Bearer Token header using token from secret. The secret needs to provide `token` field.

Example:

```yaml
auth:
  method:
    bearerToken: { }
  storage:
    secretRef:
      name: bearer-token 
```

**Generic Authentication**  
For injecting static authentication values. Useful when you need to directly specify the authentication value.

Fields:

- `position`: Where to inject the auth value
    - `header`: Add as HTTP header
    - `query`: Add as URL query parameter
    - `body`: Add to request body
- `key`: The key name to use (e.g., "Authorization" for headers)

The secret needs to provider the value matching configured key.

Examples:

* Header Auth:
    ```yaml
    method:
      generic:
        position: header
        key: Authorization
    storage:
      raw:
        Authorization: "Bearer static-token"
    ```
* Query Auth:
    ```yaml
    method:
      generic:
        position: query
        key: access_token
    storage:
      raw:
        access_token: "token123"
    ```

* Body Auth:
    ```yaml
    method:
      generic:
        position: body
        key: custom-key
    storage:
      raw:
        custom-key: "custom-value"
    ```

**Dynamic Authentication**  
Similar to Generic Authentication but uses a script to generate the auth value.

Fields:

- `position`: Where to inject the auth value
    - `header`: Add as HTTP header
    - `query`: Add as URL query parameter
    - `body`: Add to request body
- `key`: The key name to use (e.g., "Authorization" for headers)
- `script`: Name of the Auth Script that generates the auth value

The secret data will be available as script input to generate auth value.

Example:

* Dynamic Script Auth:
    ```yaml
    method:
      dynamic:
        position: header
        key: X-Auth
        script: generate-auth
    storage:
      secretRef:
        name: auth-secret
    ```

### Script

Auth-Bridge uses two types of scripts for authentication and access control:

### Auth Scripts

Auth Scripts generate authentication values for requests.

#### Input

- Secret fields are available as `input.<field_name>`

Example:

* script
    ```rego
    package proxy
    
    auth = input.token
    ```

#### Output

Must output a string value based on the script's query setting:

Example:

* script
    ```rego
    package proxy
    
    auth = "token123"
    ```
* output
    ```yaml
    engine:
      rego:
        query: "data.proxy.auth"
    ```

### Policy Scripts

Policy Scripts control when authentication should be applied.

#### Input

- `input.uri`: Target request URI
- `input.headers`: Request headers
- `input.query`: Query parameters
- `input.body`: Body parameters
- `input.meta`: Pod metadata

Example:

* script using query parameter
    ```rego
    package proxy
    
    allowed = true {
       input.query.name = "abc"
    }
    ```

* script using pod namespace
    ```rego
    package proxy
    
    allowed = true {
       contains(input.meta.namespace, "system")
    }
    ```

#### Output

Must output a bool value based on the script's query setting:

Example:

* script
    ```rego
    package proxy
    
    allowed = false
    ```
* output
    ```yaml
    engine:
      rego:
        query: "data.proxy.allowed"
    ```

### Advanced

You could see [Rego Language](https://www.openpolicyagent.org/docs/latest/policy-language/#what-is-rego) for more
advanced usages.

## Examples

For detailed examples, please check the following in the project repository:

- [Generic Authentication](examples/auth/generic)
- [Dynamic Authentication](examples/auth/dynamic)
- [Policy With HTTP](examples/policy/http)
- [Policy With Pod](examples/policy/pod)

Each example directory contains complete configuration files and usage instructions.

## Contributing

We welcome contributions of all forms! If you find a bug or have a feature request, please create an issue. If you'd
like to contribute code, please submit a pull request.

## License

Auth-Bridge is licensed under the [Apache License](https://www.apache.org/licenses/LICENSE-2.0). See
the [LICENSE](LICENSE) file for details.
