use std::sync::{Arc, Mutex};
use std::{thread, time::Duration};
use queues::*;
use tokio::{task, time};

use tts::Tts;
use youtube_chat::live_chat::LiveChatClientBuilder;

#[derive(Clone)]
struct ChatMessage {
    author: String,
    text: String
}

#[tokio::main]
async fn main() {
    let yt_channel_id = "UCz8K1occVvDTYDfFo7N5EZw";
    
    let mut tts_voice = Tts::new(tts::Backends::WinRt).unwrap();
    tts_voice.speak("hello, world", false).unwrap();
    thread::sleep(Duration::from_secs(2));

    let tts_queue = queue![];
    let tts_queue_am = Arc::new(Mutex::new(tts_queue));
    let queue_clone = Arc::clone(&tts_queue_am);

    let mut client = LiveChatClientBuilder::new()
        .channel_id(yt_channel_id.to_string())
        .on_chat(move |chat_item| {
            for item in chat_item.message {
                match item {
                    youtube_chat::item::MessageItem::Text(text) => {
                        let name = chat_item.author.name.to_owned().unwrap();
                        println!("{}: {}", name, text);
                        tts_queue_am.lock().unwrap().add(ChatMessage{author: name, text: text}).unwrap();
                    },
                    _ => {}
                } 
            }
        })
        .build();

    client.start().await.unwrap();

    let yt_chat_queue = task::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(100));
        loop {
            interval.tick().await;
            client.execute().await;
        }
    });

    thread::spawn(move || {
        loop {
            let mut lock = queue_clone.lock().unwrap();
            if lock.size() > 0 {
                let text = lock.remove().unwrap();
                drop(lock);
                tts_voice.speak(format!("{}, {}", &text.author, &text.text), false).unwrap();
            }
            thread::sleep(Duration::from_millis(100));
        }
    });

    yt_chat_queue.await.unwrap();

}

