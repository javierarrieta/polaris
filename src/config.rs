use std::old_io::{BufferedReader,IoError};
use std::old_io::net::ip::{SocketAddr};
use std::old_io::{File};

pub struct Config {
    pub local_addr: SocketAddr,
    pub contact_nodes: Vec<SocketAddr>,
    pub active_random_walk_length: u8,
    pub passive_random_walk_length: u8,
    pub active_view_size: usize,
    pub passive_view_size: usize,
    pub shuffle_period_seconds: u8,
    pub shuffle_active_view_count: u8,
    pub shuffle_passive_view_count: u8,
    pub shuffle_walk_length: u8,
}
impl Config {
    /// i really don't want to create a toml/yaml/whatever lib or pull in one (for now), so just use a 
    /// fixed format text file. current format is (each line, that is):
    /// - local_addr: ipAddrV4:port
    /// - contact_nodes: comma delimited list of {ipAddrV4:port} tuples
    /// - ARWL,PRWL: comma delimted {active|passive} random walk length
    /// - AV,PV sizes: {active|passive} view sizes
    /// - shuffle period : number of seconds between each shuffle round
    /// - shuffle AV,PV node counts: the number of {active|passive} node ids to send in a SHUFFLE message
    /// - shuffle walk length
    pub fn load_config(file_name: &str) -> Config {
        let path = Path::new(file_name);
        let mut reader = BufferedReader::new(File::open(&path));

        let line = reader.read_line().ok().unwrap();
        let local_addr: SocketAddr = line.as_slice().trim().parse().unwrap();

        let mut contact_nodes = Vec::new();
        let line = reader.read_line().ok().unwrap();
        let v: Vec<&str> = line.split_str(",").collect();
        for addr in v.iter() {
            contact_nodes.push(addr.as_slice().trim().parse().unwrap());
        }

        let (arwl, prwl) = Config::read_int_pair(&mut reader);
        let (active_size, passive_size) = Config::read_int_pair(&mut reader);
        let line = reader.read_line().ok().unwrap();
        let shuffle_period: u8 = line.as_slice().trim().parse().unwrap();
        let (shuffle_active_cnt, shuffle_passive_count) = Config::read_int_pair(&mut reader);
        let shuffle_walk_length: u8 = line.as_slice().trim().parse().unwrap();

        Config { local_addr: local_addr, contact_nodes: contact_nodes, active_random_walk_length: arwl, passive_random_walk_length: prwl,
                 active_view_size: active_size as usize, passive_view_size: passive_size as usize, shuffle_period_seconds: shuffle_period, 
                 shuffle_active_view_count: shuffle_active_cnt, shuffle_passive_view_count: shuffle_passive_count, shuffle_walk_length: shuffle_walk_length, }
    }

    fn read_int_pair(reader: &mut BufferedReader<Result<File, IoError>>) -> (u8, u8) {
        let line = reader.read_line().ok().unwrap();
        let v: Vec<&str> = line.trim().split_str(",").collect();
        let val0: u8 = v[0].parse().unwrap();
        let val1: u8 = v[1].parse().unwrap();
        (val0, val1)
    }
}

