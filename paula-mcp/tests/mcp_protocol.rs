use paula_mcp::Server;

fn response(request: &str) -> Option<serde_json::Value> {
    let mut server = Server::new();
    let mut output = Vec::new();
    server.run(request.as_bytes(), &mut output).unwrap();
    let line = output.split(|&b| b == b'\n').next().unwrap_or_default();
    serde_json::from_slice(line).ok()
}

#[test]
fn initialize_and_tools_list() {
    let init = response("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n").unwrap();
    assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
    assert!(init["result"]["capabilities"]["tools"].is_object());

    let list = response("{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{}}\n").unwrap();
    let tools = list["result"]["tools"].as_array().unwrap();
    let names: Vec<_> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["lint_score", "compile_score", "render_wav", "module_info", "propose_pattern"]);
}

#[test]
fn malformed_and_unknown_requests() {
    let bad = response("{bad json\n").unwrap();
    assert_eq!(bad["error"]["code"], -32700);

    let unknown = response("{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"no_such_method\"}\n").unwrap();
    assert_eq!(unknown["error"]["code"], -32601);

    let unknown_tool = response("{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"no_such_tool\",\"arguments\":{}}}\n").unwrap();
    assert_eq!(unknown_tool["result"]["isError"], true);
}
