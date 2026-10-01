// Guardar y leer imagenes sin librerias: PNG para capturas y BMP para texturas.

use std::fs;
use std::io;
use std::path::Path;

fn crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for (n, slot) in table.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *slot = c;
    }
    table
}

fn crc32(table: &[u32; 256], bytes: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in bytes {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in bytes.chunks(5552) {
        for &x in chunk {
            a += x as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

/// Guarda pixeles 0x00RRGGBB como PNG RGB de 8 bits.
pub fn write_png(path: &Path, w: usize, h: usize, pixels: &[u32]) -> io::Result<()> {
    let mut raw = Vec::with_capacity((w * 3 + 1) * h);
    for y in 0..h {
        raw.push(0); // filtro "None"
        for x in 0..w {
            let p = pixels[y * w + x];
            raw.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
        }
    }

    let mut z = vec![0x78, 0x01];
    let blocks = raw.len().div_ceil(65535).max(1);
    for (i, chunk) in raw.chunks(65535).enumerate() {
        z.push((i + 1 == blocks) as u8);
        let len = chunk.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(chunk);
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());

    let table = crc32_table();
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&table, &body).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    fs::write(path, out)
}

/// Guarda un BMP de 32 bits con alfa.
pub fn write_bmp(path: &Path, w: usize, h: usize, rgba: &[[u8; 4]]) -> io::Result<()> {
    let data_size = w * h * 4;
    let mut out = Vec::with_capacity(54 + data_size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&((54 + data_size) as u32).to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(-(h as i32)).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(data_size as u32).to_le_bytes());
    out.extend_from_slice(&[0; 16]);
    for p in rgba {
        out.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
    }
    fs::write(path, out)
}

/// Lee un BMP de 24 o 32 bits. Devuelve (ancho, alto, pixeles RGBA).
pub fn read_bmp(path: &Path) -> io::Result<(usize, usize, Vec<[u8; 4]>)> {
    let bad = |m: &str| io::Error::new(io::ErrorKind::InvalidData, m.to_string());
    let d = fs::read(path)?;
    if d.len() < 54 || &d[0..2] != b"BM" {
        return Err(bad("no es un BMP"));
    }
    let u32_at = |o: usize| u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]);
    let offset = u32_at(10) as usize;
    let w = u32_at(18) as i32;
    let h = u32_at(22) as i32;
    let bpp = u16::from_le_bytes([d[28], d[29]]);
    let compression = u32_at(30);
    if w <= 0 || h == 0 || !(bpp == 24 || bpp == 32) || !(compression == 0 || compression == 3) {
        return Err(bad("BMP no soportado (use 24/32 bits sin compresion)"));
    }
    let (w, top_down, h) = (w as usize, h < 0, h.unsigned_abs() as usize);
    let bytes = bpp as usize / 8;
    let stride = (w * bytes).div_ceil(4) * 4;
    if d.len() < offset + stride * h {
        return Err(bad("BMP truncado"));
    }
    let mut px = vec![[0u8; 4]; w * h];
    for row in 0..h {
        let y = if top_down { row } else { h - 1 - row };
        for x in 0..w {
            let o = offset + row * stride + x * bytes;
            let a = if bytes == 4 { d[o + 3] } else { 255 };
            px[y * w + x] = [d[o + 2], d[o + 1], d[o], a];
        }
    }
    Ok((w, h, px))
}
