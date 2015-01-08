use std::io::{IoError,IoErrorKind,IoResult};
use std::io::net::ip::{Ipv4Addr,SocketAddr};
use std::io::net::tcp::{TcpStream};

// TODO: need a much better system for identifying the messages (by type) than this simple hard-coded list, but wtf...
static HPV_MSG_ID_JOIN: u8 = 0;
static HPV_MSG_ID_FORWARD_JOIN: u8 = 1;
static HPV_MSG_ID_JOIN_ACK: u8 = 2;
static HPV_MSG_ID_DISCONNECT: u8 = 3;
static HPV_MSG_ID_NEIGHBOR_REQUEST: u8 = 4;
static HPV_MSG_ID_NEIGHBOR_RESPONSE: u8 = 5;
static HPV_MSG_ID_SHUFFLE: u8 = 6;
static HPV_MSG_ID_SHUFFLE_REPLY: u8 = 7;

#[derive(Copy,Show,PartialEq)]
pub enum Priority {
    High,
    Low
}

#[derive(Copy,Show)]
pub enum Result {
    Accept,
    Reject
}

#[derive(Show)]
pub enum HyParViewMessage {
    JoinMessage(Join,SocketAddr),
    ForwardJoinMessage(ForwardJoin,SocketAddr),
    JoinAckMessage(JoinAck,SocketAddr),
    DisconnectMessage(Disconnect,SocketAddr),
    NeighborRequestMessage(NeighborRequest,SocketAddr),
    NeighborResponseMessage(NeighborResponse,SocketAddr),
    ShuffleMessage(Shuffle,SocketAddr),
    ShuffleReplyMessage(ShuffleReply,SocketAddr),

    // control-type messages here
    JoinBegin,
    NextShuffleRound,
    PeerDisconnect(SocketAddr),
}

/// top-level function for serializing a HyParView message.
pub fn deserialize(reader: &mut TcpStream) -> IoResult<HyParViewMessage> {
    let header = match Header::deserialize(reader) {
        Ok(header) => header,
        Err(e) => return Err(e),
    };

    match header.msg_id {
        0 => Ok(HyParViewMessage::JoinMessage(Join::new(), header.sender)),
        1 => Ok(HyParViewMessage::ForwardJoinMessage(ForwardJoin::deserialize(reader).ok().expect("failed to deserailize the forward join"), header.sender)),
        2 => Ok(HyParViewMessage::JoinAckMessage(JoinAck::new(), header.sender)),
        3 => Ok(HyParViewMessage::DisconnectMessage(Disconnect::new(), header.sender)),
        4 => Ok(HyParViewMessage::NeighborRequestMessage(NeighborRequest::deserialize(reader).ok().expect("failed to deserailize the neighbor request"), header.sender)),
        5 => Ok(HyParViewMessage::NeighborResponseMessage(NeighborResponse::deserialize(reader).ok().expect("failed to deserailize the neighbor response"), header.sender)),
        6 => Ok(HyParViewMessage::ShuffleMessage(Shuffle::deserialize(reader).ok().expect("failed to deserailize the shuffle"), header.sender)),
        7 => Ok(HyParViewMessage::ShuffleReplyMessage(ShuffleReply::deserialize(reader).ok().expect("failed to deserailize the shuffle reply"), header.sender)),
        _ => Err(IoError{ kind: IoErrorKind::InvalidInput, desc: "unknown message id passed in".as_slice(), detail: None }),
    }
}

struct Header {
    sender: SocketAddr,
    msg_id: u8,
}
impl Header {
    fn new(sender: &SocketAddr, msg_id: u8) -> Header {
        Header { sender: *sender, msg_id: msg_id }
    }

    fn serailize(&self, writer: &mut Writer) -> IoResult<int> {
        let mut cnt = 1;
        match serialize_socket_addr(&self.sender, writer) {
            Ok(c) => cnt += c,
            Err(e) => return Err(e),
        }
        writer.write_u8(self.msg_id).ok();
        cnt += 1;
        Ok(cnt)
    }

