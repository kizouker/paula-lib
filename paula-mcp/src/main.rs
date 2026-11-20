use paula_mcp::Server;

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut server = Server::new();
    if let Err(e) = server.run(stdin.lock(), stdout.lock()) {
        eprintln!("paula-mcp: {e}");
        std::process::exit(1);
    }
}
