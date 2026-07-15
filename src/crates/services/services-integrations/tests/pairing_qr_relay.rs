//! Pairing Qr Relay contract tests.

#![cfg(feature = "remote-connect")]

mod common;
use common::*;

#[tokio::test]
async fn remote_connect_pairing_primitives_live_in_services_owner() {
    let desktop = DeviceIdentity {
        device_id: "desktop-id".to_string(),
        device_name: "Desktop".to_string(),
        mac_address: "00:11:22:33:44:55".to_string(),
    };
    let mobile = DeviceIdentity {
        device_id: "mobile-id".to_string(),
        device_name: "Mobile".to_string(),
        mac_address: "66:77:88:99:AA:BB".to_string(),
    };

    let mut protocol = PairingProtocol::new(desktop);
    let payload = protocol.initiate("https://relay.example.com").await.unwrap();
    assert_eq!(protocol.state().await, PairingState::WaitingForScan);
    assert_eq!(payload.url, "https://relay.example.com");

    let mobile_keypair = KeyPair::generate();
    let challenge = protocol
        .on_peer_joined(&mobile_keypair.public_key_base64())
        .await
        .unwrap();
    let response = PairingProtocol::answer_challenge(
        &challenge,
        &mobile,
        Some("install-1".to_string()),
        Some("user-1".to_string()),
    );

    assert!(protocol.verify_response(&response).await.unwrap());
    assert_eq!(protocol.state().await, PairingState::Connected);
}

#[test]
fn remote_connect_qr_and_relay_primitives_live_in_services_owner() {
    let payload = QrPayload {
        room_id: "room 1".to_string(),
        url: "https://relay.example.com/socket".to_string(),
        device_id: "device/id".to_string(),
        device_name: "Desktop Device".to_string(),
        public_key: "public/key".to_string(),
        version: 1,
    };

    let url = QrGenerator::build_url(&payload, "https://mobile.example.com/", "zh-CN");
    assert!(url.starts_with("https://mobile.example.com/#/pair?"));
    assert!(url.contains("relay=wss%3A%2F%2Frelay.example.com%2Fsocket"));
    assert!(url.contains("lang=zh-CN"));

    let message = RelayMessage::CreateRoom {
        room_id: Some(payload.room_id),
        device_id: payload.device_id,
        device_type: "desktop".to_string(),
        public_key: payload.public_key,
    };
    let json = serde_json::to_value(message).expect("serialize relay message");
    assert_eq!(json["type"], "create_room");
    assert_eq!(json["device_type"], "desktop");
}
