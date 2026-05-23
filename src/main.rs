use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{sleep, Duration};
use rand::Rng;

// =========================
// 🔧 MISMO build_command que el programa principal
// =========================
fn build_frame_tag(epc: &[u8]) -> Vec<u8> {
    // Construimos el payload igual al UR4 real:
    // [metadata_hi, metadata_lo, EPC bytes..., rssi_hi, rssi_lo, extra_hi, extra_lo]
    let mut payload: Vec<u8> = Vec::new();

    // byte[0] — tipo de tag (igual al hardware real)
    // 0x0C para EPC de 2 bytes, 0x14 para EPC de 4 bytes
    let tipo = match epc.len() {
        2  => 0x0C,
        4  => 0x14,
        _  => 0x30,
    };
    payload.push(tipo);

    // byte[1] — siempre 0x00
    payload.push(0x00);

    // byte[2..] — EPC real
    payload.extend_from_slice(epc);

    // ultimos 4 bytes — RSSI simulado aleatorio
    let mut rng = rand::thread_rng();
    payload.push(0xFD);
    payload.push(rng.gen_range(0x80..=0xFF)); // señal aleatoria
    payload.push(0x01);

    // Ahora envolvemos el payload en la trama UR4 real:
    // [A5][5A][len_hi][len_lo][83][...payload...][checksum][0D][0A]
    let comando: u8 = 0x83; // comando "tag detectado"
    let length = (8 + payload.len()) as u16;

    let mut checksum: u8 = ((length >> 8) as u8) ^ (length as u8) ^ comando;
    for b in &payload {
        checksum ^= b;
    }

    let mut frame = Vec::new();
    frame.push(0xA5);                    // cabecera
    frame.push(0x5A);                    // cabecera
    frame.push((length >> 8) as u8);     // longitud hi
    frame.push(length as u8);            // longitud lo
    frame.push(comando);                 // 0x83 = tag detectado
    frame.extend_from_slice(&payload);   // payload con EPC
    frame.push(checksum);                // checksum XOR
    frame.push(0x0D);                    // fin de trama \r
    frame.push(0x0A);                    // fin de trama \n

    frame
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:5084").await?;
    println!("📡 UR4 SIMULADO escuchando en puerto 5084...");

    // Esperar que el programa principal se conecte
    let (mut socket, addr) = listener.accept().await?;
    println!("✅ Cliente conectado desde {}", addr);

    // Leer y descartar los comandos de inicio que manda el programa principal
    // (0x60 modo lector y 0x82 iniciar inventario)
    let mut temp = [0u8; 64];
    socket.read(&mut temp).await?;
    println!("📨 Comandos de inicio recibidos, comenzando simulacion...");

    // Tags de prueba que vamos a rotar — los mismos que probaste fisicamente
    let tags: Vec<Vec<u8>> = vec![
        vec![0x01, 0x83],                    // tag corto  → EPC: "0183"
        vec![0x00, 0x41, 0x01, 0x02],        // tag medio  → EPC: "00410102"
        vec![0x00, 0x41, 0x00, 0x02],        // tag medio  → EPC: "00410002"
    ];

    let mut rng = rand::thread_rng();

    loop {
        // Elegir un tag aleatorio de la lista
        let tag = &tags[rng.gen_range(0..tags.len())];
        let frame = build_frame_tag(tag);

        let epc_hex = hex::encode(tag);
        println!("📡 Enviando TAG: {} → frame: {:02X?}", epc_hex, frame);

        match socket.write_all(&frame).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("❌ Cliente desconectado: {}", e);
                break;
            }
        }

        // Esperar entre 1 y 3 segundos para simular lecturas reales
        let espera = rng.gen_range(1..=3);
        sleep(Duration::from_secs(espera)).await;
    }

    Ok(())
}
