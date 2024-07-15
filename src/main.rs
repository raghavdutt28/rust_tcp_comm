use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::broadcast,
};

async fn get_client_name(
    writer_part:&mut tokio::net::tcp::OwnedWriteHalf,
    reader:&mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    line:&mut String,
) -> io::Result<String> {
    writer_part.write_all(b"Please enter your name: ").await?;
    
    line.clear(); // Clear the line buffer before reading
    reader.read_line(line).await?;
    
    let client_name = line.trim().to_string();
    println!("{} has connected!", client_name);

    Ok(client_name)
}

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel(10);
    let listener = TcpListener::bind("localhost:8080").await.unwrap();

    loop {
        let sender = tx.clone();
        let mut receiver = tx.subscribe();
        let (socket, peer_addr) = listener.accept().await.unwrap();
        
        tokio::spawn(async move {
            let (reader_part, mut writer_part) = socket.into_split();
            let mut reader = BufReader::new(reader_part);
            let mut line = String::new();

            let client_name = match get_client_name(&mut writer_part, &mut reader, &mut line).await {
                Ok(name) =>{
                    if name.is_empty() {
                        eprintln!("Error getting client name");
                    return;
                    }
                    name
                },
                Err(e) => {
                    eprintln!("Error getting client name: {}", e);
                    return;
                }
            };

            // Clear the line buffer for further use
            line.clear();

            loop {
                tokio::select! {
                    result = reader.read_line(&mut line) => {
                        match result {
                            Ok(0) => {
                                println!("{} disconnected", client_name);
                                break;
                            }
                            Ok(_) => {
                                let msg = format!("{}: {}", client_name, line);
                                println!("{}", msg);
                                if let Err(e) = sender.send((msg.clone(), peer_addr)) {
                                    eprintln!("Failed to send message: {}", e);
                                }
                                line.clear();
                            }
                            Err(e) => {
                                eprintln!("Error reading from socket: {}", e);
                                break;
                            }
                        }
                    },
                    result = receiver.recv() => {
                        match result {
                            Ok((msg, other_addr)) => {
                                if peer_addr != other_addr {
                                    if let Err(e) = writer_part.write_all(msg.as_bytes()).await {
                                        eprintln!("Error writing to socket: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error receiving message: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
        });
    }
}