    fn deserialize(reader: &mut Reader) -> IoResult<Header> {
        // TODO: this is a bit verbose, both match clauses
        let sender = match deserialize_socket_addr(reader) {
            Ok(socket) => socket,
            Err(e) => return Err(e),
        };

        let msg_id = match reader.read_u8() {
            Ok(id) => id,
            Err(e) => return Err(e),
        };
        Ok(Header { sender: sender, msg_id: msg_id })
    }
}

/// helper function to efficiently serialize a SocketAddr
fn serialize_socket_addr(sa: &SocketAddr, writer: &mut Writer) -> IoResult<int> {
    match sa.ip {
        Ipv4Addr(a, b, c, d) => {
            writer.write_u8(a).ok();
            writer.write_u8(b).ok();
            writer.write_u8(c).ok();
            writer.write_u8(d).ok();
            writer.write_be_u16(sa.port).ok();
        },
        _ => info!("problem writing socket addr to stream!"),
    }
    Ok(4 + 2)
}

/// helper function to efficiently deserialize a SocketAddr
fn deserialize_socket_addr(reader: &mut Reader) -> IoResult<SocketAddr> {
    let mut buf = Vec::with_capacity(4);
    let mut i = 0;
    while i < 4 {
        buf.push(reader.read_u8().ok().expect("couldn't read next byte for ip address"));
        i += 1;
    }
    let ip = Ipv4Addr (buf[0], buf[1], buf[2], buf[3]);
    let port = reader.read_be_u16().ok().expect("couldn't read port for socket address");

    let sa: SocketAddr = SocketAddr { ip: ip , port: port };
    Ok(sa)
}

#[derive(Copy,Show)]
//TODO: add a message uuid so we can register a callback (to make sure the join message gets a reponse, else resend the request)
pub struct Join;
impl Join {
    pub fn new() -> Join {
        Join
    }

    pub fn serialize(&self, writer: &mut Writer, sender: &SocketAddr) -> IoResult<int> {
        Header::new(sender, HPV_MSG_ID_JOIN).serailize(writer)
    }
}

#[derive(Copy,Show)]
pub struct ForwardJoin {
    pub originator: SocketAddr,
    pub arwl: u8,
    pub prwl: u8,
    pub ttl: u8
}
impl ForwardJoin {
    pub fn new(addr: &SocketAddr, arwl: u8, prwl: u8, ttl: u8) -> ForwardJoin {
        ForwardJoin { originator: *addr, arwl: arwl, prwl: prwl, ttl: ttl }
    }

    pub fn deserialize(reader: &mut Reader) -> IoResult<ForwardJoin> {
        match deserialize_socket_addr(reader) {
            Ok(socket) => {
                let arwl = reader.read_u8().ok().expect("could not read arwl from stream"); 
                let prwl = reader.read_u8().ok().expect("could not read prwl from stream"); 
                let ttl = reader.read_u8().ok().expect("could not read ttl from stream"); 
                Ok(ForwardJoin::new(&socket, arwl, prwl, ttl))
            },
            Err(e) => Err(e),
        }
    }

    pub fn serialize(&self, writer: &mut Writer, sender: &SocketAddr) -> IoResult<int> {
        let mut cnt = 0;
        let header = Header::new(sender, HPV_MSG_ID_FORWARD_JOIN);
        match header.serailize(writer) {
            Ok(c) => cnt += c,
            Err(e) => return Err(e),
        }

        match serialize_socket_addr(&self.originator, writer) {
            Ok(c) => cnt += c,
            Err(e) => return Err(e),
        }

        writer.write_u8(self.arwl).ok();
        cnt += 1;
        writer.write_u8(self.prwl).ok();
        cnt += 1;
        writer.write_u8(self.ttl).ok();
        cnt += 1;
        Ok(cnt)
    }
}

