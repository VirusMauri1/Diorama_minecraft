// Diorama estilo Minecraft con raytracing: un barco pirata en una isla
// flotante de cerezos, de noche. Solo usa la libreria estandar de Rust.

mod dragon;
mod image_io;
mod material;
mod math;
#[cfg(windows)]
mod music;
mod noise;
mod render;
mod scene;
mod skybox;
mod texture;
mod textures;
#[cfg(windows)]
mod win32;
mod world;

use math::Vec3;
use render::{post_process, render_rows, Camera, Scene, Settings};
use std::path::{Path, PathBuf};
use std::time::Instant;

struct Args {
    width: usize,
    height: usize,
    render: Option<PathBuf>,
    samples: u32,
    max_samples: u32,
    yaw: Option<f32>,
    pitch: Option<f32>,
    dist: Option<f32>,
    export: bool,
    info: bool,
}

fn parse_args() -> Args {
    let mut a = Args {
        width: 1280,
        height: 720,
        render: None,
        samples: 32,
        max_samples: 128,
        yaw: None,
        pitch: None,
        dist: None,
        export: false,
        info: false,
    };
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    let num = |i: usize| -> f32 { argv.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0.0) };
    while i < argv.len() {
        match argv[i].as_str() {
            "--width" => a.width = num(i) as usize,
            "--height" => a.height = num(i) as usize,
            "--samples" => a.samples = num(i) as u32,
            "--max-samples" => a.max_samples = num(i) as u32,
            "--yaw" => a.yaw = Some(num(i).to_radians()),
            "--pitch" => a.pitch = Some(num(i).to_radians()),
            "--dist" => a.dist = Some(num(i)),
            "--render" => a.render = argv.get(i + 1).map(PathBuf::from),
            "--export-textures" => {
                a.export = true;
                i += 1;
                continue;
            }
            "--info" => {
                a.info = true;
                i += 1;
                continue;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                eprintln!("Argumento desconocido: {other}");
                i += 1;
                continue;
            }
        }
        i += 2;
    }
    a.width = a.width.max(64);
    a.height = a.height.max(64);
    a.samples = a.samples.max(1);
    a.max_samples = a.max_samples.max(1);
    a
}

fn print_help() {
    println!(
        "Uso: diorama_minecraft [opciones]\n\
         \n  --width N / --height N   resolucion (por defecto 1280x720)\
         \n  --max-samples N          muestras de antialiasing en modo interactivo (128)\
         \n  --render salida.png      render sin ventana y guarda un PNG\
         \n  --samples N              muestras por pixel para --render (32)\
         \n  --yaw G --pitch G --dist D   vista inicial hacia la isla (grados / bloques)\
         \n  --export-textures        guarda texturas en assets/textures y skybox en assets/skybox"
    );
}

fn print_controls() {
    println!(
        "Controles (camara libre):\
         \n  W A S D             moverse adelante / izquierda / atras / derecha\
         \n  Espacio / E         subir\
         \n  Ctrl / Q            bajar\
         \n  Shift               moverse mas rapido\
         \n  Arrastrar el raton  mirar alrededor\
         \n  Flechas             mirar alrededor\
         \n  Rueda               avanzar y retroceder\
         \n  Z X                 rotar el diorama\
         \n  + -                 acercar y alejar (hacia el centro de la isla)\
         \n  O                   rotacion automatica del diorama\
         \n  F                   pausar / continuar el vuelo del dragon\
         \n  M                   siguiente cancion\
         \n  N                   pausar / continuar la musica y el dragon\
         \n  R                   reiniciar camara\
         \n  B                   bloom on/off\
         \n  V                   haces de luz on/off\
         \n  P                   guardar captura PNG\
         \n  Esc                 salir"
    );
}

/// Vista inicial: mirando al centro de la isla segun --yaw/--pitch/--dist.
fn initial_camera(args: &Args) -> Camera {
    let mut cam = Camera::orbit(
        Camera::CENTER,
        args.yaw.unwrap_or(Camera::DEFAULT_YAW),
        args.pitch.unwrap_or(Camera::DEFAULT_PITCH),
        args.dist.unwrap_or(Camera::DEFAULT_DIST),
    );
    cam.clamp();
    cam
}

