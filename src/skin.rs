//! 皮肤运行时配置：内置皮肤 + 从 `%APPDATA%\Daozai\skins\*.txt` 加载的自定义皮肤。
//!
//! 岛仔原本把皮肤（配色 + 宠物表情）写死在源码里、编译进 exe。为了让用户
//! 「丢一个文件就能加皮肤、不用重新编译」，这里把皮肤抽象成一份 `SkinDef`：
//! 内置的 6 款皮肤作为默认定义，另外在启动时扫描 `skins` 目录，把每个
//! `*.txt` 解析成一份新的自定义皮肤，追加到皮肤列表里。
//!
//! 自定义皮肤文件格式（`key=value`，与 settings.txt 一致，只认这几项）：
//! ```text
//! label=紫罗兰
//! description=紫色岛体，萝卜大眼睛
//! paper=180,60,220          # 岛体底色（缩略态，必填，r,g,b）
//! paper_open=255,255,255    # 展开态岛体底色（可选，缺省同缩略态）
//! ink=255,255,255           # 文字色（可选，默认深棕）
//! accent=255,180,40         # 强调色（可选，默认沿用系统蓝）
//! eye=radish                # 眼睛风格：radish 萝卜大眼睛 / default 原版豆豆眼
//! eye_ratio=0.73            # 眼白宽:高（竖椭圆 0.73，横向压扁可 >1）
//! pupil=42,69,176           # 瞳孔色（eye=radish 时生效）
//! ink_color=30,20,10        # 眼睛描边 / 嘴的颜色
//! tongue=226,110,128        # 舌头色（mouth=cute 时生效）
//! mouth=cute                # 嘴型：cute 萝卜嘴 / none 无嘴
//! ```
//! 文件名（不含 .txt）就是皮肤 key，重复则跳过。改完重启岛仔即生效。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use eframe::egui::Color32;
use crate::model::Skin;
use crate::skin_paint::Palette;

/// 眼睛风格。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EyeStyle {
    /// 原版豆豆眼：一条会随状态形变的描边线。
    Default,
    /// 萝卜大眼睛：白色眼白 + 彩色瞳孔的不规则蛋形（对眼）。
    Radish,
}

/// 嘴型风格。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouthStyle {
    /// 无嘴（原版默认）。
    None,
    /// 萝卜嘴：清醒波浪嘴 / 开心张嘴露舌 / 唱歌 O 嘴，随宠物状态自动切换。
    Cute,
}

/// 皮肤的完整定义：配色 + 宠物表情参数。
#[derive(Clone, Debug)]
pub struct SkinDef {
    pub key: String,
    pub label: String,
    pub description: String,
    /// 缩略态整体换色调色板（None = 不做整体换色，如经典黑、悬浮云）。
    pub palette: Option<Palette>,
    /// 展开态调色板（None = 与缩略态一致）。
    pub palette_open: Option<Palette>,
    /// 岛体底色（None = 经典黑 / 悬浮云动态色）。
    pub paper: Option<[u8; 3]>,
    /// 展开态岛体底色（None = 与缩略态一致）。
    pub paper_open: Option<[u8; 3]>,
    pub glass: bool,
    pub fluid: bool,
    // —— 宠物表情 ——
    pub eye_style: EyeStyle,
    /// 眼白宽:高（萝卜竖椭圆 0.73；横向压扁可 >1）。
    pub eye_ratio: f32,
    pub pupil: [u8; 3],
    /// 眼睛描边 / 嘴的颜色。
    pub ink: [u8; 3],
    pub tongue: [u8; 3],
    pub mouth: MouthStyle,
}

impl SkinDef {
    pub fn pupil_color(&self) -> Color32 {
        let [r, g, b] = self.pupil;
        Color32::from_rgb(r, g, b)
    }
    pub fn ink_color(&self) -> Color32 {
        let [r, g, b] = self.ink;
        Color32::from_rgb(r, g, b)
    }
    pub fn tongue_color(&self) -> Color32 {
        let [r, g, b] = self.tongue;
        Color32::from_rgb(r, g, b)
    }
    /// 岛体底色：缩略态 / 展开态。
    pub fn paper_color(&self, open: bool) -> Option<Color32> {
        let p = if open { self.paper_open.or(self.paper) } else { self.paper };
        p.map(|[r, g, b]| Color32::from_rgb(r, g, b))
    }
}

