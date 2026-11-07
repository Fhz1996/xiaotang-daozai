//! 岛仔的原生矢量表情与连续形变。
use crate::anim::{approach, lerp, smoothstep};
use crate::model::Skin;
use crate::paint::{Card, H};
use crate::theme::{self, W};
use eframe::egui::Color32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PetState {
    Awake,
    Curious,
    Waking,
    Happy,
    Satisfied,
    Music,
    Worried,
    Focused,
    Dozing,
    Sleeping,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn singing_keeps_each_expression_for_fifteen_seconds_after_the_transition() {
        let mut pet=Pet::default();
        pet.step(PetState::Music,0.0,0.0,[0.0;2],true);
        let pose=pet.music_pose();
        for frame in 1..960 {
            pet.step(PetState::Music,frame as f64/60.0,1.0/60.0,[0.0;2],true);
            assert_eq!(pet.music_pose(),pose,"一秒过渡和十五秒停留期间不能频繁换表情");
        }
        pet.step(PetState::Music,16.1,0.2,[0.0;2],true);
        assert_ne!(pet.music_pose(),pose);
    }
    #[test]
    fn singing_rotates_expressions_and_starts_with_the_microphone() {
        let mut pet=Pet::default();let mut poses=Vec::new();let mut eyes=Vec::new();
        for frame in 0..=5880 {
            pet.step(PetState::Music,frame as f64/60.0,1.0/60.0,[0.0;2],true);
            if [60,1020,1980,2940,3900,4860,5820].contains(&frame) {
                poses.push(pet.music_pose());
                eyes.push(pet.eyes.map(|eye|(eye.height,eye.curve)));
            }
        }
        assert_eq!(poses,vec![MusicPose::Microphone,MusicPose::Hum,MusicPose::SideSing,MusicPose::Squint,MusicPose::HighNote,MusicPose::Smile,MusicPose::Microphone]);
        assert_ne!(eyes[0],eyes[2]);assert_ne!(eyes[2],eyes[3]);assert_ne!(eyes[3],eyes[4]);
        let age=pet.age;
        pet.step(PetState::Music,100.0,20.0,[0.0;2],false);
        assert_eq!(pet.age,age,"展开面板时不应跳过演唱表情");
        pet.step(PetState::Awake,101.0,0.1,[0.0;2],true);
        pet.step(PetState::Music,102.0,0.1,[0.0;2],true);
        assert_eq!(pet.music_pose(),MusicPose::Microphone);
    }
    #[test]
    fn microphone_moves_gently_while_the_head_keeps_facing_the_mouth() {
        let mut heights=Vec::new();let mut angles=Vec::new();
        for frame in 0..600 {
            let (head,angle)=microphone_pose(frame as f32/60.0);
            let tip=[head[0]+7.0*angle.cos(),head[1]+7.0*angle.sin()];
            let distance=|p:[f32;2]|((p[0]+0.8).powi(2)+(p[1]-20.7).powi(2)).sqrt();
            assert!(distance(head)<distance(tip),"麦头应在嘴边，麦柄朝外");
            assert!(head[0]-2.6>1.4 && head[1]-2.6>17.0,"不能遮住嘴或眼睛");
            assert!(tip[0]+1.0<16.0 && tip[1]+1.0<28.0,"摆动不能超出常驻槽位");
            heights.push(head[1]);angles.push(angle);
        }
        let range=|values:Vec<f32>|values.iter().copied().fold(f32::NEG_INFINITY,f32::max)-values.iter().copied().fold(f32::INFINITY,f32::min);
        assert!((0.6..1.1).contains(&range(heights)),"麦头应有细微起伏");
        assert!((0.10..0.18).contains(&range(angles)),"麦柄应有轻微转动");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MusicPose { Microphone, Hum, SideSing, Squint, HighNote, Smile }

const MUSIC_PERIOD:f32=16.0;
fn music_pose_at(age:f32)->MusicPose {
    match (age/MUSIC_PERIOD) as usize%6 {0=>MusicPose::Microphone,1=>MusicPose::Hum,2=>MusicPose::SideSing,3=>MusicPose::Squint,4=>MusicPose::HighNote,_=>MusicPose::Smile}
}

fn microphone_pose(age:f32)->([f32;2],f32) {
    ([5.9+(age*0.95).sin()*0.2,20.8+(age*1.35).sin()*0.45],35.0_f32.to_radians()+(age*1.1).sin()*0.07)
}

fn microphone(c:&Card,cx:f32,y:f32,age:f32){
    let age=if crate::bar_layout::animations_enabled(){age}else{0.0};
    let (head,angle)=microphone_pose(age);
    let point=|s:f32,v:f32|(cx+head[0]+s*angle.cos()-v*angle.sin(),y+head[1]+s*angle.sin()+v*angle.cos());
    let capsule=|start:f32,end:f32,width:f32,color:Color32|{
        let a=point(start,0.0);let b=point(end,0.0);
        c.seg(a,b,width,color);c.circle(a.0,a.1,width/2.0,color);c.circle(b.0,b.1,width/2.0,color);
    };
    let line=|a:(f32,f32),b:(f32,f32),width:f32,color:Color32|c.seg(point(a.0,a.1),point(b.0,b.1),width,color);
    capsule(2.3,7.0,1.9,Color32::from_rgb(176,183,197));
    capsule(2.5,6.8,1.45,Color32::from_rgb(76,84,101));
    line((3.4,-0.53),(6.4,-0.53),0.28,Color32::from_rgb(218,225,237));
    capsule(2.05,2.65,2.6,Color32::from_rgb(126,137,157));
    let head=point(0.0,0.0);
    c.circle(head.0,head.1,2.6,Color32::from_rgb(222,228,238));
    c.circle(head.0,head.1,2.15,Color32::from_rgb(164,176,194));
    for s in [-1.0f32,0.0,1.0] {let v=(4.0-s*s).sqrt()*0.88;line((s,-v),(s,v),0.32,Color32::from_rgb(92,105,126));}
    for v in [-1.0f32,0.0,1.0] {let s=(4.0-v*v).sqrt()*0.88;line((-s,v),(s,v),0.3,Color32::from_rgb(226,232,243));}
}

/// 填充椭圆（多边形近似，供萝卜眼睛的瞳孔使用）。
fn ellipse(c: &Card, cx: f32, cy: f32, rx: f32, ry: f32, color: Color32) {
    let n = 24usize;
    let pts: Vec<(f32, f32)> = (0..n).map(|i| {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        (cx + rx * a.cos(), cy + ry * a.sin())
    }).collect();
    c.poly(&pts, color);
}

/// 画萝卜眼白/瞳孔的不规则椭圆：上宽下窄的蛋形（cos 项）+ 沿轮廓的三次起伏（sin 项），
/// 去掉几何椭圆的呆板感，贴近参考图的手绘轮廓。眼白与瞳孔都用它（瞳孔是缩小版）。
fn eye_white(c: &Card, cx: f32, cy: f32, rx: f32, ry: f32, color: Color32) {
    let n = 32usize;
    let pts: Vec<(f32, f32)> = (0..n).map(|i| {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        let co = a.cos();
        let si = a.sin();
        // 上宽下窄的蛋形（18%，顶部比底部宽约 36%）+ 沿轮廓的三次起伏（5%），两端仍圆润、无尖角。
        let k = 1.0 - 0.18 * co - 0.05 * (a * 3.0).sin();
        (cx + rx * si * k, cy + ry * co)
    }).collect();
    c.poly(&pts, color);
}

#[derive(Clone, Copy)]
struct Eye {
    height: f32,
    spread: f32,
    curve: f32,
    y: f32,
    tilt: f32,
}

impl Eye {
    fn open(height: f32) -> Self { Self { height, spread: 0.0, curve: 0.0, y: 0.0, tilt: 0.0 } }
    fn arc(spread: f32, curve: f32) -> Self { Self { spread, curve, ..Self::open(0.0) } }
    fn follow(&mut self, goal: Self, dt: f32) {
        self.height = approach(self.height, goal.height, 17.0, dt);
        self.spread = approach(self.spread, goal.spread, 17.0, dt);
        self.curve = approach(self.curve, goal.curve, 17.0, dt);
        self.y = approach(self.y, goal.y, 12.0, dt);
        self.tilt = approach(self.tilt, goal.tilt, 12.0, dt);
    }
}

pub(crate) struct Pet {
    pub state: PetState,
    pub reaction: Option<(PetState, f32)>,
    pub waking_until: f64,
    pub last_idle: f64,
    pub hovered: bool,
    pub worried: bool,
    age: f32,
    eyes: [Eye; 2],
    gaze: [f32; 2],
    y: f32,
}

impl Default for Pet {
    fn default() -> Self {
        Self { state: PetState::Awake, reaction: None, waking_until: 0.0, last_idle: 0.0,
            hovered: false, worried: false, age: 0.0, eyes: [Eye::open(4.4); 2], gaze: [0.0; 2], y: 0.0 }
    }
}

impl Pet {
    pub fn react(&mut self, state: PetState) { self.reaction = Some((state, 2.6)); self.age = 0.0; }
    /// 反馈只在实际可见时计时，避免用户还在面板里操作时就播完。
    pub fn advance(&mut self, dt: f32, visible: bool) {
        if visible {
            if let Some((_, remaining)) = &mut self.reaction {
                *remaining -= dt;
                if *remaining <= 0.0 { self.reaction = None; }
            }
        }
    }
    pub fn music_pose(&self)->MusicPose {
        music_pose_at(self.age)
    }

    pub fn step(&mut self, state: PetState, t: f64, dt: f32, gaze: [f32; 2], visible: bool) {
        if state != self.state { self.state = state; self.age = 0.0; }
        if visible { self.age += dt; }
        let age = self.age;
        let mut eyes = [Eye::open(4.4); 2];
        let mut y = (t as f32 * 1.7).sin() * 0.35;
        let mut look = gaze;
        match state {
            PetState::Awake => {},
            PetState::Curious => {
                eyes[0].height = 5.6; eyes[1].height = 6.3;
                eyes[0].y = 0.6; eyes[1].y = -0.6;
            },
            PetState::Waking => {
                eyes[0].height = lerp(0.0, 4.4, smoothstep(0.1, 0.65, age));
                eyes[1].height = lerp(0.0, 4.4, smoothstep(0.4, 1.1, age));
                y = 1.0 - smoothstep(0.0, 1.2, age); look = [0.0; 2];
            },
            PetState::Happy => {
                eyes = [Eye::arc(2.6, -1.8); 2];
                y = -(age * 7.5).sin().abs() * 2.0 * (-age * 0.65).exp();
            },
            PetState::Satisfied => {
                eyes = [Eye::arc(2.2, -1.2); 2];
                y = (age * 8.0).sin() * 1.2 * (-age * 0.8).exp();
            },
            PetState::Music => {
                let sway=(age*1.4).sin();
                look=[0.0,0.0];y=-0.5+sway*0.25;
                match self.music_pose() {
                    MusicPose::Microphone=>{eyes=[Eye::arc(2.5,0.7);2];},
                    MusicPose::SideSing=>{eyes=[Eye::open(1.6);2];eyes[0].y=-0.5;eyes[1].y=0.5;look=[0.4,0.0];y=sway*0.35;},
                    MusicPose::HighNote=>{eyes=[Eye::arc(2.3,-0.7);2];eyes[0].tilt=-0.3;eyes[1].tilt=0.3;y=-1.5+sway*0.2;},
                    MusicPose::Hum=>{eyes=[Eye::arc(2.5,0.7);2];y=sway*0.3;},
                    MusicPose::Squint=>{eyes=[Eye::arc(1.6,0.0);2];y=sway*0.45;},
                    MusicPose::Smile=>{eyes=[Eye::arc(2.3,-0.8);2];eyes[0].y=-0.5;eyes[1].y=0.5;look=[sway*0.5,0.0];},
                }
            },
            PetState::Worried => {
                eyes[0] = Eye { spread: -0.6, tilt: -0.7, y: 0.4, ..Eye::open(2.8) };
                eyes[1] = Eye { spread: 0.6, tilt: 0.7, y: 1.0, ..Eye::open(1.8) };
            },
            PetState::Focused => { eyes = [Eye::open(1.8); 2]; look = [gaze[0] * 0.2, 0.0]; y = 0.0; },
            PetState::Dozing => { eyes = [Eye::open(0.6); 2]; y = 1.0; look = [0.0; 2]; },
            PetState::Sleeping => { eyes = [Eye::arc(2.4, 1.0); 2]; y = 0.8 + (t as f32 * 1.2).sin() * 0.4; look = [0.0; 2]; },
        }
        if matches!(state, PetState::Awake | PetState::Curious | PetState::Focused | PetState::Worried) {
            let start = 2.0 + crate::anim::hash01((t / 5.6).floor() as u64) * 2.7;
            let blink = ((t % 5.6) as f32 - start) / 0.18;
            if (0.0..1.0).contains(&blink) {
                for eye in &mut eyes { eye.height *= 1.0 - (blink * std::f32::consts::PI).sin(); }
            }
        }
        let eye_dt=if state==PetState::Music {dt*0.2}else{dt};
        for (eye, goal) in self.eyes.iter_mut().zip(eyes) { eye.follow(goal, eye_dt); }
        for (axis, goal) in self.gaze.iter_mut().zip(look) { *axis = approach(*axis, goal, 10.0, dt); }
        self.y = approach(self.y, y, 14.0, dt);
    }

    pub fn draw(&self, c: &Card, cx: f32, t: f64, skin: Skin) {
        let ink = if skin == Skin::Radish { crate::skin_paint::RADISH_INK } else { theme::WHITE };
        if skin == Skin::Radish {
            self.draw_radish_eyes(c, cx, ink);
        } else {
            self.draw_line_eyes(c, cx, ink);
        }
        match self.state {
            PetState::Dozing | PetState::Sleeping => {
                let bob = ((t as f32 * 1.4).sin() + 1.0) * 0.7;
                c.text(cx+14.0, 13.0-bob, "z", 7.0, W::Bold, Color32::from_rgb(170,160,255), H::Left);
                if self.state == PetState::Sleeping { c.text(cx+19.0, 9.0-bob, "z", 5.5, W::Bold, Color32::from_rgb(170,160,255), H::Left); }
            },
            PetState::Happy => {
                if skin == Skin::Radish {
                    // 萝卜 IP 开心：睁圆眼（上面已按 wide 画）+ 张嘴露舌。
                    self.draw_radish_open_mouth(c, cx, ink);
                } else {
                    let k = smoothstep(0.0, 0.2, self.age) * (1.0-smoothstep(1.5, 2.5, self.age));
                    let c = c.faded(k);
                    for (dx, y, r) in [(-18.0, 9.0, 2.1), (18.0, 8.0, 2.8)] {
                        c.seg((cx+dx-r,y),(cx+dx+r,y),1.0,theme::GREEN);
                        c.seg((cx+dx,y-r),(cx+dx,y+r),1.0,theme::GREEN);
                    }
                }
            },
            PetState::Satisfied => crate::paint::droplet(c,cx+18.0,12.0,5.0,theme::CYAN),
            PetState::Music => {
                if skin == Skin::Radish {
                    // 萝卜 IP 唱歌：睁圆眼 + 圆形 O 嘴（与开心的大笑横嘴区分）+ 手持麦克风。
                    self.draw_radish_sing_mouth(c, cx, ink);
                    microphone(c, cx, self.y + self.gaze[1], self.age);
                } else {
                    let alpha=smoothstep(0.0,1.0,self.age%MUSIC_PERIOD);
                    if self.age>=MUSIC_PERIOD && alpha<1.0 {
                        self.draw_music(&c.faded(1.0-alpha),cx,music_pose_at(self.age-MUSIC_PERIOD),ink);
                    }
                    self.draw_music(&c.faded(alpha),cx,self.music_pose(),ink);
                }
            },
            PetState::Worried => {
                c.fill(cx+17.0,8.0,1.6,4.2,0.8,theme::YELLOW);
                c.circle(cx+17.8,14.5,0.9,theme::YELLOW);
            },
            _ => {
                // 萝卜皮肤在默认清醒状态画黑色波浪嘴；其余皮肤沿用原版（无嘴）。
                if skin == Skin::Radish {
                    self.draw_radish_mouth(c, cx, ink);
                }
            },
        }
    }

    /// 原版眼睛：一条会形变的描边线（胶囊豆豆眼）。
    fn draw_line_eyes(&self, c: &Card, cx: f32, color: Color32) {
        for (i, eye) in self.eyes.iter().enumerate() {
            let x = cx + if i == 0 { -5.6 } else { 5.6 } + self.gaze[0];
            let y = 14.5 + self.y + eye.y + self.gaze[1];
            self.stroke_eye(c, x, y, eye, color);
        }
    }

    /// 单只眼睛的描边：沿 Eye 参数画出睁眼竖线或眯眼弧线。
    fn stroke_eye(&self, c: &Card, x: f32, y: f32, eye: &Eye, color: Color32) {
        let points: Vec<_> = (0..=12).map(|n| {
            let q = n as f32 / 6.0 - 1.0;
            (x + q * eye.spread, y + q * (eye.height / 2.0 + eye.tilt) + (1.0 - q*q) * eye.curve)
        }).collect();
        let stroke=if self.state==PetState::Music {3.6-0.8*smoothstep(0.5,1.5,eye.spread.abs())}else{3.6};
        c.path(&points, stroke, color);
        // 圆端点保持胶囊眼睛的轮廓。
        for p in [points[0], points[12]] { c.circle(p.0, p.1, stroke/2.0, color); }
    }

    /// 萝卜 IP 眼睛：睁眼是白色眼白 + 深蓝瞳孔，且瞳孔「对眼」——左眼贴右、右眼贴左；眯眼/闭眼退化成深色弧线。
    fn draw_radish_eyes(&self, c: &Card, cx: f32, ink: Color32) {
        let pupil = crate::skin_paint::RADISH_EYE;
        // 开心：睁圆大眼、瞳孔居中；唱歌：微微睁眼（横向压扁、眼白更小）、瞳孔居中；清醒/担忧等保持对眼。
        // 所有表情眼白都用同一个不规则蛋形（eye_white），唱歌只是把半高缩小并横向压扁（宽 > 高）。
        let wide = matches!(self.state, PetState::Happy);
        let sing = matches!(self.state, PetState::Music);
        for (i, eye) in self.eyes.iter().enumerate() {
            let x = cx + if i == 0 { -4.7 } else { 4.7 } + self.gaze[0];
            let y = 14.5 + self.y + eye.y + self.gaze[1];
            if wide || sing || eye.height > 1.2 {
                let wh = if wide {
                    4.4 * 0.72            // 开心：睁圆大眼
                } else if sing {
                    2.5                   // 唱歌：微微睁眼（眼白半高更小）
                } else {
                    eye.height * 0.72
                };
                let ww = if sing {
                    wh * 1.15            // 唱歌：横向压扁（宽 > 高）
                } else {
                    wh * 0.73            // 开心/清醒：竖椭圆
                };
                eye_white(c, x, y, ww, wh, Color32::WHITE);
                // 清醒/担忧等对眼且下移；开心/唱歌瞳孔居中。
                let dx = if wide || sing { 0.0 } else if i == 0 { 0.30 * ww } else { -0.30 * ww };
                let dy = if wide || sing { 0.0 } else { wh * 0.10 };
                // 瞳孔也用不规则蛋形（eye_white 缩小版），与眼白同源，避免标准椭圆的呆板感。
                eye_white(c, x + dx, y + dy, ww * 0.62, wh * 0.68, pupil);
            } else {
                self.stroke_eye(c, x, y, eye, ink);
            }
        }
    }

    /// 萝卜 IP 嘴：从参考图逐像素提取的「实心粗条」形状（上下两条边界围成的波浪，两端球状膨大），
    /// 等比例缩放（x/y 同 scale），忠实还原参考图。上边界左→右 + 下边界右→左拼成闭合多边形填充，
    /// 两端各补一个圆球头。
    fn draw_radish_mouth(&self, c: &Card, cx: f32, ink: Color32) {
        let y = 23.5 + self.y;
        let sx = 1.12; // 整体拉宽一点点
        let top: [(f32, f32); 21] = [
            (-5.0, 0.665), (-4.495, 1.543), (-3.989, 1.543), (-3.511, 1.303), (-3.005, 0.479),
            (-2.5, 0.426), (-1.995, 0.638), (-1.489, 0.878), (-1.011, 1.037), (-0.505, 1.064),
            (0.0, 0.984), (0.505, 0.771), (1.011, 0.452), (1.489, 0.239), (1.995, 0.133),
            (2.5, 0.106), (3.005, 0.239), (3.511, 0.771), (3.989, 1.144), (4.495, 1.223), (5.0, 0.505),
        ];
        let bot: [(f32, f32); 21] = [
            (-5.0, 0.426), (-4.495, -0.505), (-3.989, -0.824), (-3.511, -1.09), (-3.005, -1.223),
            (-2.5, -1.223), (-1.995, -1.09), (-1.489, -0.878), (-1.011, -0.718), (-0.505, -0.665),
            (0.0, -0.771), (0.505, -0.957), (1.011, -1.17), (1.489, -1.356), (1.995, -1.463),
            (2.5, -1.516), (3.005, -1.489), (3.511, -1.33), (3.989, -1.09), (4.495, -0.638), (5.0, 0.213),
        ];
        let mut pts: Vec<(f32, f32)> = Vec::with_capacity(42);
        for (dx, dy) in top.iter() { pts.push((cx + dx * sx, y - dy)); }
        for (dx, dy) in bot.iter().rev() { pts.push((cx + dx * sx, y - dy)); }
        c.poly(&pts, ink);
        // 两端球头：参考图两端是「球状膨大」的圆头，直径约等于嘴巴最粗处。
        c.circle(cx - 4.5 * sx, y - 0.52, 1.05, ink);
        c.circle(cx + 4.5 * sx, y - 0.29, 1.05, ink);
    }

    /// 萝卜 IP 开心嘴：横向大笑（宽扁黑开口）+ 下方粉色舌头。
    fn draw_radish_open_mouth(&self, c: &Card, cx: f32, ink: Color32) {
        let y = 23.5 + self.y;
        let tongue = crate::skin_paint::RADISH_TONGUE;
        // 黑色口腔：接近圆形的开口（参考图宽高≈1:1）。
        ellipse(c, cx, y, 3.3, 2.7, ink);
        // 粉色舌头：更大更圆、靠下，几乎填满口腔，黑边只剩一圈窄环。
        ellipse(c, cx, y + 0.6, 2.5, 1.8, tongue);
    }

    /// 萝卜 IP 唱歌嘴：圆形 O 嘴（发「啊/哦」音），与开心的大笑横嘴区分。
    fn draw_radish_sing_mouth(&self, c: &Card, cx: f32, ink: Color32) {
        let y = 23.5 + self.y;
        ellipse(c, cx, y, 2.3, 2.3, ink);
    }

    fn draw_music(&self,c:&Card,cx:f32,pose:MusicPose,ink:Color32){
        let y=self.y+self.gaze[1];
        let pulse=(self.age*2.0).sin()*0.5+0.5;
        let mouth=|rx:f32,ry:f32|{
            let points:Vec<_>=(0..=24).map(|n|{let angle=n as f32/24.0*std::f32::consts::TAU;(cx-0.8+angle.cos()*rx,20.7+y+angle.sin()*ry)}).collect();
            c.path(&points,0.9,ink);
        };
        match pose {
            MusicPose::Microphone=>mouth(1.1,0.8+pulse*0.5),
            MusicPose::SideSing=>mouth(1.5,0.8+pulse*0.3),
            MusicPose::HighNote=>mouth(1.7,1.6+pulse*0.5),
            MusicPose::Hum=>c.path(&[(cx-2.7,20.7+y),(cx-1.8,21.4+y),(cx-0.8,21.0+y),(cx+0.2,21.4+y),(cx+1.1,20.7+y)],0.9,ink),
            MusicPose::Squint=>mouth(1.2,0.4+pulse*0.6),
            MusicPose::Smile=>c.path(&[(cx-2.5,20.6+y),(cx-1.6,21.6+y),(cx-0.5,21.9+y),(cx+0.6,21.6+y),(cx+1.5,20.6+y)],1.0,ink),
        }
        if matches!(pose,MusicPose::Microphone|MusicPose::SideSing|MusicPose::HighNote) {
            microphone(c,cx,y,self.age);
        } else {
            let y=8.5+(self.age*1.4).sin()*0.6;
            c.seg((cx+16.0,y-3.0),(cx+16.0,y+3.0),1.1,ink);
            c.seg((cx+16.0,y-3.0),(cx+19.0,y-2.0),1.1,ink);
            c.circle(cx+14.8,y+3.0,1.5,ink);
        }
    }
}
