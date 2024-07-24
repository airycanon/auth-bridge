use std::collections::{BTreeMap};
use http_body_util::{Collected, Full};
use hudsucker::{Body, HttpContext, HttpHandler, RequestOrResponse};
use hudsucker::hyper::{Request, Response, StatusCode};
use anyhow::Result;
use bytes::Bytes;
use hudsucker::tokio_tungstenite::tungstenite::http::Method;
use hyper::http::request::Parts;
use regorus::Value;
use crate::apis::{
    pod_meta,
    proxy_policy::ProxyPolicy,
};
use kube::{api::{Api, ListParams}, Client, ResourceExt};


#[derive(Clone, Default)]
pub struct PolicyHandler;

impl HttpHandler for PolicyHandler {
    async fn handle_request(&mut self, ctx: &HttpContext, req: Request<Body>) -> RequestOrResponse {
        if req.method() == Method::CONNECT {
            return RequestOrResponse::Request(req);
        }

        let (parts, body) = req.into_parts();
        let bytes = match http_body_util::BodyExt::collect(body).await.map(Collected::to_bytes) {
            Ok(bytes) => bytes,
            Err(err) => {
                return handle_parse_error(err)
            }
        };

        let parts_clone = parts.clone();
        let body_clone = Body::from(Full::from(bytes.clone()));

        let query = match parse_query(parts).await {
            Ok(query) => query,
            Err(err) => {
                return handle_parse_error(err)
            }
        };

        let content_type = parts_clone.headers.get(hyper::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok());

        let body = match content_type {
            Some(t) if t.starts_with("application/x-www-form-urlencoded") => {
                match parse_form(bytes).await {
                    Ok(body) => body,
                    Err(err) => {
                        return handle_parse_error(err)
                    }
                }
            }
            Some(t) if t.starts_with("application/json") => {
                match parse_json(bytes).await {
                    Ok(form) => form,
                    Err(err) => {
                        return handle_parse_error(err)
                    }
                }
            }
            _ => Value::new_object(),
        };

        let client = Client::try_default().await.map_err(handle_api_error).unwrap();
        let api = Api::<ProxyPolicy>::all(client);

        let params = ListParams::default();
        let policies = api.list(&params).await.map_err(handle_api_error).unwrap();

        let mut input: BTreeMap<Value, Value> = BTreeMap::new();
        input.insert(Value::from("query"), query);
        input.insert(Value::from("body"), body);

        let ip = ctx.client_addr.ip();
        if let Some(meta) = pod_meta::find(&ip.to_string()) {
            input.insert(Value::from("meta"), meta.as_input());
        }

        for item in policies.iter() {
            match eval_policy(&item, &input) {
                Ok(allow) if !allow => {
                    let mut res = Response::new(Body::from(format!("deny to pass policy: {}", item.name_any())));
                    *res.status_mut() = StatusCode::FORBIDDEN;
                    return RequestOrResponse::Response(res);
                }
                Err(err) => {
                    let mut res = Response::new(Body::from(format!("failed to eval policy: {}, err: {}", item.name_any(), err)));
                    *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    return RequestOrResponse::Response(res);
                }
                _ => continue
            }
        }

        let req_clone = Request::from_parts(parts_clone, body_clone);
        return RequestOrResponse::Request(req_clone);
    }
}

fn eval_policy(policy: &ProxyPolicy, input: &BTreeMap<Value, Value>) -> Result<bool> {
    for rule in policy.spec.rules.iter() {
        if !rule.eval(input)? {
            return Ok(false);
        }
    }

    Ok(true)
}

async fn parse_json(bytes: Bytes) -> Result<Value> {
    let str = std::str::from_utf8(bytes.iter().as_slice())?;
    Value::from_json_str(str)
}

async fn parse_form(bytes: Bytes<>) -> Result<Value> {
    let map: BTreeMap<Value, Value> = url::form_urlencoded::parse(bytes.iter().as_slice())
        .into_owned()
        .map(|(k, v)| (Value::from(k), Value::from(v)))
        .collect();

    let v = Value::from(map);
    Ok(v)
}

async fn parse_query(parts: Parts) -> Result<Value> {
    let map: BTreeMap<Value, Value> = parts.uri
        .query()
        .map(|v| {
            url::form_urlencoded::parse(v.as_bytes())
                .into_owned()
                .map(|(k, v)| (Value::from(k), Value::from(v)))
                .collect()
        })
        .unwrap_or_else(BTreeMap::new);

    let v = Value::from(map);
    Ok(v)
}

fn handle_parse_error<E: std::fmt::Display>(err: E) -> RequestOrResponse {
    let error_message = err.to_string();
    let mut res = Response::new(Body::from(error_message));
    *res.status_mut() = StatusCode::BAD_REQUEST;
    RequestOrResponse::Response(res)
}

fn handle_api_error<E: std::fmt::Display>(err: E) -> RequestOrResponse {
    let error_message = err.to_string();
    let mut res = Response::new(Body::from(error_message));
    *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    RequestOrResponse::Response(res)
}