use crate::core::filter::NameFilter;
use crate::core::resolver::ProxyResolver;
use crate::core::script::input::InputBuilder;
use axum::extract::ConnectInfo;
use axum::{
    body::Body,
    extract::{Request, State},
    response::{IntoResponse, Response}
};
use http_body_util::BodyExt;
use hyper::StatusCode;
use hyper_util::{client::legacy::connect::HttpConnector};
use log::info;
use std::net::SocketAddr;

type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;

pub async fn handler(
    State(client): State<Client>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();
    let path_query = request
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    info!("request url: {}", request.uri().to_string());

    let (parts, body) = request.into_parts();

    let bytes = body
        .collect()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_bytes();

    let input = InputBuilder::default()
        .with_uri(parts.uri.clone())
        .with_body(parts.clone(), bytes.clone())
        .with_pod_ip(addr.ip())
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let resolver = ProxyResolver::from_uri(parts.uri.clone(), NameFilter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result = resolver
        .evaluate(&input)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !result {
        let new_body = Body::from(bytes.clone());
        let new_request = Request::from_parts(parts, new_body);
        return Ok(client
            .request(new_request)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .into_response());
    }

    let (new_parts, new_body) = resolver
        .apply(parts.clone(), bytes.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let new_body = new_body
        .try_into()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(client
        .request(Request::from_parts(new_parts, new_body))
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .into_response())
}
