use std::sync::{Arc, Mutex};
use std::{thread, time::Duration};
use queues::*;
use tokio::{task, time};
use tts::Tts;
use brainrot::{twitch, youtube};
use futures_util::StreamExt;
use chrono;

#[derive(Debug, Clone)]
struct ChatMessage {
    author: String,
    text: String
}

#[tokio::main]
async fn main() {
    let yt_channel_id = "UCl8NSZ2GyhoQlAnZmW4cpBw";
    let twitch_id = "yoyoyonono";
    
    let mut tts_voice = Tts::new(tts::Backends::WinRt).unwrap();
    tts_voice.speak("hello, world", false).unwrap();
    thread::sleep(Duration::from_secs(2));

    let tts_queue = queue![];
    let tts_queue_arc = Arc::new(Mutex::new(tts_queue));
    let tts_queue_yt = Arc::clone(&tts_queue_arc);
    let tts_queue_twitch = Arc::clone(&tts_queue_arc);

    let program_start_time = chrono::Utc::now();

    let yt_handler = tokio::spawn(async move {
        let context = youtube::ChatContext::new_from_channel(yt_channel_id, youtube::ChannelSearchOptions::LatestLiveOrUpcoming).await.unwrap();
        let mut stream = youtube::stream(&context).await.unwrap();
        while let Some(Ok(c)) = stream.next().await {
            if let youtube::Action::AddChatItem { item: youtube::ChatItem::TextMessage {message_renderer_base, message}, .. } = c {
                if message_renderer_base.timestamp_usec < program_start_time {
                    continue;
                }
                let text = ChatMessage {
                    author: message_renderer_base.author_name.unwrap().simple_text,
                    text: message.unwrap().runs.into_iter().map(|c| c.to_chat_string()).collect::<String>()
                };
                let mut lock = tts_queue_yt.lock().unwrap();
                lock.add(text).unwrap();
                drop(lock);
            }
        }
    });

    let twitch_handler = tokio::spawn(async move {
        let mut client = brainrot::TwitchChat::new(twitch_id, twitch::Anonymous).await.unwrap();

        while let Some(message) = client.next().await.transpose().unwrap() {
            if let brainrot::TwitchChatEvent::Message {user, contents, ..} = message {
                let text = ChatMessage {
                    author: user.display_name,
                    text: contents.iter().map(|c| c.to_string()).collect::<String>()
                };
                let mut lock = tts_queue_twitch.lock().unwrap();
                lock.add(text).unwrap();
                drop(lock);
            }
        }
    });

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

    yt_handler.await.unwrap();
    twitch_handler.await.unwrap();

}

