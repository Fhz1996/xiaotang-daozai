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
    fn singing_first_expression_lasts_three_seconds_then_the_rest_rotate() {
        let mut pet=Pet::default();
        pet.step(PetState::Music,0.0,0.0,[0.0;2],true);
        assert_eq!(pet.music_pose(),MusicPose::Microphone);
        // 第一个表情只有 3 秒：到 2.8 秒的每一帧都还是拿话筒。
        for frame in 1..170 {
            pet.step(PetState::Music,frame as f64/60.0,1.0/60.0,[0.0;2],true);
            assert_eq!(pet.music_pose(),MusicPose::Microphone,"第一个表情应持续到 3 秒");
        }
        // 到 3.2 秒已经切成下一个表情，不再有停留段。
        for frame in 170..192 {
            pet.step(PetState::Music,frame as f64/60.0,1.0/60.0,[0.0;2],true);
        }
        let pose=pet.music_pose();
        assert_ne!(pose,MusicPose::Microphone,"3 秒后应接着变其余表情");
        // 其余表情各自只停 3 秒，随后立刻切到下一个（快节奏轮转，突出动态效果）。
        for frame in 192..350 {
            pet.step(PetState::Music,frame as f64/60.0,1.0/60.0,[0.0;2],true);
            assert_eq!(pet.music_pose(),pose,"其余表情应各自停留 3 秒");
        }
        for frame in 350..380 {
            pet.step(PetState::Music,frame as f64/60.0,1.0/60.0,[0.0;2],true);
        }
        assert_ne!(pet.music_pose(),pose,"每个表情 3 秒后应立刻轮换到下一个");
    }
    #[test]
    fn singing_rotates_expressions_and_starts_with_the_microphone() {
        let mut pet=Pet::default();let mut poses=Vec::new();let mut eyes=Vec::new();
        for frame in 0..=1200 {
            pet.step(PetState::Music,frame as f64/60.0,1.0/60.0,[0.0;2],true);
            // 新节奏：①0~3s 拿话筒 ②3~6 哼 ③6~9 侧身 ④9~12 眯 ⑤12~15 高音 ⑥15~18 微笑 ⑦18~ 拿话筒
            if [90,270,450,630,810,990,1170].contains(&frame) {
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
        assert!((0.9..1.4).contains(&range(heights)),"麦头应有细微起伏");
        assert!((0.16..0.24).contains(&range(angles)),"麦柄应有轻微转动");
    }
    #[test]
    fn singing_transition_is_soft_and_never_jumps_at_the_boundary() {
        assert_eq!(smoother01(0.0),0.0);
        assert_eq!(smoother01(1.0),1.0);
        assert!((smoother01(0.5)-0.5).abs()<1e-6,"缓动应关于中点对称");
        // 刚进唱歌时没有「上一个表情」，直接显示第一个。
        assert_eq!(music_blend(0.0),(0,None,1.0));
        // 边界前后：旧表情刚好收完、新表情从 0 开始，画面不会跳。
        let (end_slot,_,end_t)=music_blend(MUSIC_FIRST+MUSIC_PERIOD-0.001);
        let (start_slot,start_prev,start_t)=music_blend(MUSIC_FIRST+MUSIC_PERIOD+0.001);
        assert!(end_t>0.999);
        assert!(start_t<0.001);
        assert_eq!(start_prev,Some(end_slot));
        assert_eq!(start_slot,2);
        // 0.8 秒后过渡完成；眼睛和嘴巴共用同一个进度，永远同步。
        let (_,_,done)=music_blend(MUSIC_FIRST+MUSIC_FADE+0.001);
        assert_eq!(done,1.0);
    }
    #[test]
    fn singing_mouth_morph_is_a_single_continuous_shape() {
        let a=mouth_shape(MusicPose::Microphone,3.0);
        let b=mouth_shape(MusicPose::Hum,3.0);
        assert_eq!(b.cap_r,[0.0,0.0],"小圆嘴没有球头");
        let mid=lerp_mouth(a,b,0.5);
        assert!((mid.cap_r[0]-0.525).abs()<1e-5,"球头半径连续收拢到一半");
        // f32 的 a+(b-a)*t 在端点会有末位误差（还可能带上 -0.0），用容差比较形状而不是精确相等。
        let near=|u:[(f32,f32);42],v:[(f32,f32);42]| {
            u.iter().zip(v.iter()).all(|((ax,ay),(bx,by))|(ax-bx).abs()<1e-4&&(ay-by).abs()<1e-4)
        };
        assert!(near(lerp_mouth(a,b,0.0).edge,a.edge),"起点应回到波浪嘴");
        assert!(near(lerp_mouth(a,b,1.0).edge,b.edge),"终点应变成小圆嘴");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MusicPose { Microphone, Hum, SideSing, Squint, HighNote, Smile }

const MUSIC_PERIOD:f32=3.0;                        // 其余 5 个表情各 3 秒（快节奏轮转，突出动态效果）
const MUSIC_FIRST:f32=3.0;                         // 第一个表情（拿话筒）3 秒：眼睛渐变完就换表情
const MUSIC_CYCLE:f32=MUSIC_FIRST+5.0*MUSIC_PERIOD; // 3+15=18 秒一轮
const MUSIC_FADE:f32=0.8;                          // 表情切换过渡时长：拉长到 0.8 秒，形变从容不「一闪而过」
const MUSIC_POSES:[MusicPose;6]=[MusicPose::Microphone,MusicPose::Hum,MusicPose::SideSing,MusicPose::Squint,MusicPose::HighNote,MusicPose::Smile];

/// 比 smoothstep 更柔：两端的一阶、二阶导数都是 0，起步和收尾都不会有「突然动起来」的顿感。
fn smoother01(x:f32)->f32 {
    let t=x.clamp(0.0,1.0);
    t*t*t*(t*(t*6.0-15.0)+10.0)
}

/// 当前帧的唱歌表情混合：返回 (当前表情序号, 上一个表情序号, 混合进度 0..1)。
/// 眼睛与嘴巴共用这一个进度，二者永远同步形变，不会出现「眼睛已经换完、嘴还在淡入」。
fn music_blend(age:f32)->(usize,Option<usize>,f32) {
    let (slot,elapsed)=music_slot(age);
    // 刚进入唱歌时没有「上一个表情」，直接显示第一个。
    if age<MUSIC_FIRST { return (slot,None,1.0); }
    let prev=if slot==0 {5}else{slot-1};
    (slot,Some(prev),smoother01(elapsed/MUSIC_FADE))
}
/// 返回 (当前是第几个表情 0..5, 该表情已持续的秒数)。
fn music_slot(age:f32)->(usize,f32) {
    let t=age%MUSIC_CYCLE;
    if t<MUSIC_FIRST { return (0,t); }
    let k=(t-MUSIC_FIRST)/MUSIC_PERIOD;
    let idx=1+(k as usize).min(4);
    (idx,(t-MUSIC_FIRST)-k.floor()*MUSIC_PERIOD)
}
fn music_pose_at(age:f32)->MusicPose {
    MUSIC_POSES[music_slot(age).0]
}

/// 某个唱歌表情的「脸部目标参数」（眼睛 / 视线 / 身体 y）。抽成纯函数，切换时才能同时算出
/// 前后两个表情、在两者之间连续插值，而不是一帧内硬切。
fn music_face(pose:MusicPose, age:f32, sway:f32)->([Eye;2],[f32;2],f32) {
    let mut look=[0.0;2];
    let mut y=-0.5+sway*0.25;
    let eyes=match pose {
        MusicPose::Microphone=>{
            // 拿话筒：眼睛从缩略态全睁(4.4)在本表情的 3 秒内渐变到微微睁(2.5)。
            let prog=((age%MUSIC_CYCLE)/MUSIC_FIRST).clamp(0.0,1.0);
            [Eye::open(4.4+(2.5-4.4)*prog);2]
        },
        MusicPose::SideSing=>{
            let mut e=[Eye::open(2.0);2];e[0].y=-0.5;e[1].y=0.5;
            look=[0.4,0.0];y=sway*0.35;e
        },
        MusicPose::HighNote=>{
            let mut e=[Eye::arc(2.3,-0.7);2];e[0].tilt=-0.3;e[1].tilt=0.3;
            y=-1.5+sway*0.2;e
        },
        MusicPose::Hum=>{y=sway*0.3;[Eye::arc(2.5,0.7);2]},
        MusicPose::Squint=>{y=sway*0.45;[Eye::arc(1.6,0.0);2]},
        MusicPose::Smile=>{
            let mut e=[Eye::arc(2.3,-0.8);2];e[0].y=-0.5;e[1].y=0.5;
            look=[sway*0.5,0.0];e
        },
    };
    (eyes,look,y)
}

/// 两颗眼睛逐字段插值：睁眼蛋形 ↔ 闭眼弧线的弯向/张开度都能连续变化，不出现形状跳变。
fn lerp_eye(a:Eye,b:Eye,t:f32)->Eye {
    Eye {
        height: lerp(a.height,b.height,t),
        spread: lerp(a.spread,b.spread,t),
        curve:  lerp(a.curve, b.curve, t),
        y:      lerp(a.y,     b.y,     t),
        tilt:   lerp(a.tilt,  b.tilt,  t),
    }
}

fn microphone_pose(age:f32)->([f32;2],f32) {
    ([5.9+(age*0.95).sin()*0.4,20.8+(age*1.35).sin()*0.6],35.0_f32.to_radians()+(age*1.1).sin()*0.10)
}

/// 萝卜唱歌话筒：麦头贴到萝卜嘴边（萝卜嘴在 y≈23.5，比原版嘴 20.7 低），柄更陡、斜向下。
fn microphone_pose_radish(age:f32)->([f32;2],f32) {
    ([6.0+(age*0.95).sin()*0.4,22.6+(age*1.35).sin()*0.6],46.0_f32.to_radians()+(age*1.1).sin()*0.10)
}

fn microphone(c:&Card,cx:f32,y:f32,age:f32){
    let age=if crate::bar_layout::animations_enabled(){age}else{0.0};
    let (head,angle)=microphone_pose(age);
    draw_microphone(c,cx,y,head,angle);
}

fn microphone_radish(c:&Card,cx:f32,y:f32,age:f32){
    let age=if crate::bar_layout::animations_enabled(){age}else{0.0};
    let (head,angle)=microphone_pose_radish(age);
    draw_microphone(c,cx,y,head,angle);
}

fn draw_microphone(c:&Card,cx:f32,y:f32,head:[f32;2],angle:f32){
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
    let head_pt=point(0.0,0.0);
    c.circle(head_pt.0,head_pt.1,2.6,Color32::from_rgb(222,228,238));
    c.circle(head_pt.0,head_pt.1,2.15,Color32::from_rgb(164,176,194));
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

/// 不透明颜色插值：唱歌时眼睛由「白色蛋形」渐变成「墨色闭眼弧线」，
/// 用颜色过渡而不是半透明叠加，避免中间帧发灰、像蒙了一层脏影。
fn mix_color(a:Color32,b:Color32,t:f32)->Color32 {
    let t=t.clamp(0.0,1.0);
    let ch=|x:u8,y:u8|(x as f32+(y as f32-x as f32)*t).round() as u8;
    Color32::from_rgb(ch(a.r(),b.r()),ch(a.g(),b.g()),ch(a.b(),b.b()))
}

/// 唱歌态的一只眼睛：白色蛋形 ↔ 墨色闭眼弧线之间做「形状渐变」。
/// k=1 完全睁（定稿白眼+蓝瞳），k=0 完全闭（定稿黑弧线），中间用 20 段凸四边形
/// 拼出一条逐渐变弯、变细、变深的带子——不是把椭圆等比压扁（那样会留下一根难看的细蓝线）。
fn singing_eye(c:&Card,x:f32,y:f32,rw:f32,rh:f32,k:f32,curve:f32,
               ink:Color32,pupil:Color32,side:f32,gaze:[f32;2]) {
    const N:usize=20;
    let arc_half=1.35f32;                 // 闭眼线宽 2.7 的一半（与定稿一致）
    let col=mix_color(ink,Color32::WHITE,k);
    let xw=lerp(rw*0.9,rw,k);            // 闭眼弧线半宽 rw*0.9，睁眼眼白半宽 rw
    let band=|q:f32|->(f32,f32) {
        let shell=(1.0-q*q).max(0.0).sqrt();
        let bend=curve*(1.0-q*q)*(1.0-k);
        (bend,lerp(arc_half,rh*shell,k))
    };
    for j in 0..N {
        // 相邻段轻微重叠，避免拼接处出现一条条竖缝。
        let q0=(-1.0+2.0*j as f32/N as f32-0.012).max(-1.0);
        let q1=(-1.0+2.0*(j+1) as f32/N as f32+0.012).min(1.0);
        let (b0,h0)=band(q0);let (b1,h1)=band(q1);
        c.poly(&[(x+q0*xw,y+b0-h0),(x+q1*xw,y+b1-h1),
                 (x+q1*xw,y+b1+h1),(x+q0*xw,y+b0+h0)],col);
    }
    if k<0.9 {
        // 两端补圆头（只在接近闭眼时逐渐显露），与定稿 c.path 的圆头描边一致。
        let r=arc_half*(1.0-k);
        c.circle(x-xw,y,r,col);
        c.circle(x+xw,y,r,col);
    }
    if k>0.02 {
        // 瞳孔随闭眼同步缩小、颜色融入眼白，最后自然消失，不会留下细蓝条。
        let ball=mix_color(col,pupil,k);
        let pw=rw*0.62*k;let ph=rh*0.68*k;
        let gx=gaze[0]*0.22*k;let gy=gaze[1]*0.20*k;
        let base=side*0.30*rw*k;
        let max_dx=(xw-pw).max(0.0);
        let max_dy=(rh-ph).max(0.0);
        let dx=(base+gx).clamp(-max_dx,max_dx);
        let dy=(rh*0.10*k+gy).clamp(-max_dy,max_dy);
        eye_white(c,x+dx,y+dy,pw,ph,ball);
    }
}

/// 波浪嘴定稿形状：从参考图逐像素提取的上下两条边界（21 点），两端另有球状圆头。
/// 三种唱歌嘴统一用 `MouthShape` 表达，切换时逐点插值 = 一张嘴连续变形，
/// 不再把两张嘴半透明叠在一起（那样会出现「双嘴 / 灰影」的脏中间帧）。
const WAVE_SX:f32=1.12; // 整体拉宽一点点
const WAVE_TOP:[(f32,f32);21]=[
    (-5.0,0.665),(-4.495,1.543),(-3.989,1.543),(-3.511,1.303),(-3.005,0.479),
    (-2.5,0.426),(-1.995,0.638),(-1.489,0.878),(-1.011,1.037),(-0.505,1.064),
    (0.0,0.984),(0.505,0.771),(1.011,0.452),(1.489,0.239),(1.995,0.133),
    (2.5,0.106),(3.005,0.239),(3.511,0.771),(3.989,1.144),(4.495,1.223),(5.0,0.505),
];
const WAVE_BOT:[(f32,f32);21]=[
    (-5.0,0.426),(-4.495,-0.505),(-3.989,-0.824),(-3.511,-1.09),(-3.005,-1.223),
    (-2.5,-1.223),(-1.995,-1.09),(-1.489,-0.878),(-1.011,-0.718),(-0.505,-0.665),
    (0.0,-0.771),(0.505,-0.957),(1.011,-1.17),(1.489,-1.356),(1.995,-1.463),
    (2.5,-1.516),(3.005,-1.489),(3.511,-1.33),(3.989,-1.09),(4.495,-0.638),(5.0,0.213),
];

#[derive(Clone,Copy)]
struct MouthShape {
    /// 前 21 点=上边界(左→右)，后 21 点=下边界(左→右)；坐标是相对嘴中心的偏移。
    edge:[(f32,f32);42],
    cap:[(f32,f32);2],   // 两端球头圆心
    cap_r:[f32;2],       // 两端球头半径（O 嘴/小嘴为 0，过渡时收缩消失）
}

/// 某个唱歌表情在当前时刻的嘴形（O 嘴/小嘴的开合脉冲在这里算；波浪嘴是定稿静态形状）。
fn mouth_shape(pose:MusicPose,age:f32)->MouthShape {
    let mut s=MouthShape{edge:[(0.0,0.0);42],cap:[(0.0,0.0);2],cap_r:[0.0;2]};
    match pose {
        MusicPose::Microphone=>{
            for i in 0..21 {
                let (tx,ty)=WAVE_TOP[i];let (bx,by)=WAVE_BOT[i];
                s.edge[i]=(tx*WAVE_SX,-ty);
                s.edge[21+i]=(bx*WAVE_SX,-by);
            }
            s.cap=[(-4.5*WAVE_SX,-0.52),(4.5*WAVE_SX,-0.29)];
            s.cap_r=[1.05,1.05];
        },
        _=>{
            let open=(age*3.6).sin()*0.5+0.5;
            let (rx,ry)=if pose==MusicPose::SideSing {
                (2.1+open*0.5,1.6+open*1.3)   // 侧身唱：唱歌 O 嘴
            } else {
                (1.1+open*0.35,0.7+open*0.55) // 闭眼四态：简单小嘴
            };
            for i in 0..21 {
                let q=i as f32/20.0*2.0-1.0;
                let h=ry*(1.0-q*q).max(0.0).sqrt();
                s.edge[i]=(q*rx,-h);
                s.edge[21+i]=(q*rx,h);
            }
            s.cap=[(-rx,0.0),(rx,0.0)];
        },
    }
    s
}

/// 两张嘴逐点插值：波浪的每条边、两端的球头半径都连续收放，中间始终只有「一张实心嘴」。
fn lerp_mouth(a:MouthShape,b:MouthShape,t:f32)->MouthShape {
    let mut s=a;
    for i in 0..42 {
        s.edge[i]=(lerp(a.edge[i].0,b.edge[i].0,t),lerp(a.edge[i].1,b.edge[i].1,t));
    }
    for i in 0..2 {
        s.cap[i]=(lerp(a.cap[i].0,b.cap[i].0,t),lerp(a.cap[i].1,b.cap[i].1,t));
        s.cap_r[i]=lerp(a.cap_r[i],b.cap_r[i],t);
    }
    s
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
    /// 眨眼闭合程度：0=睁眼，1=闭眼（由 step 每帧按随机节律驱动，与状态无关）。
    blink: f32,
}

impl Default for Pet {
    fn default() -> Self {
        Self { state: PetState::Awake, reaction: None, waking_until: 0.0, last_idle: 0.0,
            hovered: false, worried: false, age: 0.0, eyes: [Eye::open(4.4); 2], gaze: [0.0; 2], y: 0.0, blink: 0.0 }
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
                // 切换后的 MUSIC_FADE 秒内，把「上一个表情」连续过渡到「当前表情」：
                // 眼睛逐字段插值（睁眼蛋形 ↔ 闭眼弧线）、视线与身体同步插值，避免硬切闪动。
                let (slot,prev,blend)=music_blend(age);
                let (mut cur_eyes,look_goal,y_goal)=music_face(MUSIC_POSES[slot],age,sway);
                if let Some(prev)=prev.filter(|_| blend<1.0) {
                    let (prev_eyes,prev_look,prev_y)=music_face(MUSIC_POSES[prev],age,sway);
                    for i in 0..2 { cur_eyes[i]=lerp_eye(prev_eyes[i],cur_eyes[i],blend); }
                    look=[lerp(prev_look[0],look_goal[0],blend),lerp(prev_look[1],look_goal[1],blend)];
                    y=lerp(prev_y,y_goal,blend);
                } else {
                    look=look_goal;y=y_goal;
                }
                eyes=cur_eyes;
            },
            PetState::Worried => {
                eyes[0] = Eye { spread: -0.6, tilt: -0.7, y: 0.4, ..Eye::open(2.8) };
                eyes[1] = Eye { spread: 0.6, tilt: 0.7, y: 1.0, ..Eye::open(1.8) };
            },
            PetState::Focused => { eyes = [Eye::open(1.8); 2]; look = [gaze[0] * 0.2, 0.0]; y = 0.0; },
            PetState::Dozing => { eyes = [Eye::open(0.6); 2]; y = 1.0; look = [0.0; 2]; },
            PetState::Sleeping => { eyes = [Eye::arc(2.4, 1.0); 2]; y = 0.8 + (t as f32 * 1.2).sin() * 0.4; look = [0.0; 2]; },
        }
        // 眨眼单独存成闭合程度（0..1），不再直接压 eye.height：这样萝卜大眼睛能
        // 只往中间压缩高度、保持宽度（真·闭眼），而不是把整个眼睛等比缩小。
        self.blink = 0.0;
        if matches!(state, PetState::Awake | PetState::Curious | PetState::Focused | PetState::Worried | PetState::Music) {
            let start = 2.0 + crate::anim::hash01((t / 5.6).floor() as u64) * 2.7;
            let blink = ((t % 5.6) as f32 - start) / 0.18;
            if (0.0..1.0).contains(&blink) {
                self.blink = (blink * std::f32::consts::PI).sin();
            }
        }
        // 眼睛跟随统一用正常速度：拿话筒的「缩略态→微微睁」柔和感已由 3 秒渐变本身提供，
        // 不再叠加 0.2 倍慢速跟随（滞后会把 3 秒拖成近 4 秒，达不到 3 秒到位）。
        for (eye, goal) in self.eyes.iter_mut().zip(eyes) { eye.follow(goal, dt); }
        for (axis, goal) in self.gaze.iter_mut().zip(look) { *axis = approach(*axis, goal, 10.0, dt); }
        self.y = approach(self.y, y, 14.0, dt);
    }

    pub fn draw(&self, c: &Card, cx: f32, t: f64, skin: &Skin) {
        let def = crate::skin::def(skin);
        let ink = def.ink_color();
        if def.eye_style == crate::skin::EyeStyle::Radish {
            self.draw_cute_eyes(c, cx, ink, def.pupil_color(), def.eye_ratio);
        } else {
            self.draw_line_eyes(c, cx, ink);
        }
        let cute_mouth = def.mouth == crate::skin::MouthStyle::Cute;
        match self.state {
            PetState::Dozing | PetState::Sleeping => {
                let bob = ((t as f32 * 1.4).sin() + 1.0) * 0.7;
                c.text(cx+14.0, 13.0-bob, "z", 7.0, W::Bold, Color32::from_rgb(170,160,255), H::Left);
                if self.state == PetState::Sleeping { c.text(cx+19.0, 9.0-bob, "z", 5.5, W::Bold, Color32::from_rgb(170,160,255), H::Left); }
            },
            PetState::Happy => {
                if cute_mouth {
                    // 萝卜式开心：睁圆眼（上面已按 wide 画）+ 张嘴露舌。
                    self.draw_open_mouth(c, cx, ink, def.tongue_color());
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
                if cute_mouth {
                    // 萝卜式唱歌嘴巴：拿话筒=波浪嘴 / 侧身唱=唱歌 O 嘴 / 闭眼四态=简单小嘴。
                    // 切换时只画「一张嘴」——前后两种嘴形逐点插值成中间形状（波浪慢慢收拢成 O 嘴），
                    // 不再把两个嘴半透明叠在一起（那样会出现双嘴、灰影，看着又脏又跳）。
                    // 眼睛由 step 里的参数插值连续变形，话筒始终完整绘制（不淡化，避免忽隐忽现）。
                    let (slot,prev,blend)=music_blend(self.age);
                    self.draw_music_mouth(c,cx,prev.map(|p|MUSIC_POSES[p]),MUSIC_POSES[slot],blend,ink);
                    microphone_radish(c, cx, self.y + self.gaze[1], self.age);
                } else {
                    // 原版皮肤沿用叠加淡入，但进度与萝卜皮肤共用同一个 music_blend，保持节奏一致。
                    let (slot,prev,alpha)=music_blend(self.age);
                    if let Some(prev)=prev.filter(|_| alpha<1.0) {
                        self.draw_music(&c.faded(1.0-alpha),cx,MUSIC_POSES[prev],ink);
                    }
                    self.draw_music(&c.faded(alpha),cx,MUSIC_POSES[slot],ink);
                }
            },
            PetState::Worried => {
                c.fill(cx+17.0,8.0,1.6,4.2,0.8,theme::YELLOW);
                c.circle(cx+17.8,14.5,0.9,theme::YELLOW);
            },
            _ => {
                // 萝卜式默认清醒状态画波浪嘴；其余皮肤沿用原版（无嘴）。
                if cute_mouth {
                    self.draw_wave_mouth(c, cx, ink);
                }
            },
        }
    }

    /// 原版眼睛：一条会形变的描边线（胶囊豆豆眼）。
    fn draw_line_eyes(&self, c: &Card, cx: f32, color: Color32) {
        for (i, eye) in self.eyes.iter().enumerate() {
            let x = cx + if i == 0 { -5.6 } else { 5.6 } + self.gaze[0];
            let y = 14.5 + self.y + eye.y + self.gaze[1];
            // 眨眼：高度压到 0，豆豆眼闭成一条线（与原版行为一致）。
            let mut e = *eye;
            e.height *= 1.0 - self.blink;
            self.stroke_eye(c, x, y, &e, color);
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

    /// 大眼睛（萝卜式）：睁眼是白色眼白 + 彩色瞳孔，且瞳孔「对眼」——左眼贴右、右眼贴左；眯眼/闭眼退化成深色弧线。
    ///
    /// 缩略态鼠标跟随的两个约定：
    /// 1. 眼白位置固定（不随 gaze 平移），只有瞳孔在眼白内小幅游走——避免整颗大眼跟着
    ///    鼠标左右平移导致眼距忽宽忽窄、与嘴巴错位产生「侧面感」。
    /// 2. 眨眼只把高度往中间压缩（宽度不变，最后合成一条闭眼弧线），而不是整体等比缩放。
    fn draw_cute_eyes(&self, c: &Card, cx: f32, ink: Color32, pupil: Color32, eye_ratio: f32) {
        // 开心：睁圆大眼、瞳孔居中；清醒/担忧等保持缩略态大眼（对眼下移）。
        // 唱歌态：仅第一个表情（拿话筒）从缩略态大眼开始渐变；其余 5 个表情是定稿的
        // 微微睁横向压扁蛋形（宽度固定 2.875、不放大）或闭眼弧线，不参与「从缩略态开始」。
        let wide = matches!(self.state, PetState::Happy);
        // 唱歌态整体独立绘制，睁眼/闭眼靠 eye.height 平滑过渡。
        let in_music = self.state == PetState::Music;
        // 缩略态大眼在基准上放大 50%（1.5 倍）；唱歌态「微微睁眼」不放大（保持 1 倍基准，见 radish-eye150 定稿）。
        let eye_scale = 1.5;
        for (i, eye) in self.eyes.iter().enumerate() {
            // 眼白固定位置：只随宠物整体 y 浮动，不随 gaze 平移。
            let x = cx + if i == 0 { -4.7 } else { 4.7 };
            let y = 14.5 + self.y + eye.y;
            // 唱歌态：眼睛尺寸只由 eye.height 连续决定，不再用「是否第一个表情」的布尔开关。
            // 切换过渡时 eye.height 是前后两个表情插值出来的，宽/高自然连续，不会硬切跳变。
            // 定稿值：4.4→3.469×4.752 缩略态大眼 / 2.5→2.875×2.5 微微睁 /
            // 2.0→2.875×2.0 侧身唱 / 0→闭眼弧线（t=0 时插值结果与原定稿完全一致）。
            if in_music {
                let open = (1.0 - self.blink).clamp(0.0, 1.0);
                let full_h = 4.4 * 0.72 * eye_scale;   // 缩略态满睁高度（4.752）
                let full_w = full_h * eye_ratio;       // 缩略态满睁宽度（3.469）
                let t = ((eye.height - 2.5) / 1.9).clamp(0.0, 1.0);
                let rw = 2.875 + t * (full_w - 2.875);
                let rh_full = if eye.height >= 2.5 { 2.5 + t * (full_h - 2.5) } else { eye.height };
                let rh = rh_full;
                // 「睁眼程度」由 eye.height 连续决定：闭眼 pose=0、微微睁=1；眨眼也走同一条
                // 渐变通道（睁→闭→睁），不再用 rh<0.14 的阈值硬切。
                // 整只眼睛做形状渐变（白色蛋形 → 墨色弯弧），不会在半路留下
                // 「细蓝线 → 突然一根黑弧线」这种生硬的中间帧。
                let open_k = smoothstep(0.15, 1.25, eye.height) * open;
                // 闭眼弧线的弯向也连续插值：睁眼 pose 的眨眼向下弯(0.65)，闭眼 pose
                // 按 eye.curve（上弯笑眼 / 下弯哼），避免切换瞬间弯向突变。
                let close_k = smoothstep(0.0, 0.8, eye.height);
                let curve = lerp(eye.curve * eye_scale, 0.65 * eye_scale, close_k);
                singing_eye(c,x,y,rw,rh,open_k,curve,ink,pupil,
                            if i==0 {1.0}else{-1.0},self.gaze);
                continue;
            }
            // 非唱歌：睁眼（开心/清醒大眼）；闭眼/眯眼（eye.height<=1.2）走弧线。
            if wide || eye.height > 1.2 {
                let open = (1.0 - self.blink).clamp(0.0, 1.0);
                // 缩略态大眼（开心/清醒/担忧）：满睁高度基准，眨眼只压高度（宽度不变）。
                let full_h = 4.4 * 0.72 * eye_scale; // 满睁高度基准（缩略态大眼）
                let wh_full = if wide {
                    full_h // 开心：睁圆大眼
                } else {
                    eye.height * 0.72 * eye_scale
                };
                let ww = wh_full * eye_ratio;
                let wh = wh_full * open;
                // 闭到只剩一条缝时画成向下弯的闭眼弧线（眼皮合拢），不再画压扁的眼白。
                if open < 0.14 {
                    let pts: Vec<_> = (0..=12).map(|n| {
                        let q = n as f32 / 6.0 - 1.0;
                        (x + q * ww * 0.9, y + (1.0 - q * q) * 0.65 * eye_scale)
                    }).collect();
                    c.path(&pts, 1.8 * eye_scale, ink);
                    continue;
                }
                eye_white(c, x, y, ww, wh, Color32::WHITE);
                // 瞳孔：对眼 + 跟随 gaze 的小幅游走，clamp 在眼白内不出界。
                let max_dx = ww - ww * 0.62;
                let max_dy = wh - wh * 0.68;
                let gx = self.gaze[0] * 0.22;
                let gy = self.gaze[1] * 0.20;
                // 开心睁圆眼瞳孔居中；清醒/担忧等都对眼 + 下移（与缩略态一致）。
                let base_dx = if wide { 0.0 } else if i == 0 { 0.30 * ww } else { -0.30 * ww };
                let base_dy = if wide { 0.0 } else { wh * 0.10 };
                let dx = (base_dx + gx).clamp(-max_dx, max_dx);
                let dy = (base_dy + gy).clamp(-max_dy, max_dy);
                // 瞳孔也用不规则蛋形（eye_white 缩小版），与眼白同源，避免标准椭圆的呆板感。
                eye_white(c, x + dx, y + dy, ww * 0.62, wh * 0.68, pupil);
            } else {
                // 闭眼/眯眼：闭成一条弧线，宽度与睁眼一致；弯向由 eye.curve 决定（正=向下弯闭眼，负=向上弯笑眼）。
                let ww = (4.4 * 0.72 * eye_scale) * eye_ratio;
                let curve = eye.curve * eye_scale;
                let pts: Vec<_> = (0..=12).map(|n| {
                    let q = n as f32 / 6.0 - 1.0;
                    (x + q * ww * 0.9, y + (1.0 - q * q) * curve)
                }).collect();
                c.path(&pts, 1.8 * eye_scale, ink);
            }
        }
    }

    /// 波浪嘴（也是缩略态默认嘴）：按定稿顶点画成一条实心粗波浪，两端补球头。
    fn draw_wave_mouth(&self, c: &Card, cx: f32, ink: Color32) {
        self.draw_mouth_shape(c,cx,mouth_shape(MusicPose::Microphone,self.age),ink);
    }

    /// 把一张嘴形画出来：上边界左→右 + 下边界右→左围成闭合多边形，端点半径 >0 才补球头。
    fn draw_mouth_shape(&self,c:&Card,cx:f32,s:MouthShape,ink:Color32) {
        let y=23.5+self.y;
        let mut pts:Vec<(f32,f32)>=Vec::with_capacity(42);
        for i in 0..21 { pts.push((cx+s.edge[i].0,y+s.edge[i].1)); }
        for i in (0..21).rev() { pts.push((cx+s.edge[21+i].0,y+s.edge[21+i].1)); }
        c.poly(&pts,ink);
        for k in 0..2 {
            if s.cap_r[k]>0.02 { c.circle(cx+s.cap[k].0,y+s.cap[k].1,s.cap_r[k],ink); }
        }
    }

    /// 开心张嘴：黑色开口 + 下方舌头。
    fn draw_open_mouth(&self, c: &Card, cx: f32, ink: Color32, tongue: Color32) {
        let y = 23.5 + self.y;
        // 黑色口腔：接近圆形的开口（参考图宽高≈1:1）。
        ellipse(c, cx, y, 3.3, 2.7, ink);
        // 舌头：更大更圆、靠下，几乎填满口腔，黑边只剩一圈窄环。
        ellipse(c, cx, y + 0.6, 2.5, 1.8, tongue);
    }

    /// 萝卜唱歌嘴：拿话筒=波浪嘴 / 侧身唱=O 嘴 / 闭眼四态=小嘴。
    /// 切换时把前后两张嘴逐点插值成「中间的一张嘴」再画，全程只有一个实心形状：
    /// 波浪的起伏、宽度、两端球头一起连续收放成小圆嘴，不会叠出双嘴或灰影。
    fn draw_music_mouth(&self,c:&Card,cx:f32,from:Option<MusicPose>,to:MusicPose,blend:f32,ink:Color32) {
        let s=match from {
            Some(from) if from!=to && blend<1.0 => {
                lerp_mouth(mouth_shape(from,self.age),mouth_shape(to,self.age),blend)
            },
            _ => mouth_shape(to,self.age),
        };
        self.draw_mouth_shape(c,cx,s,ink);
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
