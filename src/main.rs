use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

// ===== 窗口配置 =====
fn window_conf() -> Conf {
    Conf {
        window_title: "Dodge Rush + PowerUps (No SFX)".to_string(),
        window_width: 800,
        window_height: 600,
        high_dpi: true,
        ..Default::default()
    }
}

// ===== 常量（可调）=====
const PLAYER_W: f32 = 80.0;
const PLAYER_H: f32 = 18.0;
const PLAYER_Y: f32 = 560.0;
const PLAYER_SPEED_MAX: f32 = 520.0;
const PLAYER_ACC: f32 = 2400.0;     // 加速度
const PLAYER_DECAY: f32 = 0.0008;   // 指数衰减（松手后减速）

const OB_MIN_SIZE: f32 = 22.0;
const OB_MAX_SIZE: f32 = 60.0;
const OB_START_SPEED: f32 = 140.0;
const OB_ACC_PER_SEC: f32 = 18.0;
const SPAWN_BASE_INTERVAL: f32 = 0.9;
const SPAWN_MIN_INTERVAL: f32 = 0.25;

const FIXED_DT: f32 = 1.0 / 120.0;  // 固定物理步：120Hz

// —— 道具 —— 
const PU_SPAWN_INTERVAL: f32 = 6.0;   // 平均每 6 秒尝试一次生成
const PU_FALL_SPEED: f32 = 120.0;
const PU_SIZE: f32 = 28.0;
const SLOW_DURATION: f32 = 6.0;       // 减速持续时间
const SLOW_FACTOR: f32 = 0.5;         // 减速倍率

// ===== 模式 =====
#[derive(Clone, Copy, PartialEq)]
enum GameMode { Menu, Playing, Paused, GameOver }

// ===== 数据结构 =====
struct Player { x: f32, vx: f32 }

#[derive(Clone, Copy)]
struct Obstacle { rect: Rect, vy: f32 }

struct ObstaclePool {
    live: Vec<Obstacle>,
    dead: Vec<Obstacle>,
}
impl ObstaclePool {
    fn new() -> Self { Self { live: Vec::new(), dead: Vec::new() } }
    fn spawn(&mut self, rect: Rect, vy: f32) {
        if let Some(mut o) = self.dead.pop() {
            o.rect = rect; o.vy = vy;
            self.live.push(o);
        } else {
            self.live.push(Obstacle { rect, vy });
        }
    }
    fn update_and_sweep(&mut self, screen_h: f32, dt: f32) {
        let mut i = 0;
        while i < self.live.len() {
            let o = &mut self.live[i];
            o.rect.y += o.vy * dt;
            if o.rect.y > screen_h + 5.0 {
                let dead = self.live.swap_remove(i);
                self.dead.push(dead);
            } else {
                i += 1;
            }
        }
    }
    fn clear_all(&mut self) {
        while let Some(dead) = self.live.pop() { self.dead.push(dead); }
    }
}

// —— 道具 —— 
#[derive(Clone, Copy)]
enum PowerUpKind { Shield, Slow, Bomb }

struct PowerUp {
    rect: Rect,
    vy: f32,
    kind: PowerUpKind,
}

struct PowerUpPool {
    live: Vec<PowerUp>,
    dead: Vec<PowerUp>,
}
impl PowerUpPool {
    fn new() -> Self { Self { live: Vec::new(), dead: Vec::new() } }
    fn spawn(&mut self, x: f32, y: f32, kind: PowerUpKind) {
        let r = Rect::new(x, y, PU_SIZE, PU_SIZE);
        if let Some(mut p) = self.dead.pop() {
            p.rect = r; p.vy = PU_FALL_SPEED; p.kind = kind;
            self.live.push(p);
        } else {
            self.live.push(PowerUp { rect: r, vy: PU_FALL_SPEED, kind });
        }
    }
    fn update_and_sweep(&mut self, screen_h: f32, dt: f32) {
        let mut i = 0;
        while i < self.live.len() {
            let p = &mut self.live[i];
            p.rect.y += p.vy * dt;
            if p.rect.y > screen_h + 5.0 {
                let dead = self.live.swap_remove(i);
                self.dead.push(dead);
            } else {
                i += 1;
            }
        }
    }
   fn pick_at(&mut self, player: Rect) -> Option<PowerUpKind> {
    let mut i = 0;
    while i < self.live.len() {
        if rects_overlap(self.live[i].rect, player) {
            // 先拿到种类，再移动对象
            let kind = self.live[i].kind;
            let picked = self.live.swap_remove(i);
            self.dead.push(picked);
            return Some(kind);
        } else {
            i += 1;
        }
    }
    None
}

}

#[derive(Serialize, Deserialize, Default)]
struct Save { best: i32 }

struct Resources {
    font: Font,
}

