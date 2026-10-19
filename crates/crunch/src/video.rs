use avmanager::texture::rgb_to_565;
use smacker::Smacker;

use crate::av::Av;
use crate::host::Host;

pub const VIDEO_TOP: i32 = 100;
const SCAN_SPACE: usize = 0x39;
const SCAN_RETURN: usize = 0x1c;

// Plays a Smacker file full screen until its last frame or a skip key, then clears the screen
pub async fn play_video_modal<H: Host>(av: &mut Av, host: &mut H, file: &str) {
    av.clear_buffer(0, 0, 0);
    av.swap_buffers(host).await;
    av.clear_buffer(0, 0, 0);
    av.swap_buffers(host).await;
    let Some(data) = host.read_file(file).await else {
        host.log(&format!("*Error opening video {file}"));
        return;
    };
    let Ok(mut smk) = Smacker::open(data) else {
        host.log(&format!("*Error decoding video {file}"));
        return;
    };
    av.sound.destroy_all_sounds();
    let frame_micros = smk.header().frame_rate_micros as u64;
    let audio_rate = smk.header().audio[0].rate;
    let audio_stereo = smk.header().audio[0].stereo;
    let width = smk.header().width as usize;
    let height = smk.header().height as usize;
    let total = smk.frame_count();
    let start = host.time_ms();
    let mut shown: u64 = 0;
    loop {
        let due_ms = start.wrapping_add(((shown * frame_micros) / 1000) as u32);
        let now = host.time_ms();
        if (now.wrapping_sub(due_ms) as i32) >= 0 {
            let Ok(Some(frame)) = smk.next_frame() else { break };
            blit_frame(av, frame.pixels, frame.palette, width, height);
            av.sound.push_video_audio(frame.audio[0], audio_rate, audio_stereo);
            shown += 1;
            if shown as usize >= total {
                break;
            }
        }
        let device = host.poll_input();
        if device.scan_keys[SCAN_SPACE] || device.scan_keys[SCAN_RETURN] {
            break;
        }
        host.end_frame(Some(av.display.front()), &mut av.sound).await;
    }
    av.sound.stop_video_audio();
    av.clear_buffer(0, 0, 0);
    av.swap_buffers(host).await;
}

// Writes one decoded frame into the visible buffer with the video offset
fn blit_frame(av: &mut Av, pixels: &[u8], palette: &[[u8; 3]; 256], width: usize, height: usize) {
    let lut: Vec<u16> = palette.iter().map(|&rgb| rgb_to_565(rgb)).collect();
    let screen_w = av.display.width() as usize;
    let screen_h = av.display.height() as usize;
    let front = av.display.front_mut();
    for row in 0..height {
        let sy = row + VIDEO_TOP as usize;
        if sy >= screen_h {
            break;
        }
        let src = &pixels[row * width..(row + 1) * width];
        let dst = &mut front[sy * screen_w..sy * screen_w + width.min(screen_w)];
        for (d, &s) in dst.iter_mut().zip(src) {
            *d = lut[s as usize];
        }
    }
}
