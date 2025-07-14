use std::{collections::HashMap, net::SocketAddr, time::Instant};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    msg_type: String,
    client_id: String,
    data: Option<String>
}

#[derive(Debug)]
struct ClientInfo {
    client_addr: SocketAddr,
    timestamp: Instant
}

#[tokio::main]
async fn main() -> Result<()> {
    // bind an ip to udpsocket
    let socket = UdpSocket::bind("0.0.0.0:8080")
        .await?;
    println!("Rendezvous server listening on 0.0.0.0:8080");

    // create a hashmap of clients

    let mut clients = HashMap::<SocketAddr, ClientInfo>::new();
    // create buffer
    let mut buf = [0; 1024];
    
    // loop
    loop {
        let (len, client_addr) = socket.recv_from(&mut buf)
            .await?;

        // get the data
        let data = &buf[..len];

        match serde_json::from_slice::<Message>(data) {
            Ok(msg) => {
                println!("Received from {}: {:?}", client_addr, msg);
                println!("Message type: {}", msg.msg_type);
                match msg.msg_type.as_str() {
                    "register" => {
                        clients.insert(msg.client_id.clone().parse::<SocketAddr>()?, ClientInfo { 
                            client_addr, 
                            timestamp: Instant::now() 
                        });

                        let response = Message {
                            msg_type: String::from("registered"),
                            client_id: msg.client_id.clone(),
                            data: Some(format!("Registered as {}", msg.client_id))
                        };

                        let response_data = serde_json::to_vec(&response)?;
                        socket.send_to(&response_data, client_addr)
                            .await?;

                        if clients.len() >= 2 {
                            println!("Have {} clients, attempting to pair...", clients.len());
                            pair_clients(&socket, &clients)
                                .await?;
                        }
                    },
                    _ => {
                        println!("Unknown message type: {}", msg.msg_type);
                    }
                }
            }
            Err(e) => {
                println!("Failed to parse messagr from {}: {}", client_addr, e);
                println!("Raw data {:?}", String::from_utf8_lossy(data));
            }
        }
    }    
    Ok(())
}

async fn pair_clients(socket: &UdpSocket, clients: &HashMap<SocketAddr, ClientInfo>) -> Result<()> {
    let client_list: Vec<_> = clients.iter().collect();

    if client_list.len() >= 2 {
        let (id1, info1) = client_list[0];
        let (id2, info2) = client_list[1];

        // tell client 1 about client 2
        let msg1 = Message {
            msg_type: String::from("peer_info"),
            client_id: id2.to_string(),
            data: Some(info2.client_addr.to_string())
        };

        let msg2 = Message {
            msg_type: String::from("peer_info"),
            client_id: id1.to_string(),
            data: Some(info1.client_addr.to_string())
        };

        let msg1_data = serde_json::to_vec(&msg1)?; 
        let msg2_data = serde_json::to_vec(&msg2)?; 

        socket.send_to(&msg1_data, id2).await?;
        socket.send_to(&msg2_data, id1).await?;

        println!("Paired {} ({}) with {} ({})", id1, info1.client_addr, id2, info2.client_addr);

    }
    Ok(())
}