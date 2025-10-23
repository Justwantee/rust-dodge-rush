use macroquad::prelude::*;

// ===== 窗口配置 =====
fn window_conf() -> Conf {
    Conf {
        window_title: "Dodge Rush".to_string(),
        window_width: 800,
        window_height: 600,
        high_dpi: true,
        ..Default::default()
    }
}

// ===== 常量 =====
const PLAYER_W: f32 = 80.0;
const PLAYER_H: f32 = 18.0;
const PLAYER_Y: f32 = 560.0;
const PLAYER_SPEED: f32 = 520.0;

const OB_MIN_SIZE: f32 = 22.0;
const OB_MAX_SIZE: f32 = 60.0;
const OB_START_SPEED: f32 = 140.0;
const OB_ACC_PER_SEC: f32 = 18.0;
const SPAWN_BASE_INTERVAL: f32 = 0.9;
const SPAWN_MIN_INTERVAL: f32 = 0.25;

#[derive(Clone, Copy, PartialEq)]
enum GameMode {
    Menu,
    Playing,
    Paused,
    GameOver,
}

struct Player { x: f32 }

struct Obstacle {
    rect: Rect,
    vy: f32,
}

struct Game {
    mode: GameMode,
    player: Player,
    obstacles: Vec<Obstacle>,
    time_tick: f32,
    score: i32,
    best_score: i32,
    spawn_timer: f32,
    spawn_interval: f32,
    fall_speed: f32,
}

impl Game {
    fn new() -> Self {
        Self {
            mode: GameMode::Menu,
            player: Player { x: 0.0 },
            obstacles: Vec::new(),
            time_tick: 0.0,
            score: 0,
            best_score: 0,
            spawn_timer: 0.0,
            spawn_interval: SPAWN_BASE_INTERVAL,
            fall_speed: OB_START_SPEED,
        }
    }

    fn reset_round(&mut self) {
        self.player.x = screen_width() * 0.5 - PLAYER_W * 0.5;
        self.obstacles.clear();
        self.time_tick = 0.0;
        self.score = 0;
        self.spawn_timer = 0.0;
        self.spawn_interval = SPAWN_BASE_INTERVAL;
        self.fall_speed = OB_START_SPEED;
        self.mode = GameMode::Playing;
    }
}

// ===== 工具函数 =====
fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}

fn draw_text_center(font: &Font, text: &str, y: f32, size: f32, color: Color) {
    let dim = measure_text(text, Some(font), size as u16, 1.0);
    let x = screen_width() * 0.5 - dim.width * 0.5;
    draw_text_ex(
        text,
        x,
        y,
        TextParams { font: Some(font), font_size: size as u16, color, ..Default::default() },
    );
}

fn input_axis() -> f32 {
    let mut dir = 0.0;
    if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) { dir -= 1.0; }
    if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) { dir += 1.0; }
    dir
}

fn spawn_obstacle(game: &mut Game) {
    let size = rand::gen_range(OB_MIN_SIZE, OB_MAX_SIZE);
    let x = rand::gen_range(0.0, screen_width() - size);
    let y = -size - 10.0;
    let rect = Rect::new(x, y, size, size);
    let vy = game.fall_speed * rand::gen_range(0.9, 1.3);
    game.obstacles.push(Obstacle { rect, vy });
}

fn draw_hud(font: &Font, game: &Game) {
    draw_rectangle(0.0, 0.0, screen_width(), 46.0, Color::from_rgba(20, 24, 32, 220));

    draw_text_ex(
        &format!("SCORE: {:>4}", game.score),
        16.0,
        30.0,
        TextParams { font: Some(font), font_size: 28, color: YELLOW, ..Default::default() },
    );

    draw_text_ex(
        &format!("BEST:  {:>4}", game.best_score),
        190.0,
        30.0,
        TextParams { font: Some(font), font_size: 28, color: GOLD, ..Default::default() },
    );

    draw_text_ex(
        "[←/→]移动  [P]暂停  [R]重开  [ESC]菜单",
        screen_width() - 410.0,
        30.0,
        TextParams { font: Some(font), font_size: 22, color: LIGHTGRAY, ..Default::default() },
    );
}