struct Game {
    mode: GameMode,
    player: Player,
    obs: ObstaclePool,
    pus: PowerUpPool,
    time_tick: f32,            // 计分步进
    score: i32,
    best_score: i32,
    spawn_timer: f32,
    spawn_interval: f32,
    fall_speed: f32,
    shake: f32,                // 相机震动强度
    // —— 道具状态 ——
    shield: u32,               // 护盾层数
    slow_timer: f32,           // 减速剩余时间
    pu_spawn_timer: f32,       // 道具生成计时器
}

impl Game {
    fn new(best: i32) -> Self {
        Self {
            mode: GameMode::Menu,
            player: Player { x: 0.0, vx: 0.0 },
            obs: ObstaclePool::new(),
            pus: PowerUpPool::new(),
            time_tick: 0.0,
            score: 0,
            best_score: best,
            spawn_timer: 0.0,
            spawn_interval: SPAWN_BASE_INTERVAL,
            fall_speed: OB_START_SPEED,
            shake: 0.0,
            shield: 0,
            slow_timer: 0.0,
            pu_spawn_timer: 0.0,
        }
    }
    fn reset_round(&mut self) {
        self.player.x = screen_width() * 0.5 - PLAYER_W * 0.5;
        self.player.vx = 0.0;
        self.obs.live.clear(); self.obs.dead.clear();
        self.pus.live.clear(); self.pus.dead.clear();
        self.time_tick = 0.0;
        self.score = 0;
        self.spawn_timer = 0.0;
        self.spawn_interval = SPAWN_BASE_INTERVAL;
        self.fall_speed = OB_START_SPEED;
        self.shake = 0.0;
        self.shield = 0;
        self.slow_timer = 0.0;
        self.pu_spawn_timer = 0.0;
        self.mode = GameMode::Playing;
    }
}

// ===== 工具函数 =====
fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}

fn input_axis() -> f32 {
    let mut dir = 0.0;
    if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) { dir -= 1.0; }
    if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) { dir += 1.0; }
    dir
}

fn difficulty_curve(elapsed: f32, fall_base: f32, spawn_base: f32) -> (f32, f32) {
    let fall = fall_base + elapsed * OB_ACC_PER_SEC;
    let spawn = (spawn_base - elapsed * 0.02).max(SPAWN_MIN_INTERVAL);
    (fall, spawn)
}

fn save_best(best: i32) {
    let _ = std::fs::write("save.json", serde_json::to_string(&Save { best }).unwrap());
}

fn load_best() -> i32 {
    std::fs::read_to_string("save.json")
        .ok()
        .and_then(|s| serde_json::from_str::<Save>(&s).ok())
        .map(|v| v.best)
        .unwrap_or(0)
}

