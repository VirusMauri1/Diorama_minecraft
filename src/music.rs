// Musica de fondo: reproduce las canciones de la carpeta `musica` con el
// reproductor que trae Windows (MCI de winmm, parte del sistema).

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[link(name = "winmm")]
extern "system" {
    fn mciSendStringW(command: *const u16, ret: *mut u16, ret_len: u32, callback: *mut c_void) -> u32;
}

/// Formatos que se pueden reproducir.
const EXTENSIONS: [&str; 3] = ["mp3", "wav", "wma"];

/// Manda un comando de texto al reproductor y devuelve su respuesta.
fn mci(command: &str) -> Result<String, u32> {
    let cmd: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();
    let mut buf = [0u16; 128];
    let err = unsafe { mciSendStringW(cmd.as_ptr(), buf.as_mut_ptr(), buf.len() as u32, std::ptr::null_mut()) };
    if err != 0 {
        return Err(err);
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Ok(String::from_utf16_lossy(&buf[..len]))
}

pub struct Music {
    songs: Vec<PathBuf>,
    current: usize,
    open: bool,
    paused: bool,
    last_check: Instant,
}

impl Music {
    /// Busca las canciones de `dir` (en orden alfabetico) y empieza la primera.
    pub fn start(dir: &Path) -> Self {
        let mut songs: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| {
                        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                        EXTENSIONS.contains(&ext.as_str())
                    })
                    .collect()
            })
            .unwrap_or_default();
        songs.sort();
        let mut music = Music { songs, current: 0, open: false, paused: false, last_check: Instant::now() };
        if music.songs.is_empty() {
            println!("Sin musica: pon canciones .mp3 o .wav en la carpeta '{}'", dir.display());
        } else {
            println!("Canciones encontradas: {}", music.songs.len());
            music.play(0);
        }
        music
    }

    fn play(&mut self, index: usize) {
        if self.open {
            let _ = mci("close cancion");
            self.open = false;
        }
        self.current = index;
        self.paused = false;
        self.last_check = Instant::now();
        let path = &self.songs[index];
        let full = std::env::current_dir().map(|d| d.join(path)).unwrap_or_else(|_| path.clone());
        match mci(&format!("open \"{}\" type mpegvideo alias cancion", full.display())) {
            Ok(_) => {
                self.open = true;
                let _ = mci("play cancion");
                println!("Reproduciendo: {}", self.name().unwrap_or_default());
            }
            Err(e) => eprintln!("No se pudo abrir {} (error MCI {e})", path.display()),
        }
    }

    /// Pasa a la siguiente cancion (al final vuelve a la primera).
    pub fn next(&mut self) {
        if !self.songs.is_empty() {
            self.play((self.current + 1) % self.songs.len());
        }
    }

    /// Pausa o continua la cancion actual.
    pub fn set_paused(&mut self, paused: bool) {
        if !self.open || paused == self.paused {
            return;
        }
        let _ = mci(if paused { "pause cancion" } else { "resume cancion" });
        self.paused = paused;
    }

    /// Cuando termina una cancion empieza la siguiente. Se llama en cada cuadro.
    pub fn update(&mut self) {
        if !self.open || self.paused || self.last_check.elapsed() < Duration::from_millis(500) {
            return;
        }
        self.last_check = Instant::now();
        if mci("status cancion mode").is_ok_and(|mode| mode == "stopped") {
            self.next();
        }
    }

    /// Nombre de la cancion actual (sin la extension).
    pub fn name(&self) -> Option<String> {
        if !self.open {
            return None;
        }
        self.songs[self.current].file_stem().map(|s| s.to_string_lossy().into_owned())
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }
}

impl Drop for Music {
    fn drop(&mut self) {
        if self.open {
            let _ = mci("close cancion");
        }
    }
}
