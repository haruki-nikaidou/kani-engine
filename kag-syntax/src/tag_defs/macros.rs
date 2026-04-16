/// Declarative macro that generates [`TagName`], [`KnownTag`], and all
/// associated impls from a single tag-definition table.
///
/// See the invocation in `mod.rs` for the full list of tags.
macro_rules! define_tags {
    // ── Entry point ────────────────────────────────────────────────────────
    //
    // We accept three kinds of tag definitions, separated by commas:
    //
    // 1. Unit tag (no attributes):
    //      Variant("kag_name") { doc = "..." }
    //
    // 2. Tag with attributes:
    //      Variant("kag_name") { doc = "...", attrs { ... } }
    //
    // 3. Tag with alias:
    //      Variant("kag_name", alias AliasVariant "alias_str") { doc = "...", attrs { ... } }
    //
    // 4. WaitForCompletion group (special):
    //      @wait_group { Wa("wa"), Wm("wm"), ... }
    //
    // Attribute annotations:
    //   required   name: str,       — error if missing, string type
    //   required   name: T,         — error if missing, parsed type T
    //   recommended name: str,      — warning if missing
    //   recommended_any_of [a, b] name: str,  — warning if none of group present
    //   optional   name: str,       — no diagnostic
    //   optional   name: T,         — no diagnostic, parsed type T

    (
        $(
            $( #[doc = $doc:expr] )*
            $Variant:ident ( $kag_name:literal $( , alias $AliasVariant:ident $alias_str:literal )? )
            { $( $attr_name:ident : $attr_annot:ident $( ( $($group_key:literal),+ ) )? < $attr_ty:tt > ),* $(,)? }
        ),+
        $(,)?
        ;
        // WaitForCompletion group
        @wait_group { $( $WaitVariant:ident ( $wait_name:literal ) ),+ $(,)? }
    ) => {
        // ── TagName enum ───────────────────────────────────────────────────

        /// The canonical name of every KAG tag known to the engine.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum TagName {
            $(
                $Variant,
                $( $AliasVariant, )?
            )+
            // Wait-group variants
            $( $WaitVariant, )+
        }

        impl TagName {
            /// Parse a raw KAG tag-name string into a `TagName`.
            pub fn from_name(s: &str) -> Option<Self> {
                Some(match s {
                    $(
                        $kag_name => Self::$Variant,
                        $( $alias_str => Self::$AliasVariant, )?
                    )+
                    $( $wait_name => Self::$WaitVariant, )+
                    _ => return None,
                })
            }

            /// Return the KAG tag-name string for this variant.
            pub fn as_str(self) -> &'static str {
                match self {
                    $(
                        Self::$Variant => $kag_name,
                        $( Self::$AliasVariant => $alias_str, )?
                    )+
                    $( Self::$WaitVariant => $wait_name, )+
                }
            }

            /// Return the canonical variant (aliases map to their primary).
            pub fn canonical(self) -> Self {
                match self {
                    $( $( Self::$AliasVariant => Self::$Variant, )? )+
                    other => other,
                }
            }

            /// Iterate over all variants (including aliases).
            pub fn all() -> impl Iterator<Item = TagName> {
                static ALL: &[TagName] = &[
                    $( TagName::$Variant, $( TagName::$AliasVariant, )? )+
                    $( TagName::$WaitVariant, )+
                ];
                ALL.iter().copied()
            }

            /// Return the known parameter names for this tag.
            pub fn param_names(self) -> &'static [&'static str] {
                self.canonical().param_names_canonical()
            }

            /// (Internal) param names for canonical variants only.
            fn param_names_canonical(self) -> &'static [&'static str] {
                match self {
                    $(
                        Self::$Variant => &[ $( stringify!($attr_name), )* ],
                    )+
                    $( Self::$WaitVariant => &["canskip", "buf"], )+
                    // Aliases are resolved via canonical() so this is unreachable,
                    // but the compiler needs exhaustiveness.
                    $( $( Self::$AliasVariant => unreachable!(), )? )+
                }
            }

            /// Return a one-line doc summary for this tag.
            pub fn doc_summary(self) -> &'static str {
                self.canonical().doc_summary_canonical()
            }

            /// (Internal) doc summary for canonical variants only.
            fn doc_summary_canonical(self) -> &'static str {
                match self {
                    $(
                        Self::$Variant => define_tags!(@first_doc $( $doc, )* "", ),
                    )+
                    $( Self::$WaitVariant => "Wait for async completion.", )+
                    $( $( Self::$AliasVariant => unreachable!(), )? )+
                }
            }
        }

        impl ::std::fmt::Display for TagName {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        // ── KnownTag enum ──────────────────────────────────────────────────

        /// A KAG tag with typed, validated attributes.
        #[derive(Debug, Clone, PartialEq)]
        pub enum KnownTag<'src> {
            $(
                $Variant {
                    $( $attr_name: Option<MaybeResolved<'src, define_tags!(@rust_ty $attr_ty)>>, )*
                },
            )+

            /// Async-completion wait tags.
            WaitForCompletion {
                which: TagName,
                canskip: Option<MaybeResolved<'src, bool>>,
                buf: Option<MaybeResolved<'src, u32>>,
            },

            /// A tag not recognised by the engine.
            Extension {
                name: ::std::borrow::Cow<'src, str>,
                params: Vec<$crate::ast::Param<'src>>,
            },
        }

        impl<'src> KnownTag<'src> {
            /// Parse and validate a raw [`Tag`] into a typed [`KnownTag`].
            pub fn from_tag(
                tag: &$crate::ast::Tag<'src>,
                diags: &mut Vec<$crate::error::SyntaxWarning>,
            ) -> Self {
                let name = tag.name.as_ref();
                let span = tag.span;

                let ps = |key: &str| tag.param(key).cloned().map(parse_str_attr);

                match name {
                    $(
                        $kag_name $( | $alias_str )? => {
                            // Emit diagnostics for required / recommended attrs
                            $( define_tags!(@diag tag, diags, $attr_annot, $attr_name $( ( $($group_key),+ ) )? ); )*

                            Self::$Variant {
                                $( $attr_name: define_tags!(@extract tag, name, span, diags, $attr_ty, $attr_name, ps), )*
                            }
                        }
                    )+

                    // Wait-group
                    $( $wait_name )|+ => {
                        let which = match name {
                            $( $wait_name => TagName::$WaitVariant, )+
                            _ => unreachable!(),
                        };
                        let canskip = tag.param("canskip").cloned()
                            .map(|pv| parse_typed_attr(pv, name, "canskip", span, diags));
                        let buf = tag.param("buf").cloned()
                            .map(|pv| parse_typed_attr(pv, name, "buf", span, diags));
                        Self::WaitForCompletion { which, canskip, buf }
                    }

                    _ => Self::Extension {
                        name: tag.name.clone(),
                        params: tag.params.clone(),
                    },
                }
            }

            /// Return the [`TagName`] for this variant, or `None` for `Extension`.
            pub fn tag_name(&self) -> Option<TagName> {
                Some(match self {
                    $( Self::$Variant { .. } => TagName::$Variant, )+
                    Self::WaitForCompletion { which, .. } => *which,
                    Self::Extension { .. } => return None,
                })
            }
        }
    };

    // ── Helper: pick first doc string ──────────────────────────────────────
    (@first_doc $first:expr, $($rest:expr,)* ) => { $first };
    (@first_doc ) => { "" };

    // ── Helper: map attr type token to Rust type ───────────────────────────
    (@rust_ty str) => { AttributeString<'src> };
    (@rust_ty bool) => { bool };
    (@rust_ty u32) => { u32 };
    (@rust_ty u64) => { u64 };
    (@rust_ty f32) => { f32 };

    // ── Helper: emit diagnostic based on attr kind ─────────────────────────
    // required
    (@diag $tag:expr, $diags:expr, required, $attr_name:ident) => {
        require_attr($tag, stringify!($attr_name), $diags);
    };
    // recommended
    (@diag $tag:expr, $diags:expr, recommended, $attr_name:ident) => {
        recommend_attr($tag, stringify!($attr_name), $diags);
    };
    // recommended_any_of — only emit once per group, keyed by the first key
    (@diag $tag:expr, $diags:expr, recommended_any_of, $attr_name:ident ( $($group_key:literal),+ )) => {
        // Only emit when this attr is the first in the group
        {
            static GROUP: &[&str] = &[ $($group_key),+ ];
            if stringify!($attr_name) == GROUP[0] {
                recommend_any_attr($tag, GROUP, $diags);
            }
        }
    };
    // optional — no diagnostic
    (@diag $tag:expr, $diags:expr, optional, $attr_name:ident) => {};
    (@diag $tag:expr, $diags:expr, optional, $attr_name:ident ( $($group_key:literal),+ )) => {};
    (@diag $tag:expr, $diags:expr, required, $attr_name:ident ( $($group_key:literal),+ )) => {
        require_attr($tag, stringify!($attr_name), $diags);
    };
    (@diag $tag:expr, $diags:expr, recommended, $attr_name:ident ( $($group_key:literal),+ )) => {
        recommend_attr($tag, stringify!($attr_name), $diags);
    };

    // ── Helper: extract attribute value ────────────────────────────────────
    // String type — use ps shorthand
    (@extract $tag:expr, $name:expr, $span:expr, $diags:expr, str, $attr_name:ident, $ps:expr) => {
        $ps(stringify!($attr_name))
    };
    // Typed — use parse_typed_attr
    (@extract $tag:expr, $name:expr, $span:expr, $diags:expr, $ty:tt, $attr_name:ident, $ps:expr) => {
        $tag.param(stringify!($attr_name)).cloned()
            .map(|pv| parse_typed_attr(pv, $name, stringify!($attr_name), $span, $diags))
    };
}