// ===== 逻辑：固定时间步更新 =====
fn update_game(game: &mut Game, dt: f32, _res: &Resources) {
    match game.mode {
        GameMode::Menu => {
            if is_key_pressed(KeyCode::Space) { game.reset_round(); }
        }
        GameMode::Playing => {
            // —— 移动：加速度+限速+衰减 —— 
            let dir = input_axis();
            if dir.abs() > 0.0 {
                game.player.vx += dir * PLAYER_ACC * dt;
            } else {
                game.player.vx *= (1.0 - PLAYER_DECAY).powf(dt * 1000.0);
            }
            game.player.vx = game.player.vx.clamp(-PLAYER_SPEED_MAX, PLAYER_SPEED_MAX);
            game.player.x = (game.player.x + game.player.vx * dt)
                .clamp(0.0, screen_width() - PLAYER_W);

            // —— 减速效果衰减 —— 
            if game.slow_timer > 0.0 {
                game.slow_timer = (game.slow_timer - dt).max(0.0);
            }
            let slow_mul = if game.slow_timer > 0.0 { SLOW_FACTOR } else { 1.0 };

            // —— 难度递增 —— 
            let elapsed = macroquad::time::get_time() as f32;
            let (fall_spd, spawn_itv) = difficulty_curve(elapsed, OB_START_SPEED, game.spawn_interval);
            game.fall_speed = fall_spd * slow_mul;
            game.spawn_interval = (spawn_itv / slow_mul).max(SPAWN_MIN_INTERVAL);

            // —— 生成障碍 —— 
            game.spawn_timer += dt;
            if game.spawn_timer >= game.spawn_interval {
                game.spawn_timer = 0.0;
                let size = rand::gen_range(OB_MIN_SIZE, OB_MAX_SIZE);
                let x = rand::gen_range(0.0, screen_width() - size);
                let y = -size - 10.0;
                let vy = game.fall_speed * rand::gen_range(0.9, 1.3);
                game.obs.spawn(Rect::new(x, y, size, size), vy);
            }

            // —— 生成道具（随机一种） —— 
            game.pu_spawn_timer += dt;
            if game.pu_spawn_timer >= PU_SPAWN_INTERVAL {
                game.pu_spawn_timer = 0.0;
                if rand::gen_range(0.0, 1.0) < 0.30 {
                    let x = rand::gen_range(PU_SIZE, screen_width() - PU_SIZE);
                    let kind = match rand::gen_range(0, 3) {
                        0 => PowerUpKind::Shield,
                        1 => PowerUpKind::Slow,
                        _ => PowerUpKind::Bomb,
                    };
                    game.pus.spawn(x, -PU_SIZE - 8.0, kind);
                }
            }

            // —— 更新障碍 & 道具 —— 
            game.obs.update_and_sweep(screen_height(), dt);
            game.pus.update_and_sweep(screen_height(), dt);

            // —— 计分 —— 
            game.time_tick += dt;
            while game.time_tick >= 0.4 {
                game.time_tick -= 0.4;
                game.score += 1;
            }

            // —— 拾取道具 —— 
            let pbox = Rect::new(game.player.x, PLAYER_Y, PLAYER_W, PLAYER_H);
            if let Some(kind) = game.pus.pick_at(pbox) {
                match kind {
                    PowerUpKind::Shield => { game.shield = (game.shield + 1).min(3); }
                    PowerUpKind::Slow   => { game.slow_timer = SLOW_DURATION; }
                    PowerUpKind::Bomb   => { game.obs.clear_all(); game.shake = 6.0; }
                }
            }

            // —— 碰撞（护盾可抵消；命中盒瘦身） —— 
            let mut hit = Rect::new(game.player.x, PLAYER_Y, PLAYER_W, PLAYER_H);
            hit.x += 6.0; hit.w -= 12.0;

            let mut collided_index: Option<usize> = None;
            for (i, o) in game.obs.live.iter().enumerate() {
                if rects_overlap(o.rect, hit) { collided_index = Some(i); break; }
            }
            if let Some(i) = collided_index {
                if game.shield > 0 {
                    // 护盾抵消一次：移除该障碍、护盾-1、轻微震屏
                    let dead = game.obs.live.swap_remove(i);
                    game.obs.dead.push(dead);
                    game.shield -= 1;
                    game.shake = game.shake.max(4.0);
                } else {
                    // 游戏结束
                    game.best_score = game.best_score.max(game.score);
                    save_best(game.best_score);
                    game.mode = GameMode::GameOver;
                    game.shake = 10.0;
                }
            }

            if is_key_pressed(KeyCode::P) { game.mode = GameMode::Paused; }
        }
        GameMode::Paused => {
            if is_key_pressed(KeyCode::P) { game.mode = GameMode::Playing; }
            if is_key_pressed(KeyCode::R) { game.reset_round(); }
            if is_key_pressed(KeyCode::Escape) { game.mode = GameMode::Menu; }
        }
        GameMode::GameOver => {
            if is_key_pressed(KeyCode::R) { game.reset_round(); }
            if is_key_pressed(KeyCode::Escape) { game.mode = GameMode::Menu; }
        }
    }

    // 震动衰减
    if game.shake > 0.0 {
        game.shake = (game.shake - 60.0 * dt).max(0.0);
    }
}

// ===== 绘制 =====
fn draw_text_center(font: &Font, text: &str, y: f32, size: f32, color: Color) {
    let dim = measure_text(text, Some(font), size as u16, 1.0);
    let x = screen_width() * 0.5 - dim.width * 0.5;
    draw_text_ex(text, x, y, TextParams { font: Some(font), font_size: size as u16, color, ..Default::default() });
}

fn draw_hud(font: &Font, game: &Game) {
    draw_rectangle(0.0, 0.0, screen_width(), 46.0, Color::from_rgba(20, 24, 32, 220));
    draw_text_ex(&format!("SCORE: {:>4}", game.score), 16.0, 30.0, TextParams { font: Some(font), font_size: 28, color: YELLOW, ..Default::default() });
    draw_text_ex(&format!("BEST:  {:>4}", game.best_score), 190.0, 30.0, TextParams { font: Some(font), font_size: 28, color: GOLD, ..Default::default() });

    // 道具状态提示
    let slow_txt = if game.slow_timer > 0.0 { format!("SLOW:{:.1}s", game.slow_timer) } else { "SLOW:OFF".to_string() };
    let shield_txt = format!("SHIELD:{}", game.shield);
    draw_text_ex(&shield_txt, screen_width() - 300.0, 30.0, TextParams { font: Some(font), font_size: 22, color: SKYBLUE, ..Default::default() });
    draw_text_ex(&slow_txt,   screen_width() - 170.0, 30.0, TextParams { font: Some(font), font_size: 22, color: LIME, ..Default::default() });
}

