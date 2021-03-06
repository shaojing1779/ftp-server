use std::io::{self, Error, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::{thread, default};
use std::time;
use std::env;
use std::collections;
use rand::Rng;
use std::fs::{self, File};
use std::collections::LinkedList;
use std::sync::Mutex;


// Request type
enum Cmdlist
{ 
  ABOR, CWD, DELE, LIST, MDTM, MKD, NLST, PASS, PASV,
  PORT, PWD, QUIT, RETR, RMD, RNFR, RNTO, SITE, SIZE,
  STOR, TYPE, CDUP, USER, NOOP, SYST,
}

// Request command
struct Command
{
    command: String,
    arg: String,
}

enum TransMod {
    NORMAL, SERVER, CLIENT,
}

// user connect info
struct State
{
    // Connection mode: 0-NORMAL, 1-SERVER, 2-CLIENT
    mode: TransMod,
    user_name: String,
    message: String,
    /* PASV MOD*/
    sock_pasv: u32,
    /* PORT MOD*/
    sock_port: u32,
    /* Transport type 0-bin 1-ascii */
    trans_type: i8,
    listener: TcpListener,
    /* 0-not connect 1-connected */
    status : u8,
    t_port: (u16, u16),
    cmd: (String, String),
}

// static CMD_LIST_VALUE: &'static [&str] = &[
//     "ABOR", "CWD", "DELE", "LIST", "MDTM", "MKD", "NLST", "PASS", "PASV",
//     "PORT", "PWD", "QUIT", "RETR", "RMD", "RNFR", "RNTO", "SITE", "SIZE",
//     "STOR", "TYPE", "CDUP", "USER", "NOOP", "SYST"];

impl State {
    
// USER
fn ftp_user(&self) -> String {
    "331 User name okay, need password\n".to_owned()
}

// PASS
fn ftp_pass(&self) -> String {
    "230 Login successful\n".to_owned()
}

// PASV
fn ftp_pasv(&mut self) -> String {

    if self.status == 0 {
        let mut rng = rand::thread_rng();
    
        let seed: u16 = rng.gen();
        self.t_port.0 = 0b10000000 + seed % 0b1000000;
        self.t_port.1 = seed % 0xff;
    
        let port = (0x100 * self.t_port.0) + self.t_port.1;
        let addr = String::from("0.0.0.0:").to_string() + &port.to_string();
        self.listener = TcpListener::bind(addr).unwrap();
        self.mode = TransMod::SERVER;
        self.status = 1;
    }
    
    "227 Entering Passive Mode 0,0,0,0,".to_owned() + &self.t_port.0.to_string() + "," + &self.t_port.1.to_string() + "\n"
}


fn get_pwd(&self) -> String {
    let res = env::current_dir();
    match res {
        // path.to_string_lossy().to_string()
        Ok(path) => path.into_os_string().into_string().unwrap(),
        Err(_) => "FAILED".to_string()
    }
}
// PWD
fn ftp_pwd(&self) -> String {
    self.get_pwd().to_owned() + "\n"
}

// LIST
fn ftp_list(&mut self) -> String {

    for stream in self.listener.incoming() {
        match stream {
            Ok(mut s) => {
                let ls_paths = fs::read_dir(self.get_pwd().to_owned()).unwrap();
                for _path in ls_paths {
                   s.write((_path.unwrap().path().as_os_str().to_str().unwrap().to_owned() + "\n").as_bytes());
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                break;
            }
            Err(e) => panic!("encountered IO error: {}", e),
        }
        break;
    }

    "200 Ok!\n".to_owned()
}

// SYST
fn ftp_syst(&self) -> String {
    "200 ???????? \n".to_owned()
}

// RETR
fn ftp_retr(&self) -> String {

    for stream in self.listener.incoming() {
        match stream {
            Ok(mut s) => {
                let mut f = File::open(self.get_pwd().to_owned() + "/" + &self.cmd.1).unwrap();
                let mut buf = vec![];
                f.read(&mut buf).expect("buffer overflow");
                s.write(&buf);
                break;
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                break;
            }
            Err(e) => panic!("encountered IO error: {}", e),
        }
    }

    "226 File:[".to_owned() + &self.get_pwd() + "/" + &self.cmd.1 + "] send OK.\n"
}

// STOR
fn ftp_stor(&self) -> String {

    "".to_owned()
}

// QUIT
fn ftp_quit(&self) -> String {
    
    "221 Goodbye!\n".to_owned()
}

}

fn handle_client(mut stream: TcpStream) -> Result<(), Error>{
    let mut buf = [0; 512];
    
    let welcome = "200 Welcome to FTP service.\n";
    stream.write(welcome.as_bytes())?;

    let mut state = State {
        user_name : "".to_owned(),
        mode: TransMod::NORMAL,
        message: "".to_owned(),
        sock_pasv: 0,
        sock_port: 0,
        trans_type: 0,
        listener: TcpListener::bind("0.0.0.0:9527").unwrap(),
        status: 0,
        t_port: (0, 0),
        cmd: (String::from(""), String::from("")),
    };

    loop {
        let bytes_read = stream.read(&mut buf)?;
        if bytes_read == 0 {
            return Ok(());
        }
        state.cmd = (String::from(""), String::from(""));
        let mut t_cmd:Vec<u8> = vec![];
        for (_, it) in buf.iter().enumerate() {
            if (*it == 0x20 || *it == 0x0A || *it == 0x0D) && state.cmd.0.is_empty() {
                state.cmd.0 = String::from_utf8(t_cmd.clone()).unwrap();  /* char[] -> string */
                t_cmd.clear();
                continue;
            } else if *it != 0x0 {
                t_cmd.push(*it);
            } else {
                continue;
            }
        }

        state.cmd.1 = String::from_utf8(t_cmd.clone()).unwrap();

        println!("state->cmd:{:?}", state.cmd);
        let mut w_buf = String::new();
        match &state.cmd.0 as &str {
            "USER" => w_buf = state.ftp_user(),
            "PASS" => w_buf = state.ftp_pass(),
            "PWD" => w_buf = state.ftp_pwd(),
            "PASV" => w_buf = state.ftp_pasv(),
            "SYST" => w_buf = state.ftp_syst(),
            "QUIT" => w_buf = state.ftp_quit(),
            "RETR" => {
                stream.write("150 Opening BINARY mode data connection.\n".as_bytes())?;
                w_buf = state.ftp_retr()
            },
            "LIST" => {
                stream.write("150 Here comes the directory listing.\n".as_bytes())?;
                w_buf = state.ftp_list();
            },
            "STOR" => w_buf = state.ftp_stor(),

            _=> w_buf = "500 Unknown command ????\n".to_owned(),
        }
        stream.write(&w_buf.as_bytes())?;
        thread::sleep(time::Duration::from_secs(1 as u64));
    }

}


fn server(port: &u32)  -> Result<(), Error> {

    let addr = String::from("0.0.0.0:").to_string() + &port.to_string();
    let listener = TcpListener::bind(addr).unwrap();
    let mut v_thread: Vec<thread::JoinHandle<()>> = Vec::new();

    for stream in listener.incoming() {
        let stream = stream.expect("failed!");
        let handle = thread::spawn(move || {
            handle_client(stream)
        .unwrap_or_else(|error| eprintln!("{:?}", error));
        });
        v_thread.push(handle);
    }

    for handle in v_thread {
        handle.join().unwrap();
    }
    Ok(())
}

fn main() -> std::io::Result<()> {

    let args: Vec<String> = env::args().collect();
    let mut port: u32= 2022;
    if args.len() > 1 {
        port = args[1].parse().unwrap();
    }
    let err = server(&port);

    Ok(())
}