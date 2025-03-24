use ibverbs::{devices, ibv_qp_type, ibv_wc};
use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    let ctx = devices()
        .unwrap()
        .iter()
        .next()
        .expect("no rdma device available")
        .open()
        .unwrap();
    let cq = ctx.create_cq(16, 0).unwrap();
    let pd = ctx.alloc_pd().unwrap();
    let qp_builder = pd
        .create_qp(&cq, &cq, ibv_qp_type::IBV_QPT_RC)
        .set_gid_index(1)
        .build()
        .unwrap();
    let local_endpoint = qp_builder.endpoint();

    let listener = TcpListener::bind("0.0.0.0:12345").unwrap();
    let (mut stream, _) = listener.accept().unwrap();

    let local_endpoint_bytes = bincode::serialize(&local_endpoint).unwrap();
    let len = local_endpoint_bytes.len() as u32;
    stream.write_all(&len.to_be_bytes()).unwrap();
    stream.write_all(&local_endpoint_bytes).unwrap();

    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).unwrap();
    let len = u32::from_be_bytes(len_bytes) as usize;
    let mut remote_endpoint_bytes = vec![0u8; len];
    stream.read_exact(&mut remote_endpoint_bytes).unwrap();
    let remote_endpoint = bincode::deserialize(&remote_endpoint_bytes).unwrap();

    let mut qp = qp_builder.handshake(remote_endpoint).unwrap();

    let mut mr_server = pd.allocate::<u8>(12).unwrap();
    let message = "hello world";
    for (i, byte) in message.as_bytes().iter().enumerate() {
        mr_server[i] = *byte;
    }
    mr_server[11] = 0;

    unsafe { qp.post_send(&mut mr_server, .., 1) }.unwrap(); // wr_id = 1

    let mut completions = [ibv_wc::default(); 16];
    loop {
        let completed = cq.poll(&mut completions[..]).unwrap();
        for wr in completed {
            if wr.wr_id() == 1 {
                println!("Server send completed");
                return;
            }
        }
    }
}
