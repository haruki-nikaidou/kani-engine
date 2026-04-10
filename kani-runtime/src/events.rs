use bevy::prelude::Message;
use kag_interpreter::{ChoiceOption, InterpreterSnapshot};

#[derive(Message)]
pub struct EvDisplayText {
    pub text: String,
    pub speaker: Option<String>,
    pub speed: Option<u64>,
    pub log: bool,
}

#[derive(Message)]
pub struct EvInsertLineBreak;

#[derive(Message)]
pub struct EvClearMessage;

#[derive(Message)]
pub struct EvClearCurrentMessage;

#[derive(Message)]
pub struct EvBeginChoices(pub Vec<ChoiceOption>);

#[derive(Message)]
pub struct EvInputRequested {
    pub name: String,
    pub prompt: String,
    pub title: String,
}

#[derive(Message)]
pub struct EvEmbedText(pub String);

#[derive(Message)]
pub struct EvPushBacklog {
    pub text: String,
    pub join: bool,
}

#[derive(Message)]
pub struct EvSnapshot(pub Box<InterpreterSnapshot>);

#[derive(Message)]
pub struct EvUnknownTag {
    pub name: String,
    pub params: Vec<(String, String)>,
}

#[derive(Message)]
pub struct EvImageTag {
    pub name: String,
    pub params: Vec<(String, String)>,
}

#[derive(Message)]
pub struct EvAudioTag {
    pub name: String,
    pub params: Vec<(String, String)>,
}

#[derive(Message)]
pub struct EvTransitionTag {
    pub name: String,
    pub params: Vec<(String, String)>,
}

#[derive(Message)]
pub struct EvEffectTag {
    pub name: String,
    pub params: Vec<(String, String)>,
}

#[derive(Message)]
pub struct EvMessageTag {
    pub name: String,
    pub params: Vec<(String, String)>,
}

#[derive(Message)]
pub struct EvCharaTag {
    pub name: String,
    pub params: Vec<(String, String)>,
}
