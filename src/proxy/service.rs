use crate::proxy::layers::LogLayer;
use rama::{
    Layer, Service,
    error::{BoxError, ErrorContext},
    extensions::{Extension, ExtensionsRef},
    http::{
        Body, Request, Response, StatusCode, Version,
        client::EasyHttpWebClient,
        layer::{
            compression::CompressionLayer,
            decompression::DecompressionLayer,
            map_response_body::MapResponseBodyLayer,
            remove_header::{RemoveRequestHeaderLayer, RemoveResponseHeaderLayer},
            required_header::AddRequiredRequestHeadersLayer,
            trace::TraceLayer,
            traffic_writer::{self, RequestWriterLayer},
            upgrade::{UpgradeResponse, Upgraded},
        },
        service::web::response::IntoResponse,
    },
    layer::ConsumeErrLayer,
    net::{
        http::RequestContext,
        proxy::ProxyTarget,
        tls::{
            ApplicationProtocol, DataEncoding, SecureTransport,
            client::{ServerVerifyMode, TlsClientConfig},
            server::{
                CacheKind, ServerAuth, ServerAuthData, ServerCertIssuerData, ServerCertIssuerKind,
                ServerConfig,
            },
        },
    },
    service::service_fn,
    telemetry::tracing,
    tls::boring::{
        client::{BoringClientConfigExt, EmulateTlsProfileLayer},
        server::{TlsAcceptorData, TlsAcceptorLayer},
    },
    ua::{
        layer::emulate::{
            UserAgentEmulateHttpConnectModifierLayer, UserAgentEmulateHttpRequestModifierLayer,
            UserAgentEmulateLayer,
        },
        profile::UserAgentDatabase,
    },
    utils::str::NonEmptyStr,
};
use std::{convert::Infallible, sync::Arc};

#[derive(Debug, Clone, Extension)]
pub struct ProxyState {
    pub tls_acceptor: Option<TlsAcceptorData>,
    pub user_agent: Arc<UserAgentDatabase>,
}

type BaseService = rama::service::BoxService<Request, Response, Infallible>;

pub fn new_http_proxy<L>(
    ctx: &ProxyState,
    layers: L,
) -> impl Service<Request, Output = Response, Error = Infallible> + Clone
where
    L: Layer<BaseService> + Clone + Send + Sync + 'static,
    L::Service:
        Service<Request, Output = Response, Error = Infallible> + Clone + Send + Sync + 'static,
{
    let base = (
        MapResponseBodyLayer::new(Body::new),
        TraceLayer::new_for_http(),
        ConsumeErrLayer::default(),
        UserAgentEmulateLayer::new(ctx.user_agent.clone())
            .with_try_auto_detect_user_agent(true)
            .with_is_optional(true),
        CompressionLayer::new(),
        AddRequiredRequestHeadersLayer::new(),
        EmulateTlsProfileLayer::new(),
    )
        .into_layer(service_fn(http_proxy))
        .boxed();
    let base = layers.into_layer(base).boxed();
    LogLayer.into_layer(base)
}

pub async fn http_connect_accept(
    req: Request,
) -> Result<UpgradeResponse<Request, Response>, Response> {
    match RequestContext::try_from(&req).map(|ctx| ctx.host_with_port()) {
        Ok(authority) => {
            tracing::info!(
                server.address = %authority.host,
                server.port = authority.port,
                "accept CONNECT (lazy): insert proxy target into context",
            );
            req.extensions().insert(ProxyTarget(authority));
        }
        Err(err) => {
            tracing::error!("error extracting authority: {err:?}");
            return Err(StatusCode::BAD_REQUEST.into_response());
        }
    }

    let extensions = req.extensions().clone();
    Ok(UpgradeResponse {
        response: StatusCode::OK.into_response(),
        request: req,
        extensions,
    })
}