#[test]
fn test_join_serialization() {
    use std::io::{MemReader,BufferedWriter};
    use std::io::net::ip::{SocketAddr};

    let arwl = 6u8;
    let prwl = 3u8;
    let ttl = 4u8;

    let sock_addr: SocketAddr = ("127.0.0.1:9090").parse().expect("invalid socket addr");
    let fjoin_msg = ForwardJoin::new(&sock_addr, arwl, prwl, ttl);
    let vec = Vec::new();
    let mut writer = BufferedWriter::new(vec);
    let result = fjoin_msg.serialize(&mut writer, &sock_addr);
    
    let vec = writer.into_inner();
    let mut reader = MemReader::new(vec);
    let header = reader.read_u8().ok().expect("failed to read the header");
    let return_fjoin_msg = ForwardJoin::deserialize(&mut reader).ok().expect("failed to parse socket addr");
    assert!(return_fjoin_msg.originator.eq(&sock_addr));
    assert_eq!(return_fjoin_msg.arwl, arwl);
    assert_eq!(return_fjoin_msg.prwl, prwl);
    assert_eq!(return_fjoin_msg.ttl, ttl);
}

#[derive(Copy,Show)]
pub struct Disconnect;
impl Disconnect {
    pub fn new() -> Disconnect {
        Disconnect
    }

     pub fn serialize(&self, writer: &mut Writer, sender: &SocketAddr) -> IoResult<int> { 
        Header::new(sender, HPV_MSG_ID_DISCONNECT).serailize(writer)
    }
}

#[derive(Copy,Show)]
pub struct JoinAck;
impl JoinAck {
    pub fn new() -> JoinAck {
        JoinAck
    }

    pub fn serialize(&self, writer: &mut Writer, sender: &SocketAddr) -> IoResult<int> {
        Header::new(sender, HPV_MSG_ID_JOIN_ACK).serailize(writer)
    }
}

#[derive(Copy,Show)]
pub struct NeighborRequest {
    pub priority: Priority,
}
impl NeighborRequest {
    pub fn new(priority: Priority) -> NeighborRequest {
        NeighborRequest { priority: priority}
    }

    pub fn deserialize(reader: &mut Reader) -> IoResult<NeighborRequest> {
        let p = match reader.read_u8().ok().expect("could not read priority from stream") {
            0 => Priority::Low,
            1 => Priority::High,
            _ =>  {
                info!("received unknown priority level, defaulting to low priority");
                Priority::Low
            },
        };
        Ok(NeighborRequest::new(p))
    }

    pub fn serialize(&self, writer: &mut Writer, sender: &SocketAddr) -> IoResult<int> {
        let mut cnt = 0;
        let header = Header::new(sender, HPV_MSG_ID_NEIGHBOR_REQUEST);
        match header.serailize(writer) {
            Ok(c) => cnt += c,
            Err(e) => return Err(e),
        }

        let p: u8 = match self.priority {
            Priority::Low => 0,
            Priority::High => 1,
        };
        writer.write_u8(p).ok();
        cnt += 1;
        Ok(cnt)
    }
}

#[derive(Copy,Show)]
pub struct NeighborResponse {
    pub result: Result,
}
impl NeighborResponse {
    pub fn new(result: Result) -> NeighborResponse {
        NeighborResponse { result: result }
    }

    pub fn deserialize(reader: &mut Reader) -> IoResult<NeighborResponse> {
        let r = match reader.read_u8().ok().expect("could not read result from stream") {
            0 => Result::Accept,
            1 => Result::Reject,
            // if we can't undertand the result code, just default to reject
            _ => Result::Reject 
        };
        Ok(NeighborResponse::new(r))
    }

    pub fn serialize(&self, writer: &mut Writer, sender: &SocketAddr) -> IoResult<int> {
        let mut cnt = 0;
        let header = Header::new(sender, HPV_MSG_ID_NEIGHBOR_RESPONSE);
        match header.serailize(writer) {
            Ok(c) => cnt += c,
            Err(e) => return Err(e),
        }

        let p: u8 = match self.result {
            Result::Accept => 0,
            Result::Reject => 1,
        };
        writer.write_u8(p).ok();
        cnt += 1;
        Ok(cnt)
    }
}

