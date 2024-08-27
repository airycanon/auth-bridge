# Auth-Bridge

Auth-Bridge is an open-source project that provides proxy capabilities for Kubernetes Pods, enabling secure access to tools such as GitLab and Harbor while automatically injecting specified credentials based on predefined policies.

## Features

- Proxy access for Kubernetes Pods
- Support for various common tools like GitLab, Harbor, etc.
- Policy-based automatic credential injection
- Flexible proxy configuration based on [Open Policy Agent](https://www.openpolicyagent.org).


## Installation

Before installing Auth-Bridge, ensure you have the following prerequisites:

#### A Kubernetes cluster
You can use [Kind](https://kind.sigs.k8s.io) for local development.   
Alternatively [OrbStack](https://orbstack.dev) provides a lightweight Kubernetes environment.


#### Install cert-manager
```shell
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml
```

#### Install skaffold
```bash
curl -Lo skaffold https://storage.googleapis.com/skaffold/releases/latest/skaffold-linux-amd64 && \
sudo install skaffold /usr/local/bin/
```

#### Install

```bash
skaffold deploy
```

## Configuration

Auth-Bridge is configured by ProxyPolicy and Secret. Ensure that your ProxyPolicy and associated Secret are correctly configured based on your chosen authentication method and validation rules.

Here's a basic configuration example:

```yaml
apiVersion: auth-bridge.dev/v1alpha1
kind: ProxyPolicy
metadata:
  name: basic-auth
  namespace: default
spec:
  auth:
    method: basicAuth
    secret:
      reference:
        name: basic-auth
        namespace: <secret namespace>
  rules:
    - name: basic-rule
      validate: | 
        package proxy
        
        default allow = true
---
apiVersion: v1
kind: Secret
metadata:
  name: basic-auth
  namespace: default
type: Opaque
stringData:
  username: username
  password: password
```

#### Field Definition

* `auth.method`
   This field specifies the authentication method to be used. It can be set to either:
    - `basicAuth`: For basic authentication using a username and password.
    - `bearerToken`: For authentication using a bearer token.

* `auth.secret.reference`
   This field refers to the Kubernetes Secret containing the authentication credentials.
    - For `basicAuth`, the referenced Secret data must contain `username` and `password`
    - For `bearerToken`, the referenced Secret data must contain `token`

* `rules.validate`:
   This field contains the Open Policy Agent (OPA) validation rule. The OPA script must include a boolean variable 
   named `allow`, which determines whether the secret should be injected during proxying based on this rule. For example:

     ```
     package proxy

     default allow = false

     allow {
       input.uri == "example.com"
     }
     ```
  
#### Advanced
The OPA script also has access to an input object that contains information about the target request and the pod.
You can use input.<field> in your OPA script to make decisions. The available fields include:

- input.uri: The URI of the target request
- input.query: The query parameters of the target request
- input.body: The body of the target request
- input.meta: Metadata of the pod making the request

In this example, the secret will only be injected if the request host is "example.com".

      ```
      package proxy
        
      default allow = false
        
      allow {
        contains(input.uri, "example.com")
        input.meta.namespace == "allowed-namespace"
        input.query.action == "read"
      }
     ```

## Usage
Using Auth-Bridge involves several key steps:

#### Configure ProxyPolicy
Create a ProxyPolicy resource to define your proxy rules:
```yaml
apiVersion: auth-bridge.dev/v1alpha1
kind: ProxyPolicy
metadata:
  name: proxy-policy
spec:
  auth:
    method: basicAuth
    secret:
      reference:
        name: <secret name>
        namespace: <secret namespace>
  rules:
    - name: <rule name>
      validate: <rule opa>
```

#### Create Secret

Create a Secret with correct credentials based on your policy auth method: 

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: credentials
  namespace: default
type: Opaque
stringData:
  username: <username>
  password: <password>
```

#### Set proxy
To enable the Auth-Bridge proxy, set the following environment variables for your application:
```shell
HTTP_PROXY=http://auth-bridge-proxy.auth-bridge:80
HTTPS_PROXY=http://auth-bridge-proxy.auth-bridge:80
http_proxy=http://auth-bridge-proxy.auth-bridge:80
https_proxy=http://auth-bridge-proxy.auth-bridge:80
```
the proxy host `auth-bridge-proxy.auth-bridge` here follows the Kubernetes service naming convention:`<service-name>.<namespace>`

For a more detailed demonstration of how these steps come together, please refer to the [examples](examples).

## Contributing

We welcome contributions of all forms! If you find a bug or have a feature request, please create an issue. If you'd like to contribute code, please submit a pull request.

## License

Auth-Bridge is licensed under the [Apache License](https://www.apache.org/licenses/LICENSE-2.0). See the [LICENSE](LICENSE) file for details.