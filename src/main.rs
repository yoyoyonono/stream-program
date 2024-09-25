use std::sync::{Arc, Mutex};
use std::{thread, time::Duration};
use queues::*;
use tokio::{task, time};

use chrono;
use tts::Tts;
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::ServerMessage;
use youtube_chat::live_chat::LiveChatClientBuilder;

use twitch_irc::{SecureTCPTransport, TwitchIRCClient};

#[derive(Debug, Clone)]
struct ChatMessage {
    author: String,
    text: String
}

#[tokio::main]
async fn main() {
    let yt_channel_id = "UCl8NSZ2GyhoQlAnZmW4cpBw";
    
    let mut tts_voice = Tts::new(tts::Backends::WinRt).unwrap();
    tts_voice.speak("hello, world", false).unwrap();
    thread::sleep(Duration::from_secs(2));

    let tts_queue = queue![];
    let tts_queue_arc = Arc::new(Mutex::new(tts_queue));
    let tts_queue_yt = Arc::clone(&tts_queue_arc);
    let tts_queue_twitch = Arc::clone(&tts_queue_arc);

    let mut yt_client = LiveChatClientBuilder::new()
        .channel_id(yt_channel_id.to_string())
        .on_chat(move |chat_item| {
            if chat_item.message.len() > 1 {
                return;
            }
            for item in chat_item.message {
                match item {
                    youtube_chat::item::MessageItem::Text(text) => {
                        let name = chat_item.author.name.to_owned().unwrap();
                        println!("{} | {}: {}", chat_item.timestamp.unwrap().format("%Y/%m/%d %H:%M"), name, text);
                        tts_queue_yt.lock().unwrap().add(ChatMessage{author: name, text: text}).unwrap();
                    },
                    _ => {}
                } 
            }
        })
        .build();


    let twitch_chat_config = twitch_irc::ClientConfig::default();
    let (mut twitch_incoming_messages, twitch_client) = TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(twitch_chat_config);

    let handle_twitch_chat = tokio::spawn(async move {
        while let Some(message) = twitch_incoming_messages.recv().await {
            match message {
                ServerMessage::Privmsg(privmsg) => {
                    println!("{}: {}", privmsg.sender.name, privmsg.message_text);
                    tts_queue_twitch.lock().unwrap().add(ChatMessage{author: privmsg.sender.name, text: privmsg.message_text}).unwrap();
                }
                _ => {}
            }
        }
    });

    twitch_client.join("yoyoyonono".to_owned()).unwrap();

    let is_youtube_live = yt_client.start().await;
    match &is_youtube_live {
        Err(e) => {
            tts_voice.speak("youtube chat not available", false).unwrap();
        }, 
        _ => {}
    }

    thread::spawn(move || {
        loop {
            let mut lock = tts_queue_arc.lock().unwrap();
            println!("{:?}", lock);
            if lock.size() > 0 {
                let text = lock.remove().unwrap();
                tts_voice.speak(format!("{}, {}", &text.author, &text.text), false).unwrap();
            }
            drop(lock);
            thread::sleep(Duration::from_millis(100));
        }
    });

    match is_youtube_live {
        Ok(_) => {
            let yt_chat_queue = task::spawn(async move {
                let mut interval = time::interval(Duration::from_millis(100));
                loop {
                    interval.tick().await;
                    yt_client.execute().await;
                }
            });
            yt_chat_queue.await.unwrap();
        },
        _ => {}
    }

    handle_twitch_chat.await.unwrap();

}