/// 内置 6 款皮肤的定义（与原有枚举一一对应，颜色与原 skin_paint 常量一致）。
fn builtin() -> Vec<SkinDef> {
    use crate::skin_paint::{CREAM, GLASS_DARK, GLASS_LIGHT, RADISH, RADISH_EYE, RADISH_INK, RADISH_LIGHT, RADISH_TONGUE};
    fn def(
        key: &str, label: &str, description: &str,
        palette: Option<Palette>, palette_open: Option<Palette>,
        paper: Option<[u8; 3]>, paper_open: Option<[u8; 3]>,
        glass: bool, fluid: bool,
        eye_style: EyeStyle, eye_ratio: f32, pupil: [u8; 3], ink: [u8; 3], tongue: [u8; 3], mouth: MouthStyle,
    ) -> SkinDef {
        SkinDef { key: key.into(), label: label.into(), description: description.into(), palette, palette_open, paper, paper_open, glass, fluid, eye_style, eye_ratio, pupil, ink, tongue, mouth }
    }
    let white = [255u8, 255, 255];
    let radish_pupil = [RADISH_EYE.r(), RADISH_EYE.g(), RADISH_EYE.b()];
    let radish_ink = [RADISH_INK.r(), RADISH_INK.g(), RADISH_INK.b()];
    let radish_tongue = [RADISH_TONGUE.r(), RADISH_TONGUE.g(), RADISH_TONGUE.b()];
    vec![
        def("classic", "经典", "纯黑岛体，与屏幕刘海融为一体，资源占用最低。",
            None, None, None, None, false, false,
            EyeStyle::Default, 1.0, [0, 0, 0], white, radish_tongue, MouthStyle::None),
        def("cream", "奶油", "暖白底色配深棕文字和橙色强调，柔和明亮。",
            Some(CREAM), None, Some(CREAM.paper), None, false, false,
            EyeStyle::Default, 1.0, [0, 0, 0], white, radish_tongue, MouthStyle::None),
        def("radish", "萝卜", "橙色萝卜岛体，配绿叶强调与蓝色眼睛，圆润可爱，对应萝卜 IP 配色。",
            Some(RADISH), Some(RADISH_LIGHT), Some(RADISH.paper), Some([255, 255, 255]), false, false,
            EyeStyle::Radish, 0.73, radish_pupil, radish_ink, radish_tongue, MouthStyle::Cute),
        def("glass_light", "毛玻璃·浅", "浅色磨砂玻璃，透出身后桌面的颜色，文字为深色；按背景明暗自动调整，保证清晰。",
            Some(GLASS_LIGHT), None, None, None, true, false,
            EyeStyle::Default, 1.0, [0, 0, 0], white, radish_tongue, MouthStyle::None),
        def("glass", "毛玻璃·深", "深色磨砂玻璃，透出身后桌面的颜色，文字为白色；按背景明暗自动调整，保证清晰。",
            Some(GLASS_DARK), None, None, None, true, false,
            EyeStyle::Default, 1.0, [0, 0, 0], white, radish_tongue, MouthStyle::None),
        def("fluid", "悬浮云", "半透明灰底随音乐封面换色，光晕跟随鼠标，播放进度带有渐隐光尾。皮肤作者：啵（@hanhb123）。",
            None, None, None, None, false, true,
            EyeStyle::Default, 1.0, [0, 0, 0], white, radish_tongue, MouthStyle::None),
    ]
}

fn custom_dir() -> PathBuf {
    crate::model::config_dir().join("skins")
}

fn parse_rgb(v: &str) -> Option<[u8; 3]> {
    let mut it = v.split(',').map(|s| s.trim().parse::<u8>().ok());
    Some([it.next()??, it.next()??, it.next()??])
}