fn draw_player(game: &Game) {
    let r = Rect::new(game.player.x, PLAYER_Y, PLAYER_W, PLAYER_H);
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(90, 200, 255, 255));
    draw_rectangle(r.x + 10.0, r.y + 4.0, r.w - 20.0, 3.0, Color::from_rgba(200, 245, 255, 255));
    // 若有护盾，画一圈外发光
    if game.shield > 0 {
        draw_rectangle_lines(r.x - 4.0, r.y - 4.0, r.w + 8.0, r.h + 8.0, 2.0, Color::from_rgba(120, 220, 255, 220));
    }
}

fn draw_obstacles(game: &Game) {
    for o in &game.obs.live {
        draw_rectangle(o.rect.x, o.rect.y, o.rect.w, o.rect.h, Color::from_rgba(255, 100, 100, 230));
        draw_rectangle_lines(o.rect.x, o.rect.y, o.rect.w, o.rect.h, 2.0, Color::from_rgba(255, 180, 180, 240));
    }
}

fn draw_powerups(game: &Game) {
    for p in &game.pus.live {
        match p.kind {
            PowerUpKind::Shield => {
                draw_circle(p.rect.x + p.rect.w/2.0, p.rect.y + p.rect.h/2.0, p.rect.w*0.45, SKYBLUE);
            }
            PowerUpKind::Slow => {
                draw_circle(p.rect.x + p.rect.w/2.0, p.rect.y + p.rect.h/2.0, p.rect.w*0.45, LIME);
            }
            PowerUpKind::Bomb => {
                draw_circle(p.rect.x + p.rect.w/2.0, p.rect.y + p.rect.h/2.0, p.rect.w*0.45, ORANGE);
            }
        }
        draw_rectangle_lines(p.rect.x, p.rect.y, p.rect.w, p.rect.h, 1.5, WHITE);
    }
}

fn draw_game(game: &Game, res: &Resources) {
    // 简单相机震动偏移
    let ox = if game.shake > 0.0 { rand::gen_range(-game.shake, game.shake) } else { 0.0 };
    let oy = if game.shake > 0.0 { rand::gen_range(-game.shake, game.shake) } else { 0.0 };

    set_camera(&Camera2D {
    target: vec2(screen_width() / 2.0 + ox, screen_height() / 2.0 + oy),
    zoom: vec2(2.0 / screen_width(),  2.0 / screen_height()), // <-- 去掉负号，保持 y 向下
    ..Default::default()
});


    clear_background(Color::from_rgba(14, 17, 22, 255));

    match game.mode {
        GameMode::Menu => {
            draw_text_center(&res.font, "Dodge Rush", 140.0, 62.0, SKYBLUE);
            draw_text_center(&res.font, "左右移动躲避方块，收集道具增强能力", 200.0, 24.0, LIGHTGRAY);
            draw_text_center(&res.font, "按 [SPACE] 开始", 300.0, 28.0, WHITE);
        }
        GameMode::Playing => {
            draw_hud(&res.font, game);
            draw_player(game);
            draw_obstacles(game);
            draw_powerups(game);
        }
        GameMode::Paused => {
            draw_hud(&res.font, game);
            draw_player(game);
            draw_obstacles(game);
            draw_powerups(game);
            draw_text_center(&res.font, "已暂停 [P]继续 / [R]重开 / [ESC]菜单", 300.0, 28.0, YELLOW);
        }
        GameMode::GameOver => {
            draw_hud(&res.font, game);
            draw_player(game);
            draw_obstacles(game);
            draw_powerups(game);
            draw_text_center(&res.font, "💥 游戏结束!", 250.0, 44.0, RED);
            draw_text_center(&res.font, &format!("得分：{}   最高：{}", game.score, game.best_score), 300.0, 28.0, WHITE);
            draw_text_center(&res.font, "[R] 再来一局   [ESC] 返回菜单", 350.0, 24.0, ORANGE);
        }
    }

    set_default_camera();
}

// ===== 主循环（固定物理步 + 渲染分离）=====
#[macroquad::main(window_conf)]
async fn main() {
    // 字体
    let font = load_ttf_font("assets/NotoSansCJKsc-Regular.otf")
        .await
        .expect("无法加载中文字体：assets/NotoSansCJKsc-Regular.otf");

    let res = Resources { font };
    let best = load_best();
    let mut game = Game::new(best);
    game.player.x = screen_width() * 0.5 - PLAYER_W * 0.5;

    let mut acc = 0.0f32;

    loop {
        let dt = get_frame_time();
        acc += dt;
        while acc >= FIXED_DT {
            update_game(&mut game, FIXED_DT, &res);
            acc -= FIXED_DT;
        }
        draw_game(&game, &res);
        next_frame().await;
    }
}
