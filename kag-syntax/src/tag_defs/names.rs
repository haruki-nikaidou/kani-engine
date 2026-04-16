use std::fmt;
use super::KnownTag;

/// The canonical name of every KAG tag known to the engine.
///
/// A [`TagName`] is obtained via [`TagName::from_name`]; strings that do not
/// match any known tag return `None` and the tag is forwarded to the host as a
/// generic `KagEvent::Tag`.
///
/// Aliases (`playSe` and `voice`) are separate variants so that
/// [`TagName::as_str`] round-trips the original string exactly.  Where the two
/// forms are semantically identical, [`KnownTag`] merges them into a single
/// variant (`Se` and `Vo` respectively).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TagName {
    // ── Interpreter: control flow ─────────────────────────────────────────
    If,
    Elsif,
    Else,
    Endif,
    Ignore,
    Endignore,

    // ── Interpreter: navigation ───────────────────────────────────────────
    Jump,
    Call,
    Return,

    // ── Interpreter: choice links ─────────────────────────────────────────
    Link,
    Endlink,
    Glink,

    // ── Interpreter: scripting / expressions ─────────────────────────────
    Eval,
    Emb,
    Trace,

    // ── Interpreter: display control ──────────────────────────────────────
    L,
    P,
    R,
    S,
    Cm,
    Er,
    Ch,
    Hch,

    // ── Interpreter: timed waits ──────────────────────────────────────────
    Wait,
    Wc,

    // ── Interpreter: async-completion waits ───────────────────────────────
    Wa,
    Wm,
    Wt,
    Wq,
    Wb,
    Wf,
    Wl,
    Ws,
    Wv,
    Wp,
    /// Cancel all in-progress asynchronous operations (`[ct]`).
    Ct,

    // ── Interpreter: input / event handlers ───────────────────────────────
    Timeout,
    Waitclick,
    Cclick,
    Ctimeout,
    Cwheel,
    Click,
    Wheel,

    // ── Interpreter: log control ──────────────────────────────────────────
    Nolog,
    Endnolog,

    // ── Interpreter: display-speed control ───────────────────────────────
    Nowait,
    Endnowait,
    Resetdelay,
    Delay,
    Configdelay,
    Resetwait,
    Autowc,

    // ── Interpreter: backlog ──────────────────────────────────────────────
    Pushlog,

    // ── Interpreter: player input / triggers ─────────────────────────────
    Input,
    Waittrig,

    // ── Interpreter: macro management ────────────────────────────────────
    Macro,
    Erasemacro,
    Endmacro,

    // ── Interpreter: variable management ─────────────────────────────────
    Clearvar,
    Clearsysvar,
    Clearstack,

    // ── Interpreter: misc ────────────────────────────────────────────────
    Clickskip,
    CharaPtext,

    // ── Runtime: image / layer ────────────────────────────────────────────
    Bg,
    Image,
    Layopt,
    Free,
    Position,

    // ── Runtime: audio ────────────────────────────────────────────────────
    Bgm,
    Stopbgm,
    Se,
    /// Alias of `Se` — original tag string `"playSe"`.
    PlaySe,
    Stopse,
    Vo,
    /// Alias of `Vo` — original tag string `"voice"`.
    Voice,
    Fadebgm,

    // ── Runtime: transition ───────────────────────────────────────────────
    Trans,
    Fadein,
    Fadeout,
    Movetrans,

    // ── Runtime: effect ───────────────────────────────────────────────────
    Quake,
    Shake,
    Flash,

    // ── Runtime: message window ───────────────────────────────────────────
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

    // ── Runtime: character sprites ────────────────────────────────────────
    Chara,
    CharaHide,
    CharaFree,
    CharaMod,
}