/// 解析一个自定义皮肤文件。`key` 为文件名（不含扩展名）。
fn parse_custom(key: &str, text: &str) -> Option<SkinDef> {
    let mut map = HashMap::new();
    for line in text.trim_start_matches('\u{feff}').lines() {
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    let get = |k: &str| map.get(k).cloned();
    let paper = get("paper").and_then(|v| parse_rgb(&v))?;
    let paper_open = get("paper_open").and_then(|v| parse_rgb(&v));
    let ink = get("ink").and_then(|v| parse_rgb(&v)).unwrap_or([36, 24, 12]);
    let accent = get("accent").and_then(|v| parse_rgb(&v));
    let label = get("label").unwrap_or_else(|| key.to_string());
    let description = get("description").unwrap_or_default();
    let eye_style = match get("eye").as_deref() { Some("radish") => EyeStyle::Radish, _ => EyeStyle::Default };
    let eye_ratio = get("eye_ratio").and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.73);
    let pupil = get("pupil").and_then(|v| parse_rgb(&v)).unwrap_or([42, 69, 176]);
    let ink_color = get("ink_color").and_then(|v| parse_rgb(&v)).unwrap_or([30, 20, 10]);
    let tongue = get("tongue").and_then(|v| parse_rgb(&v)).unwrap_or([226, 110, 128]);
    // 用萝卜大眼睛时默认也带萝卜嘴；写了 mouth=none 则只留眼睛。
    let mouth = match get("mouth").as_deref() {
        Some("cute") => MouthStyle::Cute,
        Some("none") => MouthStyle::None,
        _ if eye_style == EyeStyle::Radish => MouthStyle::Cute,
        _ => MouthStyle::None,
    };
    let palette = Some(Palette { paper, ink, accent, dark_alpha: 1.0, lift: 1.0, darken: 0.82, chip: 1.0 });
    let palette_open = paper_open.map(|paper| Palette {
        paper, ink: [40, 26, 14], accent: accent.or(Some([46, 125, 50])),
        dark_alpha: 1.0, lift: 1.0, darken: 0.78, chip: 0.7,
    });
    Some(SkinDef {
        key: key.to_string(), label, description,
        palette, palette_open, paper: Some(paper), paper_open,
        glass: false, fluid: false,
        eye_style, eye_ratio, pupil, ink: ink_color, tongue, mouth,
    })
}

fn load_custom() -> Vec<SkinDef> {
    let dir = custom_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else { return Vec::new(); };
    let builtin_keys = builtin().into_iter().map(|d| d.key).collect::<Vec<_>>();
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") { continue; }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue; };
        if builtin_keys.iter().any(|k| k.as_str() == stem) { continue; }
        let Ok(text) = std::fs::read_to_string(&path) else { continue; };
        if let Some(def) = parse_custom(stem, &text) { out.push(def); }
    }
    out.sort_by(|a, b| a.key.cmp(&b.key));
    out
}

fn registry() -> &'static Vec<SkinDef> {
    static ALL: OnceLock<Vec<SkinDef>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut v = builtin();
        v.extend(load_custom());
        v
    })
}

/// 全部可用皮肤（内置在前，自定义在后，按 key 排序）。
pub fn all() -> &'static [SkinDef] {
    registry().as_slice()
}

/// 把皮肤 key 解析成 `Skin`：内置 key 映射到对应枚举，自定义 key 映射到
/// `Skin::Custom(key)`，未知 key 回退到 `Skin::Classic`。
pub fn resolve_key(key: &str) -> Skin {
    match key {
        "classic" => Skin::Classic,
        "fluid" => Skin::Fluid,
        "cream" => Skin::Cream,
        "radish" => Skin::Radish,
        "glass_light" => Skin::GlassLight,
        "glass" | "glass_dark" => Skin::GlassDark,
        other if all().iter().any(|d| d.key.as_str() == other) => Skin::Custom(other.to_string()),
        _ => Skin::Classic,
    }
}

/// 取某个皮肤对应的运行时定义（找不到回退到第一个内置皮肤「经典」）。
pub fn def(skin: &Skin) -> &'static SkinDef {
    let key = skin.key();
    all().iter().find(|d| d.key.as_str() == key).or_else(|| all().first()).expect("内置皮肤不可为空")
}