fn export_assets(scene: &Scene) {
    let dir = Path::new("assets/textures");
    for t in &scene.world.textures {
        if let Err(e) = t.save_bmp(dir) {
            eprintln!("Error guardando {}: {e}", t.name);
        }
    }
    let sky_dir = Path::new("assets/skybox");
    for f in &scene.skybox.faces {
        if let Err(e) = f.save_bmp(sky_dir) {
            eprintln!("Error guardando {}: {e}", f.name);
        }
    }
    println!("Texturas exportadas a assets/textures y assets/skybox");
}

fn print_info(scene: &Scene) {
    let w = &scene.world;
    println!(
        "
{:<22} {:<14} {:>6} {:>6} {:>6} {:>6} {:>6} {:>5} {:>5}",
        "material", "textura", "albedo", "spec", "brillo", "refl", "transp", "ior", "emis"
    );
    for m in &w.materials {
        println!(
            "{:<22} {:<14} {:>6.2} {:>6.2} {:>6.0} {:>6.2} {:>6.2} {:>5.2} {:>5.1}{}",
            m.name,
            w.textures[m.texture].name,
            m.albedo,
            m.specular,
            m.shininess,
            m.reflectivity,
            m.transparency,
            m.ior,
            m.emission,
            if m.cutout { "  (recorte alfa)" } else { "" }
        );
    }
    let names: Vec<&str> = w.blocks.iter().skip(1).map(|b| b.name).collect();
    println!("
Tipos de bloque ({}): {}
", names.len(), names.join(", "));
}

fn render_offline(scene: &Scene, args: &Args, path: &Path) {
    let (w, h) = (args.width, args.height);
    let cam = initial_camera(args);
    let settings = Settings::default();
    let mut accum = vec![Vec3::ZERO; w * h];
    let t = Instant::now();
    for s in 0..args.samples {
        render_rows(scene, &cam, &settings, w, h, 0, h, s, &mut accum, 1.0 / (s + 1) as f32);
        print!("\rMuestra {}/{}  ({:.1}s)", s + 1, args.samples, t.elapsed().as_secs_f32());
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    println!();
    let mut out = vec![0u32; w * h];
    post_process(&accum, w, h, &mut out, &settings);
    match image_io::write_png(path, w, h, &out) {
        Ok(()) => println!("Guardado {}", path.display()),
        Err(e) => eprintln!("Error guardando {}: {e}", path.display()),
    }
}

#[cfg(windows)]
fn run_interactive(scene: &mut Scene, args: &Args) {
    use std::time::Duration;
    use win32::*;

    let (w, h) = (args.width, args.height);
    let window = match Window::new("Diorama Minecraft - Raytracer en Rust", w, h) {
        Ok(win) => win,
        Err(e) => {
            eprintln!("No se pudo crear la ventana: {e}");
            return;
        }
    };
    print_controls();
    let mut music = music::Music::start(Path::new("musica"));

    let mut cam = initial_camera(args);
    let mut settings = Settings::default();

    let mut accum = vec![Vec3::ZERO; w * h];
    let mut out = vec![0u32; w * h];
    let mut low = vec![Vec3::ZERO; w * h];
    let mut low_out = vec![0u32; w * h];
    let mut preview_scale = 3usize;
    let (mut lw, mut lh) = (w / preview_scale, h / preview_scale);

    let mut samples = 0u32;
    let mut pass_row = 0usize;
    let mut band_rows = 48usize;
    let mut moving_until = Instant::now();
    let mut last = Instant::now();
    let mut auto_rotate = false;
    let mut redisplay = true;
    let mut shot = 0;
    let mut last_title = String::new();
    let mut dragon_flying = true;
    let mut paused = false;
    let mut dragon_time = 0.0f32;

    while window.pump() {
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(0.1);
        last = now;

        // Entrada
        let input = window.take_input();
        let mut changed = false;
        let rot = 1.4 * dt;
        let key = |k: i32| window.key_down(k);
        // Mirar alrededor con las flechas
        if key(VK_LEFT) {
            cam.yaw += rot;
            changed = true;
        }
        if key(VK_RIGHT) {
            cam.yaw -= rot;
            changed = true;
        }
        if key(VK_UP) {
            cam.pitch -= rot * 0.7;
            changed = true;
        }
        if key(VK_DOWN) {
            cam.pitch += rot * 0.7;
            changed = true;
        }
        if input.drag_dx != 0 || input.drag_dy != 0 {
            cam.yaw -= input.drag_dx as f32 * 0.005;
            cam.pitch += input.drag_dy as f32 * 0.005;
            changed = true;
        }

        // Moverse libremente hacia donde se mira
        let (fwd, right) = (cam.forward(), cam.right());
        let mut mv = Vec3::ZERO;
        if key('W' as i32) {
            mv += fwd;
        }
        if key('S' as i32) {
            mv -= fwd;
        }
        if key('D' as i32) {
            mv += right;
        }
        if key('A' as i32) {
            mv -= right;
        }
        if key(VK_SPACE) || key('E' as i32) || key(VK_PRIOR) {
            mv += Vec3::UP;
        }
        if key(VK_CONTROL) || key('Q' as i32) || key(VK_NEXT) {
            mv -= Vec3::UP;
        }
        if mv != Vec3::ZERO {
            let speed = if key(VK_SHIFT) { 60.0 } else { 18.0 };
            cam.pos += mv.normalize() * (speed * dt);
            changed = true;
        }
        if input.wheel != 0 {
            cam.pos += fwd * (input.wheel as f32 / 120.0 * 5.0);
            changed = true;
        }

        // Rotar el diorama y acercar / alejar
        if key('Z' as i32) {
            cam.orbit_around(Camera::CENTER, -rot);
            changed = true;
        }
        if key('X' as i32) {
            cam.orbit_around(Camera::CENTER, rot);
            changed = true;
        }
        if key(VK_ADD) || key(VK_OEM_PLUS) {
            cam.zoom_to(Camera::CENTER, 1.0 - 1.2 * dt);
            changed = true;
        }
        if key(VK_SUBTRACT) || key(VK_OEM_MINUS) {
            cam.zoom_to(Camera::CENTER, 1.0 + 1.2 * dt);
            changed = true;
        }
        for k in input.pressed {
            match k {
                VK_ESCAPE => return,
                k if k == 'O' as i32 => auto_rotate = !auto_rotate,
                k if k == 'F' as i32 => dragon_flying = !dragon_flying,
                k if k == 'M' as i32 => music.next(),
                k if k == 'N' as i32 => {
                    // Pausar la musica tambien detiene al dragon (asi la imagen se refina)
                    paused = !paused;
                    music.set_paused(paused);
                    dragon_flying = !paused;
                }
                k if k == 'R' as i32 => {
                    cam = Camera::default_view();
                    changed = true;
                }
                k if k == 'B' as i32 => {
                    settings.bloom = !settings.bloom;
                    redisplay = true;
                }
                k if k == 'V' as i32 => {
                    settings.beams = !settings.beams;
                    changed = true;
                }
                k if k == 'P' as i32 => {
                    post_process(&accum, w, h, &mut out, &settings);
                    let name = format!("captura_{shot:02}.png");
                    shot += 1;
                    match image_io::write_png(Path::new(&name), w, h, &out) {
                        Ok(()) => println!("Captura guardada: {name} ({samples} muestras)"),
                        Err(e) => eprintln!("Error guardando captura: {e}"),
                    }
                }
                _ => {}
            }
        }
        if auto_rotate {
            cam.orbit_around(Camera::CENTER, 0.3 * dt);
            changed = true;
        }
        cam.clamp();

        // El dragon vuela en circulos (mientras vuela la imagen no se refina)
        if dragon_flying {
            dragon_time += dt;
            scene.dragon.place(&mut scene.world, dragon_time);
            changed = true;
        }
        music.update();

        if changed {
            samples = 0;
            pass_row = 0;
            moving_until = now + Duration::from_millis(150);
        }

        // Render
        let status;
        if changed || now < moving_until {
            // Vista previa a baja resolucion mientras la camara se mueve
            let t = Instant::now();
            render_rows(scene, &cam, &settings, lw, lh, 0, lh, 0, &mut low, 1.0);
            post_process(&low[..lw * lh], lw, lh, &mut low_out[..lw * lh], &settings);
            window.present(&low_out[..lw * lh], lw, lh);
            let ms = t.elapsed().as_secs_f32() * 1000.0;
            // Ajustar la resolucion de la vista previa para que sea fluida
            if ms > 45.0 && preview_scale < 8 {
                preview_scale += 1;
            } else if ms < 14.0 && preview_scale > 1 {
                preview_scale -= 1;
            }
            lw = w / preview_scale;
            lh = h / preview_scale;
            status = format!("moviendo | {:.0} ms | vista previa 1/{}", ms, preview_scale);
            redisplay = false;
        } else if samples < args.max_samples {
            // Refinar en alta resolucion, por bandas de filas
            if samples == 0 && pass_row == 0 {
                render::upscale_into(&low[..lw * lh], lw, lh, &mut accum, w, h);
            }
            let y1 = (pass_row + band_rows).min(h);
            let t = Instant::now();
            render_rows(scene, &cam, &settings, w, h, pass_row, y1, samples, &mut accum, 1.0 / (samples + 1) as f32);
            let ms = t.elapsed().as_secs_f32() * 1000.0;
            let per_row = ms / (y1 - pass_row) as f32;
            band_rows = ((40.0 / per_row.max(0.01)) as usize).clamp(8, h);
            pass_row = y1;
            if pass_row >= h {
                pass_row = 0;
                samples += 1;
            }
            post_process(&accum, w, h, &mut out, &settings);
            window.present(&out, w, h);
            status = format!("refinando | muestras {}/{}", samples, args.max_samples);
            redisplay = false;
        } else {
            if redisplay {
                post_process(&accum, w, h, &mut out, &settings);
                window.present(&out, w, h);
                redisplay = false;
            }
            std::thread::sleep(Duration::from_millis(15));
            status = format!("listo | {} muestras", samples);
        }

        let title = format!(
            "Diorama Minecraft | {} | pos ({:.0}, {:.0}, {:.0})  yaw {:.0}  pitch {:.0}{}{}{}",
            status,
            cam.pos.x,
            cam.pos.y,
            cam.pos.z,
            cam.yaw.to_degrees(),
            cam.pitch.to_degrees(),
            if settings.beams { "" } else { " | sin haces" },
            if settings.bloom { "" } else { " | sin bloom" },
            match music.name() {
                Some(n) if music.is_paused() => format!(" | musica en pausa: {n}"),
                Some(n) => format!(" | musica: {n}"),
                None => String::new(),
            },
        );
        if title != last_title {
            window.set_title(&title);
            last_title = title;
        }
    }
}

fn main() {
    let args = parse_args();
    println!("Diorama Minecraft - raytracer en Rust sin librerias externas");
    let t = Instant::now();
    #[allow(unused_mut)] // solo se modifica en la ventana interactiva (Windows)
    let mut scene = scene::build();
    println!(
        "Escena lista en {:.2}s: {} bloques, {} materiales, {} texturas, {} luces, {} hilos",
        t.elapsed().as_secs_f32(),
        scene.world.count_blocks(),
        scene.world.materials.len(),
        scene.world.textures.len(),
        scene.lights.len(),
        render::thread_count()
    );

    if args.info {
        print_info(&scene);
    }
    if args.export {
        export_assets(&scene);
    }
    if let Some(path) = &args.render {
        render_offline(&scene, &args, path);
        return;
    }

    #[cfg(windows)]
    run_interactive(&mut scene, &args);

    #[cfg(not(windows))]
    {
        eprintln!("El modo interactivo solo esta disponible en Windows; se usa --render.");
        render_offline(&scene, &args, Path::new("render.png"));
    }
}