impl TagName {
    /// Parse a raw KAG tag-name string into a `TagName`.
    ///
    /// Returns `None` for any name not recognised by the engine.  The match is
    /// **case-sensitive** to mirror KAG's own parser behaviour (`playSe` ≠
    /// `playse`).
    pub fn from_name(s: &str) -> Option<Self> {
        Some(match s {
            "if" => Self::If,
            "elsif" => Self::Elsif,
            "else" => Self::Else,
            "endif" => Self::Endif,
            "ignore" => Self::Ignore,
            "endignore" => Self::Endignore,
            "jump" => Self::Jump,
            "call" => Self::Call,
            "return" => Self::Return,
            "link" => Self::Link,
            "endlink" => Self::Endlink,
            "glink" => Self::Glink,
            "eval" => Self::Eval,
            "emb" => Self::Emb,
            "trace" => Self::Trace,
            "l" => Self::L,
            "p" => Self::P,
            "r" => Self::R,
            "s" => Self::S,
            "cm" => Self::Cm,
            "er" => Self::Er,
            "ch" => Self::Ch,
            "hch" => Self::Hch,
            "wait" => Self::Wait,
            "wc" => Self::Wc,
            "wa" => Self::Wa,
            "wm" => Self::Wm,
            "wt" => Self::Wt,
            "wq" => Self::Wq,
            "wb" => Self::Wb,
            "wf" => Self::Wf,
            "wl" => Self::Wl,
            "ws" => Self::Ws,
            "wv" => Self::Wv,
            "wp" => Self::Wp,
            "ct" => Self::Ct,
            "timeout" => Self::Timeout,
            "waitclick" => Self::Waitclick,
            "cclick" => Self::Cclick,
            "ctimeout" => Self::Ctimeout,
            "cwheel" => Self::Cwheel,
            "click" => Self::Click,
            "wheel" => Self::Wheel,
            "nolog" => Self::Nolog,
            "endnolog" => Self::Endnolog,
            "nowait" => Self::Nowait,
            "endnowait" => Self::Endnowait,
            "resetdelay" => Self::Resetdelay,
            "delay" => Self::Delay,
            "configdelay" => Self::Configdelay,
            "resetwait" => Self::Resetwait,
            "autowc" => Self::Autowc,
            "pushlog" => Self::Pushlog,
            "input" => Self::Input,
            "waittrig" => Self::Waittrig,
            "macro" => Self::Macro,
            "erasemacro" => Self::Erasemacro,
            "endmacro" => Self::Endmacro,
            "clearvar" => Self::Clearvar,
            "clearsysvar" => Self::Clearsysvar,
            "clearstack" => Self::Clearstack,
            "clickskip" => Self::Clickskip,
            "chara_ptext" => Self::CharaPtext,
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

    /// Return the canonical KAG tag-name string for this variant.
    ///
    /// For alias variants ([`TagName::PlaySe`], [``TagName::Voice`]) this
    /// returns the alias string itself, so
    /// `TagName::from_name(name.as_str()) == Some(name)` holds for every
    /// variant.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::If => "if",
            Self::Elsif => "elsif",
            Self::Else => "else",
            Self::Endif => "endif",
            Self::Ignore => "ignore",
            Self::Endignore => "endignore",
            Self::Jump => "jump",
            Self::Call => "call",
            Self::Return => "return",
            Self::Link => "link",
            Self::Endlink => "endlink",
            Self::Glink => "glink",
            Self::Eval => "eval",
            Self::Emb => "emb",
            Self::Trace => "trace",
            Self::L => "l",
            Self::P => "p",
            Self::R => "r",
            Self::S => "s",
            Self::Cm => "cm",
            Self::Er => "er",
            Self::Ch => "ch",
            Self::Hch => "hch",
            Self::Wait => "wait",
            Self::Wc => "wc",
            Self::Wa => "wa",
            Self::Wm => "wm",
            Self::Wt => "wt",
            Self::Wq => "wq",
            Self::Wb => "wb",
            Self::Wf => "wf",
            Self::Wl => "wl",
            Self::Ws => "ws",
            Self::Wv => "wv",
            Self::Wp => "wp",
            Self::Ct => "ct",
            Self::Timeout => "timeout",
            Self::Waitclick => "waitclick",
            Self::Cclick => "cclick",
            Self::Ctimeout => "ctimeout",
            Self::Cwheel => "cwheel",
            Self::Click => "click",
            Self::Wheel => "wheel",
            Self::Nolog => "nolog",
            Self::Endnolog => "endnolog",
            Self::Nowait => "nowait",
            Self::Endnowait => "endnowait",
            Self::Resetdelay => "resetdelay",
            Self::Delay => "delay",
            Self::Configdelay => "configdelay",
            Self::Resetwait => "resetwait",
            Self::Autowc => "autowc",
            Self::Pushlog => "pushlog",
            Self::Input => "input",
            Self::Waittrig => "waittrig",
            Self::Macro => "macro",
            Self::Erasemacro => "erasemacro",
            Self::Endmacro => "endmacro",
            Self::Clearvar => "clearvar",
            Self::Clearsysvar => "clearsysvar",
            Self::Clearstack => "clearstack",
            Self::Clickskip => "clickskip",
            Self::CharaPtext => "chara_ptext",
            Self::Bg => "bg",
            Self::Image => "image",
            Self::Layopt => "layopt",
            Self::Free => "free",
            Self::Position => "position",
            Self::Bgm => "bgm",
            Self::Stopbgm => "stopbgm",
            Self::Se => "se",
            Self::PlaySe => "playSe",
            Self::Stopse => "stopse",
            Self::Vo => "vo",
            Self::Voice => "voice",
            Self::Fadebgm => "fadebgm",
            Self::Trans => "trans",
            Self::Fadein => "fadein",
            Self::Fadeout => "fadeout",
            Self::Movetrans => "movetrans",
            Self::Quake => "quake",
            Self::Shake => "shake",
            Self::Flash => "flash",
            Self::Msgwnd => "msgwnd",
            Self::Wndctrl => "wndctrl",
            Self::Resetfont => "resetfont",
            Self::Font => "font",
            Self::Size => "size",
            Self::Bold => "bold",
            Self::Italic => "italic",
            Self::Ruby => "ruby",
            Self::Nowrap => "nowrap",
            Self::Endnowrap => "endnowrap",
            Self::Chara => "chara",
            Self::CharaHide => "chara_hide",
            Self::CharaFree => "chara_free",
            Self::CharaMod => "chara_mod",
        }
    }
}

impl fmt::Display for TagName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}