use crate::vetis_adapter::{VetisAdapter, VetisAdapterConfig};
use caramelo::{expect, matchers::eq};
use deboa::{
    cert::{CertificateExt as _, ContentEncoding},
    request::get,
    HttpClient,
};
use deboa_smol::{cert::DeboaCertificate, Client};
use easyhttpmock::{
    config::EasyHttpMockConfig,
    matchers::{method, path},
    mock::{given, AsyncMatcherExt, Mock, StatusCodeExt},
    server::ServerAdapter as _,
    EasyHttpMock,
};
use http::{StatusCode, Version};
use macro_rules_attribute::apply;
use smol_macros::test;
use std::{error::Error, net::Ipv4Addr};

const CA_CERT: &[u8] = include_bytes!("../../certs/ca.der");
const SERVER_CERT: &[u8] = include_bytes!("../../certs/server.der");
const SERVER_KEY: &[u8] = include_bytes!("../../certs/server.key.der");

#[apply(test!)]
async fn test_mock_request() -> Result<(), Box<dyn Error>> {
    let server_cert = SERVER_CERT;
    let server_key = SERVER_KEY;

    let vetis_adapter_config = VetisAdapterConfig::builder()
        .hostname("localhost")
        .interface(
            "0.0.0.0"
                .parse()
                .unwrap(),
        )
        .protos(vec![Version::HTTP_2])
        .cert(server_cert.to_vec())
        .key(server_key.to_vec())
        .ca(CA_CERT.to_vec())
        .build();

    expect(vetis_adapter_config.ca()).to_be(eq(&Some(CA_CERT.to_vec())));
    expect(vetis_adapter_config.cert()).to_be(eq(&Some(server_cert.to_vec())));
    expect(vetis_adapter_config.key()).to_be(eq(&Some(server_key.to_vec())));
    expect(vetis_adapter_config.hostname()).to_be(eq("localhost"));
    expect(vetis_adapter_config.interface())
        .to_be(eq(&std::net::IpAddr::V4(Ipv4Addr::UNSPECIFIED)));

    let random_port = vetis_adapter_config.port();

    let config = EasyHttpMockConfig::<VetisAdapter>::builder()
        .server_config(vetis_adapter_config)
        .build();

    let Ok(mut server) = EasyHttpMock::new(config) else {
        panic!("Failed to create mock server");
    };

    expect(
        server
            .config()
            .port(),
    )
    .to_be(eq(random_port));

    let mock = Mock::of(
        given(path("/test").and(method("GET"))).will_return(
            StatusCode::OK
                .respond()
                .with_body(b"teste"),
        ),
    );

    server
        .register_mock(mock)
        .await?;

    let client = Client::builder()
        .certificate(DeboaCertificate::from_slice(CA_CERT, ContentEncoding::DER))
        .build();

    let request = get(server.url("/test"))?.build()?;
    let response = client
        .execute(request)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    server
        .stop()
        .await?;

    Ok(())
}
