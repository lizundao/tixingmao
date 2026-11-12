#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""生成提醒猫（Tixingmao）桌面应用图标全套。透明底、暖橙猫头、双描边（浅环+深棕环）保证深浅背景都清晰。"""
import io
import os
from PIL import Image, ImageDraw, ImageFilter

OUT = r"E:\rustc\tools\tixingmao\src-tauri\icons"
WEB = r"E:\Node\inkdot\public\assets\img\tixingmao-icon.png"

ORANGE   = (233, 146, 58, 255)
ORANGE_L = (247, 184, 116, 255)
OUTLINE  = (66, 42, 26, 255)
HALO     = (255, 255, 255, 255)
PINK     = (244, 160, 152, 255)
PINK_D   = (219, 116, 108, 255)
WHITE    = (255, 255, 255, 255)
EYE      = (52, 36, 26, 255)
BG_TILE  = (255, 249, 240, 255)
S = 1024

def aa(draw_fn, scale=4):
    big = Image.new("RGBA", (S*scale, S*scale), (0,0,0,0))
    draw_fn(ImageDraw.Draw(big), scale)
    return big.resize((S, S), Image.LANCZOS)

def draw_cat(d, k):
    C = S*k//2
    # 耳朵（宽、根深植入脸部，之后被脸覆盖 → 自然连体）
    l_ear = [(250*k,118*k), (176*k,404*k), (486*k,330*k)]
    r_ear = [(538*k,330*k), (848*k,404*k), (774*k,118*k)]
    for ear in (l_ear, r_ear):
        d.polygon(ear, fill=ORANGE)
    # 脸
    fx0, fy0, fx1, fy1 = 200*k, 298*k, 824*k, 862*k
    d.ellipse([fx0, fy0, fx1, fy1], fill=ORANGE)
    d.ellipse([fx0+120*k, fy0+18*k, fx1-120*k, fy0+150*k], fill=ORANGE_L)  # 高光
    # 内耳粉
    l_in = [(276*k,196*k), (238*k,352*k), (430*k,302*k)]
    r_in = [(594*k,302*k), (786*k,352*k), (748*k,196*k)]
    for ein in (l_in, r_in):
        d.polygon(ein, fill=PINK)
    # 腮红
    d.ellipse([290*k,632*k,392*k,718*k], fill=PINK)
    d.ellipse([632*k,632*k,734*k,718*k], fill=PINK)
    # 眼睛
    for ex in (396*k, 628*k):
        ey = 552*k
        d.ellipse([ex-74*k, ey-90*k, ex+74*k, ey+90*k], fill=WHITE)
        d.ellipse([ex-44*k, ey-60*k, ex+44*k, ey+68*k], fill=EYE)
        d.ellipse([ex-22*k, ey-38*k, ex+16*k, ey-2*k], fill=WHITE)
    # 鼻
    d.polygon([(C-30*k, 660*k), (C+30*k, 660*k), (C, 704*k)], fill=PINK_D)
    # 嘴：清晰加粗 ω
    lw = int(18*k)
    mw = 70*k
    d.line([(C, 704*k), (C, 726*k)], fill=OUTLINE, width=lw)
    d.arc([C-mw-16*k, 726*k, C-16*k, 820*k], 18, 162, fill=OUTLINE, width=lw)
    d.arc([C+16*k, 726*k, C+mw+16*k, 820*k], 18, 162, fill=OUTLINE, width=lw)

