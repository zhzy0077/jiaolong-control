use jiaolong_control::protocol::request;

#[test]
fn all_original_dll_request_headers_match() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/original-request-vectors.json")).unwrap();
    let vectors = vectors.as_array().unwrap();
    assert_eq!(vectors.len(), 512);
    for vector in vectors {
        let command = vector["command"].as_u64().unwrap() as u8;
        let set = vector["kind"].as_u64().unwrap() == 251;
        let output = request(command, if set { Some(0) } else { None });
        let hex: String = output.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            vector["hex"].as_str().unwrap(),
            "command {command}, set {set}"
        );
    }
}
