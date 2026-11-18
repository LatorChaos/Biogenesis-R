// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

macro_rules! reference_colors {
    ($( $variant:ident = ($index:literal, $argb:literal), )*) => {
        /// Exact reference classifier; unknown ARGB values retain their separate stored color
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(i32)]
        pub enum ColorKind {
            NoColor = -1,
            $( $variant = $index, )*
        }

        /// Return a fixed source palette holder after the caller validates its named index
        pub fn source_palette_color(index: usize) -> Option<crate::JavaColor> {
            const VALUES: &[u32] = &[$( $argb ),*];
            const COUNT: usize = VALUES.len();
            static COLORS: std::sync::OnceLock<[crate::JavaColor; COUNT]> = std::sync::OnceLock::new();
            COLORS
                .get_or_init(|| [$( crate::JavaColor::from_argb($argb) ),*])
                .get(index)
                .cloned()
        }

        impl ColorKind {
            pub const fn argb(self) -> Option<u32> {
                match self {
                    $( Self::$variant => Some($argb), )*
                    Self::NoColor => None,
                }
            }

            /// Return the actual fixed source literal holder
            pub fn color_ref(self) -> Option<crate::JavaColor> {
                source_palette_color(self as usize)
            }

            pub fn from_argb(argb: u32) -> Self {
                match argb {
                    $( $argb => Self::$variant, )*
                    _ => Self::NoColor,
                }
            }
        }
    };
}

// Identities are pinned by the Java color_identities observation in core.jsonl
reference_colors! {
    Green = (0, 0xff00ff00),
    Forest = (1, 0xff008000),
    Spring = (2, 0xff00ff80),
    Summer = (3, 0xff80ff40),
    Lime = (4, 0xffb0ff00),
    Leaf = (5, 0xff5cb800),
    C4 = (6, 0xff60c060),
    Jade = (7, 0xff00a86b),
    Grass = (8, 0xff90b040),
    Bark = (9, 0xff608040),
    Purple = (10, 0xffa80054),
    Red = (11, 0xffff0000),
    Fire = (12, 0xffff6400),
    Orange = (13, 0xffffc800),
    Maroon = (14, 0xff800000),
    Pink = (15, 0xffffafaf),
    Cream = (16, 0xffd0c08c),
    Silver = (17, 0xffc0c0c0),
    Spike = (18, 0xffa48464),
    Lilac = (19, 0xffc080ff),
    Gray = (20, 0xff808080),
    Violet = (21, 0xff800080),
    Olive = (22, 0xffb0b000),
    Sky = (23, 0xff80c0ff),
    Blue = (24, 0xff0000ff),
    Ochre = (25, 0xffcc7722),
    Fallow = (26, 0xff967117),
    Spore = (27, 0xff0050a0),
    White = (28, 0xffffffff),
    Plague = (29, 0xffffc0ff),
    Coral = (30, 0xffff648a),
    Mint = (31, 0xffa0e0a0),
    Lavender = (32, 0xff8060b0),
    Magenta = (33, 0xffff00ff),
    Rose = (34, 0xffff0080),
    Cyan = (35, 0xff00ffff),
    Teal = (36, 0xff008080),
    Yellow = (37, 0xffffff00),
    Auburn = (38, 0xff803030),
    Indigo = (39, 0xff6f00ff),
    Blond = (40, 0xffffff80),
    Flower = (41, 0xff8080ff),
    DarkGray = (42, 0xff404040),
    Gold = (43, 0xffd4af37),
    Dark = (44, 0xff381c0e),
    Eye = (45, 0xff004040),
    Winter = (46, 0xff365400),
    OldBark = (47, 0xff502800),
    DarkJade = (48, 0xff005436),
    DarkGreen = (49, 0xff365436),
    DarkFire = (50, 0xffe89246),
    DarkLilac = (51, 0xff604060),
    DeepSky = (52, 0xff4060ff),
    DarkOlive = (53, 0xff585800),
    SpikePoint = (54, 0xffa48463),
    Fruit = (55, 0xffff80c0),
    Vision = (56, 0xffe89146),
    Ice = (57, 0xff406080),
    LightBlue = (58, 0xff000080),
    LightBrown = (59, 0xff807040),
    GreenBrown = (60, 0xff808440),
    Broken = (61, 0xff608060),
    DeadBark = (62, 0xff806040),
    Brown = (63, 0xff964b00),
}
