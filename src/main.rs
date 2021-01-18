/*
    Listen localhost portA

*/
use std::thread;
use async_std::net::{Incoming, TcpListener, TcpStream};
use async_std::io::{BufRead, BufReader, BufWriter, Write};
use async_std::prelude::*;
// use std::io::ErrorKind;
use async_std::task;
use futures::future::FutureExt;
use futures::{pin_mut, select, future::select_all, future,io::{ReadHalf, WriteHalf, Read},};
use futures::stream::FusedStream;
use std::net::ToSocketAddrs;
use futures::stream::StreamExt;

pub type EcResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

use std::io;

pub async fn read_stream(mut reader: &mut BufReader<&TcpStream>) -> io::Result<Box<[u8]>> {
    let mut buffer: Vec<u8> = vec![0; 1024];
    match reader.read(&mut buffer).await {
        Ok(len) => {
            buffer.truncate(len);
            Ok(buffer.into_boxed_slice())
        },
        Err(err) => Err(err),
    }
}

pub async fn do_forward(mut stream: TcpStream, up_addr: &str) ->  EcResult<()> {
    println!("connecting: {}", stream.peer_addr()?);
    let (hreader, mut hwriter) = (&stream, &stream);

    let mut reader = BufReader::new(hreader);
    let mut writer = BufWriter::new(hwriter);
    
    let mut up_stream = TcpStream::connect("www.baidu.com:80").await?;
    let (up_reader, mut up_writer) = (&up_stream, &up_stream);
    let up_reader = BufReader::new(up_reader);
    let mut up_writer = BufWriter::new(up_writer);
    let mut up_lines = up_reader.lines().fuse();

    loop {
        let mut buffer: Vec<u8> = vec![0; 1024];

        select! {
            // line = reader.read(&mut buffer).fuse() => match line{
                line = read_stream(&mut reader).fuse() => match line{
                /*
                None => {
                    println!("Connection was closed by server");
                    break;
                },
                Some(line) => {
                    if let Err(e) = line{
                        println!("err {:?}", e);
                        break;
                    }
                    let r = line.unwrap();
                    println!("in data:[{:?}], {:?}", r, r.len());
                    // handle_server_line(line?),
                    {
                        up_writer.write_all((r+"\n").as_bytes()).await?;
                        up_writer.flush().await?;
                    }
                },
                */
                Ok(data) => {
                    // buffer.truncate(data);
                    // let buf = buffer.into_boxed_slice();
                    up_writer.write_all(&data).await?;
                    up_writer.flush().await?;
                    println!("in ok: [{:?}]", data);
                },
                Err(e) => {
                    println!("in err: {:?}", e);
                    break;
                }
            },
            line = up_lines.next() => match line{
                None => {
                    println!("Connection was closed by remote peer");
                    break;
                },
                Some(line) => {
                    if let Err(e) = line{
                        println!("remote err {:?}", e);
                        break;
                    }
                    println!("remote data:[{:?}]", line);
                    // handle_server_line(line?),
                    writer.write_all(line.unwrap().as_bytes());
                    writer.flush();
                },
            },
            // line = stdin_lines.next() => match line{
            //     None => break,
            //     Some(line) => handle_stdin_line(line?).await?,
            // },
        }    
    }
    Ok(())
}

async fn start_server() -> std::io::Result<()> {
    let server = TcpListener::bind("0.0.0.0:8081").await?;
    println!("listening: {}", server.local_addr()?);
    
    let up_addr = "0.0.0.0:80";
    let mut incoming: Incoming = server.incoming();

    while let Some(stream) = incoming.next().await {
        let stream : TcpStream = stream?;
        task::spawn(async{
            println!("incoming");
            if let Err(s) = do_forward(stream, "172.105.242.229:80").await{
                println!("connect forward failed: {:?}", s);
            }
            println!("incoming done");
        });
    }


    Ok(())

}

fn main() -> std::io::Result<()> {
    
    futures::executor::block_on(start_server());
 
    Ok(())
}
