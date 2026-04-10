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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

pub fn route_tag(
    name: String,
    params: Vec<(String, String)>,
    ev_image: &mut MessageWriter<EvImageTag>,
    ev_audio: &mut MessageWriter<EvAudioTag>,
    ev_transition: &mut MessageWriter<EvTransitionTag>,
    ev_effect: &mut MessageWriter<EvEffectTag>,
    ev_message: &mut MessageWriter<EvMessageTag>,
    ev_chara: &mut MessageWriter<EvCharaTag>,
    ev_unknown: &mut MessageWriter<EvUnknownTag>,
) {
    match RuntimeTag::parse(&name).map(RuntimeTag::category) {
        Some(TagCategory::Image) => {
            ev_image.write(EvImageTag { name, params });
        }
        Some(TagCategory::Audio) => {
            ev_audio.write(EvAudioTag { name, params });
        }
        Some(TagCategory::Transition) => {
            ev_transition.write(EvTransitionTag { name, params });
        }
        Some(TagCategory::Effect) => {
            ev_effect.write(EvEffectTag { name, params });
        }
        Some(TagCategory::Message) => {
            ev_message.write(EvMessageTag { name, params });
        }
        Some(TagCategory::Chara) => {
            ev_chara.write(EvCharaTag { name, params });
        }
        None => {
            ev_unknown.write(EvUnknownTag { name, params });
        }
    }
}
