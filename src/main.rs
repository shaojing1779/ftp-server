use std::{
    env,
    io::{Error, Read},
    fs::{self, File}
};
use tokio::{
    self,
    net::{TcpListener, TcpStream},
    io::{AsyncWriteExt, AsyncReadExt}
};
use rand::Rng;


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
fn ftp_user(&mut self) -> String {
    if self.cmd.1.len() > 2 {
        self.user_name = self.cmd.1[0..self.cmd.1.len() - 2].to_string();
    } else {
        self.user_name = "🐱".to_owned();
    }

    "331 User name okay, need password\n".to_owned()
}

// PASS
fn ftp_pass(&self) -> String {
    "230 Login successful\n".to_owned()
}

// PASV
async fn ftp_pasv(&mut self) -> String {

    if self.status == 0 {  
        let seed: u16 = rand::thread_rng().gen();
        self.t_port.0 = 0b10000000 + seed % 0b1000000;
        self.t_port.1 = seed % 0xff;
    
        let port = (0x100 * self.t_port.0) + self.t_port.1;
        let addr = String::from("0.0.0.0:").to_string() + &port.to_string();
        match TcpListener::bind(addr).await {
            Ok(v) => self.listener = v,
            Err(e)=> println!("Err: {}", e),
        }
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
async fn ftp_list(&mut self) -> String {

    let (mut s, _) = self.listener.accept().await.unwrap();

    let ls_paths = fs::read_dir(self.get_pwd().to_owned()).unwrap();
    for _path in ls_paths {
        match s.write((_path.unwrap().path().as_os_str().to_str().unwrap().to_owned() + "\n").as_bytes()).await {
            Ok(_) => (),
            Err(e) => println!("Err: {}", e),
        }
    }

    "200 Ok!\n".to_owned()
}

// SYST
fn ftp_syst(&self) -> String {
    "200 🐶🐶 \n".to_owned()
}

// RETR
async fn ftp_retr(&self) -> String {

    let (mut s, _) = self.listener.accept().await.unwrap();

    let mut f = File::open(self.get_pwd().to_owned() + "/" + &self.cmd.1).unwrap();
    let mut buf = vec![];
    f.read(&mut buf).expect("buffer overflow");
    match s.write(&buf).await {
        Ok(_) => (),
        Err(e) => println!("Err: {}", e),
    }

    "226 File:[".to_owned() + &self.get_pwd() + "/" + &self.cmd.1 + "] send OK.\n"
}

// STOR
fn ftp_stor(&self) -> String {

    "".to_owned()
}

}

async fn handle_client(mut stream: TcpStream) -> Result<(), Error>{
    let mut buf = [0; 512];
    
    let welcome = "200 Welcome to FTP service.\n";
    if let Err(e) = stream.write_all(welcome.as_bytes()).await {
        eprintln!("failed to write to socket; err = {:?}", e);
    }
    
    let mut state = State {
        user_name : "".to_owned(),
        mode: TransMod::NORMAL,
        message: "".to_owned(),
        sock_pasv: 0,
        sock_port: 0,
        trans_type: 0,
        listener: TcpListener::bind("0.0.0.0:9527").await.unwrap(),
        status: 0,
        t_port: (0, 0),
        cmd: (String::from(""), String::from("")),
    };

    loop {
        match stream.read(&mut buf).await {
            Ok(v) => println!("bytes_read: {}", v),
            Err(e) => println!("Err: {}", e),
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

        println!("[{}] state->cmd:{:?}", state.user_name, state.cmd);
        let mut w_buf = String::new();
        match &state.cmd.0 as &str {
            "USER" => w_buf = state.ftp_user(),
            "PASS" => w_buf = state.ftp_pass(),
            "PWD" => w_buf = state.ftp_pwd(),
            "PASV" => w_buf = state.ftp_pasv().await,
            "SYST" => w_buf = state.ftp_syst(),
            "QUIT" => return stream.write_all("221 Goodbye!\n".to_owned().as_bytes()).await,
            "RETR" => {
                match stream.write_all("150 Opening BINARY mode data connection.\n".as_bytes()).await {
                    Ok(_) => (),
                    Err(e) => println!("Err: {}", e),
                }
                w_buf = state.ftp_retr().await
            },
            "LIST" => {
                match stream.write_all("150 Here comes the directory listing.\n".as_bytes()).await {
                    Ok(_) => (),
                    Err(e) => println!("Err: {}", e),
                }
                w_buf = state.ftp_list().await;
            },
            "STOR" => w_buf = state.ftp_stor(),

            _=> w_buf = "500 Unknown command 🙅\n".to_owned(),
        }
        match stream.write_all(&w_buf.as_bytes()).await {
            Ok(_) => (),
            Err(e) => println!("Err: {}", e),
        }
    }

}


async fn server(port: &u32)  -> Result<(), Error> {

    let addr = String::from("0.0.0.0:").to_string() + &port.to_string();
    let listener = TcpListener::bind(addr).await?;
    let mut handles = vec![];
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                println!("new client: {:?}", &addr);
                handles.push(
                    tokio::spawn(async move{
                        match handle_client(socket).await {
                            Ok(_) => (),
                            Err(e) => println!("Err: {}", e),
                        }
                    })

                );
            },
            Err(e) => println!("couldn't get client: {:?}", e),
        }
    }
}
#[tokio::main]
async fn main() -> std::io::Result<()> {

    let args: Vec<String> = env::args().collect();
    let mut port: u32= 2022;
    if args.len() > 1 {
        port = args[1].parse().unwrap();
    }
    match server(&port).await {
        Ok(_) => (),
        Err(e) => println!("Err: {}", e),
    }

    Ok(())
}