// ===== 主程序 =====
#[macroquad::main(window_conf)]
async fn main() {
    // 加载中文字体（确保 assets/NotoSansCJKsc-Regular.otf 存在）
    let font = load_ttf_font("assets/NotoSansCJKsc-Regular.otf")
        .await
        .expect("无法加载中文字体，请确认 assets/NotoSansCJKsc-Regular.otf 存在");

    let mut game = Game::new();
    let mut flash_time: f32 = 0.0;
    game.player.x = screen_width() * 0.5 - PLAYER_W * 0.5;

    loop {
        let dt = get_frame_time();
        clear_background(Color::from_rgba(14, 17, 22, 255));

        match game.mode {
            GameMode::Menu => {
                draw_text_center(&font, "Dodge Rush", 140.0, 62.0, SKYBLUE);
                draw_text_center(&font, "左右移动躲避方块，活得越久分数越高", 200.0, 24.0, LIGHTGRAY);
                draw_text_center(&font, "按 [SPACE] 开始， [H] 查看操作说明", 300.0, 28.0, WHITE);

                if is_key_pressed(KeyCode::H) { show_help_overlay(&font).await; }
                if is_key_pressed(KeyCode::Space) { game.reset_round(); }
            }
            GameMode::Playing => {
                // 输入 & 移动
                let dir = input_axis();
                game.player.x = (game.player.x + dir * PLAYER_SPEED * dt)
                    .clamp(0.0, screen_width() - PLAYER_W);

                // 生成障碍
                game.spawn_timer += dt;
                if game.spawn_timer >= game.spawn_interval {
                    game.spawn_timer = 0.0;
                    spawn_obstacle(&mut game);
                    game.spawn_interval = (game.spawn_interval - 0.02).max(SPAWN_MIN_INTERVAL);
                }

                // 难度递增
                game.fall_speed += OB_ACC_PER_SEC * dt;

                // 更新障碍
                for ob in &mut game.obstacles {
                    ob.rect.y += ob.vy * dt;
                }
                game.obstacles.retain(|o| o.rect.y <= screen_height() + 5.0);

                // 计分
                game.time_tick += dt;
                while game.time_tick >= 0.4 {
                    game.time_tick -= 0.4;
                    game.score += 1;
                }

                // 碰撞
                let player_rect = Rect::new(game.player.x, PLAYER_Y, PLAYER_W, PLAYER_H);
                if game.obstacles.iter().any(|o| rects_overlap(o.rect, player_rect)) {
                    game.best_score = game.best_score.max(game.score);
                    game.mode = GameMode::GameOver;
                    flash_time = 0.35;
                }

                // 绘制
                draw_hud(&font, &game);
                draw_player(&game);
                draw_obstacles(&game);

                if is_key_pressed(KeyCode::P) { game.mode = GameMode::Paused; }
            }
            GameMode::Paused => {
                draw_hud(&font, &game);
                draw_player(&game);
                draw_obstacles(&game);
                draw_text_center(&font, "已暂停 [P]继续 / [R]重开 / [ESC]菜单", 300.0, 28.0, YELLOW);
                if is_key_pressed(KeyCode::P) { game.mode = GameMode::Playing; }
                if is_key_pressed(KeyCode::R) { game.reset_round(); }
                if is_key_pressed(KeyCode::Escape) { game.mode = GameMode::Menu; }
            }
            GameMode::GameOver => {
                if flash_time > 0.0 {
                    flash_time -= dt;
                    let alpha = ((flash_time / 0.35) * 120.0).clamp(0.0, 120.0) as u8;
                    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(255, 40, 40, alpha));
                }

                draw_hud(&font, &game);
                draw_player(&game);
                draw_obstacles(&game);
                draw_text_center(&font, "💥 游戏结束!", 250.0, 44.0, RED);
                draw_text_center(&font, &format!("得分：{}   最高：{}", game.score, game.best_score), 300.0, 28.0, WHITE);
                draw_text_center(&font, "[R] 再来一局   [ESC] 返回菜单", 350.0, 24.0, ORANGE);

                if is_key_pressed(KeyCode::R) { game.reset_round(); }
                if is_key_pressed(KeyCode::Escape) { game.mode = GameMode::Menu; }
            }
        }

        next_frame().await;
    }
}

// ===== 绘制元素 =====
fn draw_player(game: &Game) {
    let r = Rect::new(game.player.x, PLAYER_Y, PLAYER_W, PLAYER_H);
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(90, 200, 255, 255));
    draw_rectangle(r.x + 10.0, r.y + 4.0, r.w - 20.0, 3.0, Color::from_rgba(200, 245, 255, 255));
}

fn draw_obstacles(game: &Game) {
    for o in &game.obstacles {
        draw_rectangle(o.rect.x, o.rect.y, o.rect.w, o.rect.h, Color::from_rgba(255, 100, 100, 230));
        draw_rectangle_lines(o.rect.x, o.rect.y, o.rect.w, o.rect.h, 2.0, Color::from_rgba(255, 180, 180, 240));
    }
}

// ===== 帮助页（异步） =====
async fn show_help_overlay(font: &Font) {
    loop {
        clear_background(Color::from_rgba(14, 17, 22, 255));
        draw_text_center(font, "操作说明", 120.0, 48.0, WHITE);
        draw_text_center(font, "←/→ 或 A/D：左右移动", 200.0, 28.0, LIGHTGRAY);
        draw_text_center(font, "P：暂停 / 继续", 240.0, 28.0, LIGHTGRAY);
        draw_text_center(font, "R：重新开始", 280.0, 28.0, LIGHTGRAY);
        draw_text_center(font, "ESC：返回菜单", 320.0, 28.0, LIGHTGRAY);
        draw_text_center(font, "按 [ESC] 返回", 420.0, 24.0, YELLOW);
        if is_key_pressed(KeyCode::Escape) { break; }
        next_frame().await;
    }
}