#[derive(Show)]
pub struct Shuffle {
    pub originator: SocketAddr,
    pub nodes: Vec<SocketAddr>,
    pub ttl: u8,
}
impl Shuffle {
    pub fn new(originator: SocketAddr, nodes: Vec<SocketAddr>, ttl: u8) -> Shuffle {
        Shuffle { originator: originator, nodes: nodes, ttl: ttl }
    }

    pub fn deserialize(reader: &mut Reader) -> IoResult<Shuffle> {
        let originator = deserialize_socket_addr(reader).ok().expect("could not read socket addr from stream");
        let nodes = deserialize_socket_addrs(reader);
        let ttl = reader.read_u8().ok().expect("could not read ttl from stream"); 
        Ok(Shuffle::new(originator, nodes, ttl))
    }

    pub fn serialize(&self, writer: &mut Writer, sender: &SocketAddr) -> IoResult<int> {
        let mut cnt = 0;
        let header = Header::new(sender, HPV_MSG_ID_SHUFFLE);
        match header.serailize(writer) {
            Ok(c) => cnt += c,
            Err(e) => return Err(e),
        }

        match serialize_socket_addr(&self.originator, writer) {
            Ok(c) => cnt += c,
            Err(e) => return Err(e),
        }

        match serialize_socket_addrs(&self.nodes, writer) {
            Ok(c) => cnt += c,
            Err(e) => return Err(e),
        }

        writer.write_u8(self.ttl).ok();
        cnt += 1;
        Ok(cnt)
    }
}

fn deserialize_socket_addrs(reader: &mut Reader) -> Vec<SocketAddr> {
    let len = reader.read_u8().ok().expect("could not read vector size from stream") as uint;
    let mut nodes = Vec::with_capacity(len);

    for _ in range (0u, len) {
        match deserialize_socket_addr(reader) {
            Ok(socket) => nodes.push(socket),
            Err(e) => error!("failed to deserialize socket addr: {}", e),
        }
    }
    nodes
}

fn serialize_socket_addrs(nodes: &Vec<SocketAddr>, writer: &mut Writer) -> IoResult<int> {
    let mut cnt = 0;
    writer.write_u8(nodes.len() as u8).ok().expect("failed to write vec len");
    cnt += 1;
    
    for node in nodes.iter() {
        match serialize_socket_addr(node, writer) {
            Ok(c) => cnt += c,
            Err(e) => return Err(e),
        }
    }
    Ok(cnt)
}

#[derive(Show)]
pub struct ShuffleReply {
    /// return the vector of nodes the originator first sent out; that way the originator does not need to keep track of that.
    pub sent_nodes: Vec<SocketAddr>,
    pub nodes: Vec<SocketAddr>,
}
impl ShuffleReply {
    pub fn new(sent_nodes: Vec<SocketAddr>, nodes: Vec<SocketAddr>) -> ShuffleReply {
        ShuffleReply { sent_nodes: sent_nodes, nodes: nodes }
    }

    pub fn deserialize(reader: &mut Reader) -> IoResult<ShuffleReply> {
        let sent_nodes = deserialize_socket_addrs(reader);
        let nodes = deserialize_socket_addrs(reader);
        Ok(ShuffleReply::new(sent_nodes, nodes))
    }

    pub fn serialize(&self, writer: &mut Writer, sender: &SocketAddr) -> IoResult<int> {
        let mut cnt = 0;
        let header = Header::new(sender, HPV_MSG_ID_SHUFFLE_REPLY);
        match header.serailize(writer) {
            Ok(c) => cnt += c,
            Err(e) => return Err(e),
        }

        match serialize_socket_addrs(&self.sent_nodes, writer) {
            Ok(c) => cnt += c,
            Err(e) => return Err(e),
        }

        match serialize_socket_addrs(&self.nodes, writer) {
            Ok(c) => cnt += c,
            Err(e) => return Err(e),
        }
        Ok(cnt)
    }
}
