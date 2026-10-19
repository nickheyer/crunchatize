#!/usr/bin/env python3
# Builds the Go Online menu panel, pill buttons, white font, helper cursor and scene from the original game art
import os
import struct
import sys

from PIL import Image

MAGIC = 0x1234ACDC
WAM_MAGIC = 0x7788ACDC
KEY = (0, 255, 0)
PANEL_W, PANEL_H = 352, 224
PANEL_X, PANEL_Y = 144, 128
PILL_W = 128
CORNER = 20
EDGE = 20
TITLE = "PLAY ONLINE"
PILLS = ["host", "join", "back", "leave"]
FONT_SCALE_SMALL = 2 / 3
FONT_SCALE_PILL = 0.8


def read_bam(path):
    data = open(path, "rb").read()
    magic, version, nframes, transparent, color_key, loop_mode, fps = struct.unpack_from("<IIHHIhH", data, 0)
    assert magic == MAGIC, path
    off = 20
    bbox = struct.unpack_from("<iiii", data, off)
    off += 16
    palette = []
    if version > 1:
        (n,) = struct.unpack_from("<I", data, off)
        off += 4
        for _ in range(n):
            palette.append(tuple(data[off : off + 3]))
            off += 3
    frames = []
    for i in range(nframes):
        x, y = struct.unpack_from("<hh", data, off)
        off += 4
        assert data[off] == ord("z")
        fmt = data[off + 1]
        w, h, hx, hy = struct.unpack_from(">HHhh", data, off + 2)
        off += 10
        count = w * h
        px = []
        if fmt == 0:
            px = list(struct.unpack_from(">%dH" % count, data, off))
            off += 2 * count
        else:
            size = 2 if fmt == 1 else 1
            run = 0
            val = 0
            while len(px) < count:
                if run == 0:
                    val = struct.unpack_from(">H" if fmt == 1 else "B", data, off)[0]
                    off += size
                    if val == 0:
                        run = struct.unpack_from(">H" if fmt == 1 else "B", data, off)[0]
                        off += size
                px.append(val)
                run = max(run - 1, 0)
        frames.append(dict(x=x, y=y, w=w, h=h, hx=hx, hy=hy, fmt=fmt, px=px))
    return dict(transparent=transparent, color_key=color_key, loop_mode=loop_mode, fps=fps, bbox=bbox, palette=palette, frames=frames)


def pixels(img):
    return list(img.get_flattened_data()) if hasattr(img, "get_flattened_data") else list(img.getdata())


def to_565(rgb):
    return ((rgb[0] >> 3) << 11) | ((rgb[1] >> 2) << 5) | (rgb[2] >> 3)


# Expands one frame to RGBA, transparent where the pixel matches the colour key
def bam_image(bam, fi=0):
    f = bam["frames"][fi]
    key = bam["color_key"]
    key565 = to_565(((key >> 16) & 0xFF, (key >> 8) & 0xFF, key & 0xFF))
    out = []
    for p in f["px"]:
        if f["fmt"] == 2:
            rgb = bam["palette"][p]
            v = to_565(rgb)
        else:
            v = p
            rgb = (((v >> 11) & 0x1F) << 3, ((v >> 5) & 0x3F) << 2, (v & 0x1F) << 3)
        a = 0 if (bam["transparent"] and v == key565) else 255
        out.append((rgb[0], rgb[1], rgb[2], a))
    img = Image.new("RGBA", (f["w"], f["h"]))
    img.putdata(out)
    return img


def rle8(indices):
    out = bytearray()
    i = 0
    n = len(indices)
    while i < n:
        v = indices[i]
        if v == 0:
            run = 1
            while i + run < n and indices[i + run] == 0 and run < 255:
                run += 1
            out += bytes([0, run])
            i += run
        else:
            out.append(v)
            i += 1
    return bytes(out)