def add_outline_double(layer, Rdark=15, Rlight=28):
    a = layer.split()[3]
    def dilate(R):
        x = a
        for _ in range(3): x = x.filter(ImageFilter.MaxFilter(2*R+1))
        return x.filter(ImageFilter.GaussianBlur(max(1, R//3)))
    light = Image.new("RGBA", layer.size, HALO); light.putalpha(dilate(Rlight))
    dark  = Image.new("RGBA", layer.size, OUTLINE); dark.putalpha(dilate(Rdark))
    res = Image.alpha_composite(light, dark)
    res = Image.alpha_composite(res, layer)
    return res

def build_master():
    layer = aa(draw_cat, scale=4)
    return add_outline_double(layer)

def make_tile(bg):
    base = Image.new("RGBA", (S, S), bg)
    cat = master.resize((int(S*0.8), int(S*0.8)), Image.LANCZOS)
    base.alpha_composite(cat, ((S-cat.width)//2, (S-cat.height)//2 + 18))
    return base.convert("RGB")

def save_ico(path, sizes):
    # 必须以高分辨率原图为源，再列出目标尺寸由 Pillow 降采样生成多帧
    master.save(path, sizes=[(s, s) for s in sizes]); print("OK", os.path.basename(path))

def build_icns(path):
    entries = {b"ic07":128, b"ic08":256, b"ic09":512, b"ic10":1024}
    body = bytearray()
    for typ, sz in entries.items():
        buf = io.BytesIO(); master.resize((sz,sz), Image.LANCZOS).save(buf, format="PNG"); png = buf.getvalue()
        body += typ + len(png).to_bytes(4,"big") + png  # length = PNG 数据长度（不含 8 字节元素头）
    total = 8 + len(body)
    data = bytearray(b"icns") + total.to_bytes(4,"big") + body
    with open(path,"wb") as f: f.write(data)
    print("OK", os.path.basename(path))

def preview():
    pad=40
    light = Image.new("RGBA",(S,S),(240,242,245,255)); light.alpha_composite(master,(0,0))
    dark  = Image.new("RGBA",(S,S),(32,33,36,255));   dark.alpha_composite(master,(0,0))
    cells=[(master,"透明原图"),(light,"浅底(桌面)"),(dark,"深底(托盘)")]
    n=len(cells); cell=S+pad*2
    sheet=Image.new("RGBA",(cell*n, cell+170),(255,255,255,255)); dd=ImageDraw.Draw(sheet)
    try:
        from PIL import ImageFont
        font=ImageFont.truetype("arial.ttf",38)
    except Exception:
        font=None
    for i,(im,label) in enumerate(cells):
        x=i*cell+pad; sheet.alpha_composite(im,(x,pad))
        if font: dd.text((x+10,S+pad+18),label,fill=(0,0,0),font=font)
        sm=im.resize((112,112),Image.LANCZOS); sheet.alpha_composite(sm,(x+(S-112)//2,pad+S+66))
    sheet.convert("RGB").save(r"E:\rustc\tools\tixingmao\artifacts\icon-preview.png"); print("OK preview")

if __name__=="__main__":
    os.makedirs(OUT, exist_ok=True); os.makedirs(os.path.dirname(WEB), exist_ok=True)
    master = build_master(); preview()
    master.save(os.path.join(OUT,"icon.png")); print("OK icon.png")
    for sz in (32,64,128):
        master.resize((sz,sz),Image.LANCZOS).save(os.path.join(OUT,f"{sz}x{sz}.png")); print(f"OK {sz}x{sz}.png")
    master.resize((256,256),Image.LANCZOS).save(os.path.join(OUT,"128x128@2x.png")); print("OK 128x128@2x.png")
    save_ico(os.path.join(OUT,"icon.ico"),[16,24,32,48,64,128,256])
    build_icns(os.path.join(OUT,"icon.icns"))
    for name,sz in {"Square30x30Logo":30,"Square44x44Logo":44,"Square71x71Logo":71,
        "Square89x89Logo":89,"Square107x107Logo":107,"Square142x142Logo":142,
        "Square150x150Logo":150,"Square284x284Logo":284,"Square310x310Logo":310,"StoreLogo":50}.items():
        make_tile(BG_TILE).resize((sz,sz),Image.LANCZOS).save(os.path.join(OUT,f"{name}.png")); print(f"OK {name}.png")
    master.save(WEB); print("OK", WEB)