pub async fn http_connect_proxy<L>(upgraded: Upgraded, layers: L) -> Result<(), Infallible>
where
    L: Layer<BaseService> + Clone + Send + Sync + 'static,
    L::Service:
        Service<Request, Output = Response, Error = Infallible> + Clone + Send + Sync + 'static,
{
    let ctx = upgraded
        .extensions()
        .get_ref::<ProxyState>()
        .expect("proxy context");
    let http_service = new_http_proxy(ctx, layers);

    let executor = rama::rt::Executor::default();

    let mut http_tp = rama::http::server::HttpServer::auto(executor);
    http_tp.h2_mut().set_enable_connect_protocol();

    let http_transport_service = http_tp.service(http_service);

    let tls_acceptor = match ctx.tls_acceptor.as_ref() {
        Some(tls_acceptor) => tls_acceptor.clone(),
        None => {
            tracing::error!("missing TLS acceptor for CONNECT MITM");
            return Ok(());
        }
    };
    let https_service = TlsAcceptorLayer::new(tls_acceptor)
        .with_store_client_hello(true)
        .into_layer(http_transport_service);

    if let Err(err) = https_service.serve(upgraded).await {
        tracing::error!("https service failed with an error: {err}");
    }

    Ok(())
}

pub async fn http_proxy(req: Request) -> Result<Response, Infallible> {
    let base_tls_config = req
        .extensions()
        .get_ref::<SecureTransport>()
        .and_then(|st| st.client_hello())
        .map(TlsClientConfig::new_from_client_hello)
        .unwrap_or_else(TlsClientConfig::default_http)
        .with_server_verify(ServerVerifyMode::Disable);

    let executor = rama::rt::Executor::default();

    let client = EasyHttpWebClient::connector_builder()
        .with_default_transport_connector()
        .with_tls_proxy_support_using_boringssl()
        .with_proxy_support()
        .with_tls_support_using_boringssl_and_default_http_version(base_tls_config, Version::HTTP_11)
        .with_custom_connector(UserAgentEmulateHttpConnectModifierLayer::default())
        .with_default_http_connector(executor.clone())
        .build_client()
        .with_jit_layer((
            UserAgentEmulateHttpRequestModifierLayer::default(),
            RequestWriterLayer::stdout_unbounded(
                &executor,
                Some(traffic_writer::WriterMode::Headers),
            ),
        ));

    let client = (
        RemoveResponseHeaderLayer::hop_by_hop(),
        RemoveRequestHeaderLayer::hop_by_hop(),
        MapResponseBodyLayer::new(Body::new),
        DecompressionLayer::new(),
    )
        .into_layer(client);

    match client.serve(req).await {
        Ok(resp) => Ok(resp),
        Err(err) => {
            tracing::error!("error in client request: {err:?}");
            Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

pub fn new_tls_acceptor(
    ca_key_path: String,
    ca_cert_path: String,
) -> Result<TlsAcceptorData, BoxError> {
    let ca_key_pem =
        std::fs::read_to_string(&ca_key_path).context("read MITM_CA_KEY_PATH (PEM)")?;
    let ca_cert_pem =
        std::fs::read_to_string(&ca_cert_path).context("read MITM_CA_CERT_PATH (PEM)")?;

    let server_auth = ServerAuth::CertIssuer(ServerCertIssuerData {
        kind: ServerCertIssuerKind::Single(ServerAuthData {
            private_key: DataEncoding::Pem(
                NonEmptyStr::try_from(ca_key_pem).map_err(BoxError::from)?,
            ),
            cert_chain: DataEncoding::Pem(
                NonEmptyStr::try_from(ca_cert_pem).map_err(BoxError::from)?,
            ),
            ocsp: None,
        }),
        cache_kind: CacheKind::default(),
    });

    let tls_server_config = ServerConfig {
        application_layer_protocol_negotiation: Some(vec![
            ApplicationProtocol::HTTP_2,
            ApplicationProtocol::HTTP_11,
        ]),
        ..ServerConfig::new(server_auth)
    };
    tls_server_config
        .try_into()
        .context("create tls server config")
}
