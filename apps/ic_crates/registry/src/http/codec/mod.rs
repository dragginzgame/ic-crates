//! Required external IC HTTP gateway Candid, independent of owned registry RPC.

use ic_crates_protocol::{HttpRequest, HttpResponse};
use ic_platform::candid::contract::{DecoderConfig, decode_args_with_config, encode_one};

pub(crate) fn decode_request(bytes: Vec<u8>) -> HttpRequest {
    // Preserve the previous CDK entrypoint's argument decoder and skipping quota.
    let mut config = DecoderConfig::new();
    config.set_skipping_quota(10_000);
    let (request,) = decode_args_with_config(&bytes, &config)
        .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()));
    request
}

pub(crate) fn encode_response(response: HttpResponse) -> Vec<u8> {
    encode_one(response).unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
}