# Maps an RGBA image to palette indices with index 0 reserved for the colour key
def index_image(img):
    px = pixels(img)
    opaque = {}
    for r, g, b, a in px:
        if a:
            opaque[(r, g, b)] = opaque.get((r, g, b), 0) + 1
    if len(opaque) > 255:
        fill = max(opaque, key=opaque.get)
        rgb = Image.new("RGB", img.size)
        rgb.putdata([(r, g, b) if a else fill for r, g, b, a in px])
        q = rgb.quantize(colors=255, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
        qpal = q.getpalette()[: 255 * 3]
        colours = [tuple(qpal[i : i + 3]) for i in range(0, len(qpal), 3)]
        qpx = pixels(q)
        palette = [KEY] + colours
        indices = [0 if a == 0 else qpx[i] + 1 for i, (r, g, b, a) in enumerate(px)]
    else:
        colours = sorted(opaque, key=opaque.get, reverse=True)
        lut = {c: i + 1 for i, c in enumerate(colours)}
        palette = [KEY] + colours
        indices = [0 if a == 0 else lut[(r, g, b)] for r, g, b, a in px]
    palette = [c if (i == 0 or to_565(c) != to_565(KEY)) else (c[0], 240, c[2]) for i, c in enumerate(palette)]
    palette += [(0, 0, 0)] * (256 - len(palette))
    return palette, indices


# Writes one or more RGBA frames as an indexed BAM sharing a palette built from the first frame
def write_bam(path, frames, fps=15, bbox=None, loop_mode=0):
    if isinstance(frames, Image.Image):
        frames = [(frames, (0, 0), (0, 0))]
    img0, _, hot0 = frames[0]
    w, h = img0.size
    if bbox is None:
        bbox = (hot0[0], hot0[1], hot0[0] + w, hot0[1] + h)
    strip = Image.new("RGBA", (w, h * len(frames)))
    for i, (img, _, _) in enumerate(frames):
        strip.paste(img, (0, i * h))
    palette, indices = index_image(strip)
    out = bytearray()
    out += struct.pack("<IIHHIhH", MAGIC, 2, len(frames), 1, (KEY[0] << 16) | (KEY[1] << 8) | KEY[2], loop_mode, fps)
    out += struct.pack("<iiii", *bbox)
    out += struct.pack("<I", 256)
    for c in palette:
        out += bytes(c)
    for i, (img, frame_xy, hot) in enumerate(frames):
        out += struct.pack("<hh", *frame_xy)
        out += b"z" + bytes([2]) + struct.pack(">HHhh", w, h, hot[0], hot[1])
        out += rle8(indices[i * w * h : (i + 1) * w * h])
    open(path, "wb").write(out)


# Glyph cells of the yellow Kahuna sheet keyed by character, in .fnt order
def load_glyphs(data_dir):
    lines = open(os.path.join(data_dir, "font.fnt"), "rb").read().replace(b"\r", b"").split(b"\n")
    assert lines[0].startswith(b"#BAM")
    width = int(lines[1].split()[1])
    height = int(lines[2].split()[1])
    chars = [l[0:1].decode() for l in lines[3:] if l]
    bam = read_bam(os.path.join(data_dir, "kahunafont_yellow.bam"))
    sheet = bam_image(bam)
    per_row = 0
    cx = 0
    while bam["bbox"][2] - 6 > cx:
        cx += width
        per_row += 1
    glyphs = {}
    for k, ch in enumerate(chars):
        col, row = k % per_row, k // per_row
        glyphs[ch] = sheet.crop((col * width, row * height, col * width + width, row * height + height))
    return chars, glyphs, width, height


# Turns the yellow glossy lettering white, keeping the dark outline
def whiten(img):
    out = []
    for r, g, b, a in pixels(img):
        lum = 0.3 * r + 0.59 * g + 0.11 * b
        v = int(round(255 * (lum / 255) ** 0.45))
        out.append((v, v, v, a))
    res = Image.new("RGBA", img.size)
    res.putdata(out)
    return res


def scale_rgba(img, scale):
    w, h = img.size
    size = (max(1, round(w * scale)), max(1, round(h * scale)))
    pre = Image.new("RGBA", img.size)
    pre.putdata([(r * a // 255, g * a // 255, b * a // 255, a) for r, g, b, a in pixels(img)])
    small = pre.resize(size, Image.Resampling.LANCZOS)
    out = []
    for r, g, b, a in pixels(small):
        if a < 128:
            out.append((0, 0, 0, 0))
        else:
            out.append((min(255, r * 255 // a), min(255, g * 255 // a), min(255, b * 255 // a), 255))
    res = Image.new("RGBA", size)
    res.putdata(out)
    return res


# Draws text the way the engine does, each glyph cell advancing by its width less two
def draw_text(target, text, glyphs, cell_w, x, y, centred=True):
    advance = cell_w - 2
    width = sum(advance if ch != " " else cell_w for ch in text)
    if centred:
        x -= width // 2
    for ch in text:
        if ch == " ":
            x += cell_w
            continue
        target.alpha_composite(glyphs[ch], (x, y))
        x += advance


def make_white_font(out_dir, chars, glyphs, width, height):
    cell_w = round(width * FONT_SCALE_SMALL)
    cell_h = round(height * FONT_SCALE_SMALL)
    per_row = 10
    rows = (len(chars) + per_row - 1) // per_row
    sheet = Image.new("RGBA", (cell_w * per_row, cell_h * rows), (0, 0, 0, 0))
    small = {}
    for k, ch in enumerate(chars):
        g = scale_rgba(whiten(glyphs[ch]), FONT_SCALE_SMALL)
        small[ch] = g
        sheet.alpha_composite(g, ((k % per_row) * cell_w, (k // per_row) * cell_h))
    write_bam(os.path.join(out_dir, "kahunafont_white.bam"), sheet, fps=10)
    with open(os.path.join(out_dir, "fontwhite.fnt"), "wb") as f:
        f.write(b"#BAM KahunaFont_white.bam\r\n#WIDTH %d\r\n#HEIGHT %d\r\n" % (cell_w, cell_h))
        for ch in chars:
            f.write(ch.encode() + b"\r\n")
    return small, cell_w


def make_panel(out_dir, data_dir, small_glyphs, small_w):
    src = bam_image(read_bam(os.path.join(data_dir, "deletebox.bam")))
    sw, sh = src.size
    fill = src.getpixel((40, 60))
    assert fill[2] > 200 and fill[0] < 16 and fill[1] < 16, fill
    panel = Image.new("RGBA", (PANEL_W, PANEL_H), (0, 0, 0, 0))
    inner = Image.new("RGBA", (PANEL_W - 2 * CORNER, PANEL_H - 2 * CORNER), fill)
    panel.paste(inner, (CORNER, CORNER))
    top = src.crop((128, 0, 129, EDGE)).resize((PANEL_W - 2 * CORNER, EDGE), Image.Resampling.NEAREST)
    bottom = src.crop((128, sh - EDGE, 129, sh)).resize((PANEL_W - 2 * CORNER, EDGE), Image.Resampling.NEAREST)
    left = src.crop((0, 60, EDGE, 61)).resize((EDGE, PANEL_H - 2 * CORNER), Image.Resampling.NEAREST)
    right = src.crop((sw - EDGE, 60, sw, 61)).resize((EDGE, PANEL_H - 2 * CORNER), Image.Resampling.NEAREST)
    panel.paste(top, (CORNER, 0))
    panel.paste(bottom, (CORNER, PANEL_H - EDGE))
    panel.paste(left, (0, CORNER))
    panel.paste(right, (PANEL_W - EDGE, CORNER))
    tl = src.crop((0, 0, CORNER, CORNER))
    tr = src.crop((sw - CORNER, 0, sw, CORNER))
    bl = src.crop((0, sh - CORNER, CORNER, sh))
    br = bl.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
    panel.paste(tl, (0, 0))
    panel.paste(tr, (PANEL_W - CORNER, 0))
    panel.paste(bl, (0, PANEL_H - CORNER))
    panel.paste(br, (PANEL_W - CORNER, PANEL_H - CORNER))
    draw_text(panel, TITLE, small_glyphs, small_w, PANEL_W // 2, 14)
    write_bam(os.path.join(out_dir, "onlinebox.bam"), panel, fps=15)


# Blends two pill columns across a span so the plate's own shading stretches with it
def blend_columns(src, x0, x1, width):
    out = Image.new("RGBA", (width, src.size[1]))
    a = [src.getpixel((x0, y)) for y in range(src.size[1])]
    b = [src.getpixel((x1, y)) for y in range(src.size[1])]
    for x in range(width):
        t = x / max(width - 1, 1)
        for y in range(src.size[1]):
            out.putpixel((x, y), tuple(round(a[y][c] * (1 - t) + b[y][c] * t) for c in range(4)))
    return out


# Widens the no pill plate to the Go Online width from its own clean columns and stamps a white label
def make_pill(out_dir, data_dir, label, glyphs, width, height):
    pill_glyphs = {ch: scale_rgba(whiten(glyphs[ch]), FONT_SCALE_PILL) for ch in label.upper()}
    cell_w = round(width * FONT_SCALE_PILL)
    cell_h = round(height * FONT_SCALE_PILL)
    for variant in ["cold", "hot"]:
        src = bam_image(read_bam(os.path.join(data_dir, f"deletebox_no_{variant}.bam")))
        sw, sh = src.size
        left_w, mid_col, right_from = 28, 41, 58
        right_w = sw - right_from
        span = PILL_W - left_w - right_w
        first, second = span // 2, span - span // 2
        plate = Image.new("RGBA", (PILL_W, sh), (0, 0, 0, 0))
        plate.paste(src.crop((0, 0, left_w, sh)), (0, 0))
        plate.paste(blend_columns(src, left_w, mid_col, first), (left_w, 0))
        plate.paste(blend_columns(src, mid_col, right_from, second), (left_w + first, 0))
        plate.paste(src.crop((right_from, 0, sw, sh)), (left_w + span, 0))
        opaque_right = max(x for x in range(PILL_W) if plate.getpixel((x, sh // 2))[3])
        draw_text(plate, label.upper(), pill_glyphs, cell_w, (opaque_right + 1) // 2, (sh - cell_h) // 2)
        write_bam(os.path.join(out_dir, f"online_{label}_{variant}.bam"), plate, fps=15)


# Recolours the blue Cap'n hat cursor red so the helper's hand stands out
def make_helper_cursor(out_dir, data_dir):
    bam = read_bam(os.path.join(data_dir, "mouse.bam"))
    frames = []
    for i, f in enumerate(bam["frames"]):
        img = bam_image(bam, i)
        img.putdata([(b, g, r, a) if (b > r + 40 and b > g + 40) else (r, g, b, a) for r, g, b, a in pixels(img)])
        frames.append((img, (f["x"], f["y"]), (f["hx"], f["hy"])))
    write_bam(os.path.join(out_dir, "mouse_online.bam"), frames, fps=bam["fps"], bbox=bam["bbox"], loop_mode=bam["loop_mode"])


def wam_string(s):
    b = s.encode()
    return struct.pack("<I", len(b)) + b


def wam_record(name, x, y, layer, anims=(), text=None, font=None, align=0):
    out = struct.pack("<i", 2)
    out += name.encode().ljust(16, b"\0")[:16]
    for slot in range(4):
        if slot < len(anims) and anims[slot]:
            out += b"\x01" + wam_string(anims[slot]) + struct.pack("<hhf", 3, 15, 0.0)
        else:
            out += b"\x00"
    kind = 1 if text is not None else 0
    out += struct.pack("<iiiiiiiii", x, y, 0, 0, 0, 0, 32, layer, align)
    out += struct.pack("<III", 0, kind, 0)
    out += wam_string(text or "")
    out += struct.pack("<iiff", 0, 0, 0.0, 0.0)
    out += (b"\x01" + wam_string(font)) if font else b"\x00"
    out += b"\x00"
    return out


def make_scene(out_dir):
    pill_x = (640 - PILL_W) // 2
    records = [
        wam_record("box", PANEL_X, PANEL_Y, 0, ["onlinebox"]),
        wam_record("label", 320, 172, 0, text="", font="fontwhite", align=1),
        wam_record("code", 320, 196, 0, text="", font="font", align=1),
        wam_record("status", 320, 238, 0, text="", font="fontwhite", align=1),
        wam_record("m_host", pill_x, 166, 0, ["online_host_cold", "online_host_hot"]),
        wam_record("m_join", pill_x, 212, 0, ["online_join_cold", "online_join_hot"]),
        wam_record("m_back", pill_x, 258, 0, ["online_back_cold", "online_back_hot"]),
        wam_record("h_back", pill_x, 280, 0, ["online_back_cold", "online_back_hot"]),
        wam_record("j_join", 176, 280, 0, ["online_join_cold", "online_join_hot"]),
        wam_record("j_back", 336, 280, 0, ["online_back_cold", "online_back_hot"]),
        wam_record("s_leave", 176, 280, 0, ["online_leave_cold", "online_leave_hot"]),
        wam_record("s_back", 336, 280, 0, ["online_back_cold", "online_back_hot"]),
    ]
    out = struct.pack("<III", 1, WAM_MAGIC, len(records)) + b"".join(records)
    open(os.path.join(out_dir, "online.wam"), "wb").write(out)


def main():
    if len(sys.argv) != 3:
        print("usage: make-online-assets.py <CrunchData dir> <assets dir>")
        return 2
    data_dir, out_dir = sys.argv[1], sys.argv[2]
    os.makedirs(out_dir, exist_ok=True)
    chars, glyphs, width, height = load_glyphs(data_dir)
    small_glyphs, small_w = make_white_font(out_dir, chars, glyphs, width, height)
    make_panel(out_dir, data_dir, small_glyphs, small_w)
    for label in PILLS:
        make_pill(out_dir, data_dir, label, glyphs, width, height)
    make_helper_cursor(out_dir, data_dir)
    make_scene(out_dir)
    print(sorted(os.listdir(out_dir)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
