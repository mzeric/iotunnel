/*
    Listen localhost portA

*/
use std::thread;
use async_std::net::{Incoming, TcpListener, TcpStream};
use async_std::io::{BufReader, BufWriter};
use async_std::prelude::*;
use async_std::task;
use futures::future::FutureExt;
use futures::{pin_mut, select, future::select_all, future,io::{ReadHalf, WriteHalf, Read},};
use futures::stream::FusedStream;
use futures::stream::StreamExt;
use std::net::ToSocketAddrs;

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

/*
    read_xor_stream paired with write_xor_stream
*/
pub async fn read_xor_stream(reader: &mut BufReader<&TcpStream>) -> io::Result<Box<[u8]>> {
    let mut buffer: Vec<u8> = vec![0; 1024];
    match reader.read(&mut buffer).await {
        Ok(len) => {
            buffer.truncate(len);
            buffer.iter_mut().for_each(|x| *x ^= 0x55);
            Ok(buffer.into_boxed_slice())
        },
        Err(err) => Err(err),
    }
}
pub fn write_xor_stream(reader: &BufWriter<&TcpStream>, data: &mut Box<[u8]>) -> EcResult<()> {
    data.iter_mut().for_each(|x| *x ^= 0x55);
    Ok(())
}
pub async fn do_forward(mut stream: TcpStream, up_addr: &str) ->  EcResult<()> {
    println!("connecting: {} -> {:?}", stream.peer_addr()?, up_addr);

    let (reader, mut writer) = (&stream, &stream);
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    let mut up_stream = TcpStream::connect(up_addr).await?;

    let (up_reader, mut up_writer) = (&up_stream, &up_stream);
    let mut up_reader = BufReader::new(up_reader);
    let mut up_writer = BufWriter::new(up_writer);

    loop {

        select! {
            line = read_stream(&mut reader).fuse() => match line{

                Ok(data) => {

                    up_writer.write_all(&data).await?;
                    up_writer.flush().await?;
                    let a = String::from_utf8((&data).to_vec());
                    // println!("in ok: [{:?}]", a);
                    if data.len() == 0 {
                        break;
                    }
                },
                Err(e) => {
                    println!("in err: {:?}", e);
                    break;
                }
            },
            line = read_stream(&mut up_reader).fuse() => match line{
                Err(e) => {
                    println!("Connection was closed by remote peer");
                    break;
                },
                Ok(mut data) => {
                    write_xor_stream(&writer, &mut data);
                    writer.write_all(&data).await?;
                    writer.flush().await?;
                    let a = String::from_utf8((&data).to_vec());
                    // println!("remote data:[{:?}]", a);
                    if data.len() == 0 {
                        break;
                    }
                }
            },
        }    
    }
    Ok(())
}

async fn start_server() -> std::io::Result<()> {
    let server = TcpListener::bind("0.0.0.0:8080").await?;
    println!("listening: {}", server.local_addr()?);
    
    let up_addr = "0.0.0.0:80";
    let mut incoming: Incoming = server.incoming();

    while let Some(stream) = incoming.next().await {
        let mut stream : TcpStream = stream?;

        task::spawn(async{
            println!("incoming");
            if let Err(s) = do_forward(stream, "localhost:8081").await{
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
