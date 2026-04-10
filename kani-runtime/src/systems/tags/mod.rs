use bevy::prelude::MessageWriter;

use crate::events::{
    EvAudioTag, EvCharaTag, EvEffectTag, EvImageTag, EvMessageTag, EvTransitionTag, EvUnknownTag,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeTag {
    Bg,
    Image,
    Layopt,
    Free,
    Position,
    Bgm,
    Stopbgm,
    Se,
    PlaySe,
    Stopse,
    Vo,
    Voice,
    Fadebgm,
    Trans,
    Fadein,
    Fadeout,
    Movetrans,
    Quake,
    Shake,
    Flash,
    Msgwnd,
    Wndctrl,
    Resetfont,
    Font,
    Size,
    Bold,
    Italic,
    Ruby,
    Nowrap,
    Endnowrap,
    Chara,
    CharaHide,
    CharaFree,
    CharaMod,
}

pub struct TagWriters<'a, 'w> {
    pub ev_image: &'a mut MessageWriter<'w, EvImageTag>,
    pub ev_audio: &'a mut MessageWriter<'w, EvAudioTag>,
    pub ev_transition: &'a mut MessageWriter<'w, EvTransitionTag>,
    pub ev_effect: &'a mut MessageWriter<'w, EvEffectTag>,
    pub ev_message: &'a mut MessageWriter<'w, EvMessageTag>,
    pub ev_chara: &'a mut MessageWriter<'w, EvCharaTag>,
    pub ev_unknown: &'a mut MessageWriter<'w, EvUnknownTag>,
}

enum TagCategory {
    Image,
    Audio,
    Transition,
    Effect,
    Message,
    Chara,
}

impl RuntimeTag {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "bg" => Self::Bg,
            "image" => Self::Image,
            "layopt" => Self::Layopt,
            "free" => Self::Free,
            "position" => Self::Position,
            "bgm" => Self::Bgm,
            "stopbgm" => Self::Stopbgm,
            "se" => Self::Se,
            "playSe" => Self::PlaySe,
            "stopse" => Self::Stopse,
            "vo" => Self::Vo,
            "voice" => Self::Voice,
            "fadebgm" => Self::Fadebgm,
            "trans" => Self::Trans,
            "fadein" => Self::Fadein,
            "fadeout" => Self::Fadeout,
            "movetrans" => Self::Movetrans,
            "quake" => Self::Quake,
            "shake" => Self::Shake,
            "flash" => Self::Flash,
            "msgwnd" => Self::Msgwnd,
            "wndctrl" => Self::Wndctrl,
            "resetfont" => Self::Resetfont,
            "font" => Self::Font,
            "size" => Self::Size,
            "bold" => Self::Bold,
            "italic" => Self::Italic,
            "ruby" => Self::Ruby,
            "nowrap" => Self::Nowrap,
            "endnowrap" => Self::Endnowrap,
            "chara" => Self::Chara,
            "chara_hide" => Self::CharaHide,
            "chara_free" => Self::CharaFree,
            "chara_mod" => Self::CharaMod,
            _ => return None,
        })
    }

    fn category(self) -> TagCategory {
        match self {
            Self::Bg | Self::Image | Self::Layopt | Self::Free | Self::Position => {
                TagCategory::Image
            }
            Self::Bgm
            | Self::Stopbgm
            | Self::Se
            | Self::PlaySe
            | Self::Stopse
            | Self::Vo
            | Self::Voice
            | Self::Fadebgm => TagCategory::Audio,
            Self::Trans | Self::Fadein | Self::Fadeout | Self::Movetrans => TagCategory::Transition,
            Self::Quake | Self::Shake | Self::Flash => TagCategory::Effect,
            Self::Msgwnd
            | Self::Wndctrl
            | Self::Resetfont
            | Self::Font
            | Self::Size
            | Self::Bold
            | Self::Italic
            | Self::Ruby
            | Self::Nowrap
            | Self::Endnowrap => TagCategory::Message,
            Self::Chara | Self::CharaHide | Self::CharaFree | Self::CharaMod => TagCategory::Chara,
        }
    }
}

pub fn route_tag(name: String, params: Vec<(String, String)>, writers: &mut TagWriters<'_, '_>) {
    match RuntimeTag::parse(&name).map(RuntimeTag::category) {
        Some(TagCategory::Image) => {
            writers.ev_image.write(EvImageTag { name, params });
        }
        Some(TagCategory::Audio) => {
            writers.ev_audio.write(EvAudioTag { name, params });
        }
        Some(TagCategory::Transition) => {
            writers
                .ev_transition
                .write(EvTransitionTag { name, params });
        }
        Some(TagCategory::Effect) => {
            writers.ev_effect.write(EvEffectTag { name, params });
        }
        Some(TagCategory::Message) => {
            writers.ev_message.write(EvMessageTag { name, params });
        }
        Some(TagCategory::Chara) => {
            writers.ev_chara.write(EvCharaTag { name, params });
        }
        None => {
            writers.ev_unknown.write(EvUnknownTag { name, params });
        }
    }
}
