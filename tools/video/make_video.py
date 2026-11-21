import argparse
import hashlib
import json
import math
import os
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
APP = ROOT / "target" / "release" / "fractique-gui"
CLI = ROOT / "target" / "release" / "fractique"
FONTS = ROOT / "crates" / "fractique-app" / "assets" / "fonts"
FILM = json.loads((HERE / "film.json").read_text(encoding="utf-8"))
CAPTIONS = json.loads((HERE / "captions.json").read_text(encoding="utf-8"))
SOCKET = "fractique-film"
SCREEN = ":93"
FPS = 60
FADE = 0.3
PHONE = 0.9
NVENC = "h264_nvenc" in subprocess.run(["ffmpeg", "-hide_banner", "-encoders"], capture_output=True, text=True).stdout
# The card's encoder where there is one: the processor does not keep up with 3840 x 2160 at 60 frames a second.
CAMERA = (
    ["-c:v", "h264_nvenc", "-preset", "p1", "-tune", "lossless", "-pix_fmt", "yuv420p"]
    if NVENC
    else ["-c:v", "libx264", "-preset", "ultrafast", "-qp", "0"]
)
ENCODE = ["-r", str(FPS)] + (
    ["-c:v", "h264_nvenc", "-preset", "p6", "-tune", "hq", "-rc", "vbr", "-cq", "17", "-b:v", "0", "-pix_fmt", "yuv420p"]
    if NVENC
    else ["-c:v", "libx264", "-preset", "medium", "-crf", "15", "-pix_fmt", "yuv420p"]
)
DECODE = ["-hwaccel", "cuda"] if NVENC else []


def run(command, **more):
    return subprocess.run([str(part) for part in command], check=True, **more)


def ffmpeg(*parts):
    run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", *parts])


class Display:
    def __init__(self, width, height):
        self.size = (width, height)
        self.programs = []

    def __enter__(self):
        runtime = Path(os.environ["XDG_RUNTIME_DIR"])
        socket = runtime / SOCKET
        for stale in (socket, runtime / (SOCKET + ".lock")):
            stale.unlink(missing_ok=True)
        quiet = {"stdout": subprocess.DEVNULL, "stderr": subprocess.DEVNULL}
        size = f"{self.size[0]}x{self.size[1]}"
        weston = [
            "weston", "--backend=headless", "--renderer=gl", f"--width={self.size[0]}", f"--height={self.size[1]}",
            f"--socket={SOCKET}", "--idle-time=0", "--no-config", "--shell=kiosk",
        ]  # fmt: skip
        self.programs.append(subprocess.Popen(weston, **quiet))
        self.wait(socket, "weston did not start: is it installed, and does it have a GL renderer?")
        server = ["Xwayland", SCREEN, "-geometry", size, "-fullscreen", "-noreset", "-nolisten", "tcp"]
        self.programs.append(subprocess.Popen(server, env={**os.environ, "WAYLAND_DISPLAY": SOCKET}, **quiet))
        self.wait(Path("/tmp/.X11-unix") / f"X{SCREEN[1:]}", "Xwayland did not start")
        return self

    def wait(self, path, failure):
        for _ in range(100):
            if path.exists():
                time.sleep(0.3)
                return
            time.sleep(0.05)
        self.__exit__()
        raise SystemExit(failure)

    def __exit__(self, *_):
        for program in reversed(self.programs):
            program.terminate()
            program.wait()


def seconds(video):
    asked = ["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(video)]
    return float(subprocess.run(asked, check=True, capture_output=True, text=True).stdout)


def marks(trace):
    found = {}
    if trace.exists():
        for line in trace.read_text(encoding="utf-8", errors="replace").splitlines():
            words = line.split()
            if len(words) == 3 and words[0] == "start":
                found[words[1]] = float(words[2]) / 1000.0
    return found


def record(scene, out, scale):
    width, height = (side * scale for side in scene["size"])
    trace = out.with_suffix(".trace")
    trace.unlink(missing_ok=True)
    env = {name: value for name, value in os.environ.items() if name != "WAYLAND_DISPLAY"}
    with tempfile.TemporaryDirectory() as home, Display(width, height):
        env.update(
            DISPLAY=SCREEN,
            XDG_DATA_HOME=home,
            TMPDIR=home,
            FRACTIQUE_MANY="1",
            FRACTIQUE_NO_UPDATE_CHECK="1",
            FRACTIQUE_TILING="off",
            FRACTIQUE_VISIBLE="1",
            FRACTIQUE_SCALE=str(scale),
            FRACTIQUE_TRACE=str(trace),
            FRACTIQUE_POINTER=scene.get("pointer", "arrow"),
            FRACTIQUE_STEPS=",".join(scene["steps"]),
            **scene.get("env", {}),
        )
        still = out.with_suffix(".end.png")
        command = [str(APP), "--size", f"{width}x{height}", "--screenshot", str(still)]
        if scene.get("open"):
            command.append(str(ROOT / scene["open"]))
        app = subprocess.Popen(command, env=env, stderr=subprocess.DEVNULL)
        grab = None
        ends = time.monotonic() + 180
        # The camera runs from the step frames:N to the last of those frames; what the app does after is not filmed.
        while app.poll() is None and time.monotonic() < ends:
            seen = marks(trace)
            if grab is None and "recording" in seen:
                grab = subprocess.Popen(
                    [
                        "ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-f", "x11grab", "-draw_mouse", "0",
                        "-framerate", str(FPS), "-video_size", f"{width}x{height}", "-i", SCREEN,
                        *CAMERA, str(out),
                    ],
                    stdin=subprocess.PIPE,
                )  # fmt: skip
            if "recorded" in seen:
                break
            time.sleep(0.01)
        if grab is None:
            app.kill()
            raise SystemExit(f"{scene['name']}: the app never came to its step frames:N")
        grab.communicate(b"q")
        if app.poll() is None:
            app.terminate()
        app.wait()
    return seconds(out)


def clip_of(scene, raw, length, out):
    width, height = FILM["size"]
    fades = f"fade=t=in:st=0:d={FADE},fade=t=out:st={length - FADE:.3f}:d={FADE}"
    if not scene.get("phone"):
        graph = f"[0:v]scale={width}:{height}:flags=lanczos,{fades},format=yuv420p[v]"
        inputs = [*DECODE, "-i", raw]
    else:
        first = out.with_name(out.stem + "-first.png")
        ffmpeg("-i", raw, "-frames:v", "1", first)
        backdrop, mask, box = phone_stage(first, out.with_name(out.stem + "-stage"))
        graph = (
            f"[0:v]scale={box[2]}:{box[3]}:flags=lanczos[phone];"
            f"[1:v][phone]overlay={box[0]}:{box[1]}[staged];"
            f"[staged][2:v]overlay=0:0,{fades},format=yuv420p[v]"
        )
        inputs = [*DECODE, "-i", raw, "-loop", "1", "-i", backdrop, "-loop", "1", "-i", mask]
    ffmpeg(*inputs, "-filter_complex", graph, "-map", "[v]", "-t", f"{length:.3f}", *ENCODE, out)


def phone_stage(first, stem):
    width, height = FILM["size"]
    frame = Image.open(first).convert("RGB")
    tall = int(height * PHONE)
    wide = round(tall * frame.width / frame.height / 2) * 2
    left, top = (width - wide) // 2, (height - tall) // 2
    backdrop = frame.resize((width, round(width * frame.height / frame.width)), Image.LANCZOS)
    backdrop = backdrop.crop((0, (backdrop.height - height) // 2, width, (backdrop.height - height) // 2 + height))
    backdrop = backdrop.filter(ImageFilter.GaussianBlur(60)).point(lambda value: value * 0.45)
    radius, rim = round(wide * 0.11), max(round(wide * 0.022), 4)
    mask = backdrop.convert("RGBA")
    hole = Image.new("L", (width, height), 255)
    draw = ImageDraw.Draw(hole)
    draw.rounded_rectangle((left, top, left + wide, top + tall), radius, fill=0)
    mask.putalpha(hole)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle(
        (left - rim, top - rim, left + wide + rim, top + tall + rim),
        radius + rim,
        outline=(20, 22, 28, 255),
        width=rim,
    )
    draw.rounded_rectangle(
        (left - rim - 1, top - rim - 1, left + wide + rim + 1, top + tall + rim + 1),
        radius + rim + 1,
        outline=(86, 92, 104, 255),
        width=2,
    )
    paths = (stem.with_name(stem.name + "-backdrop.png"), stem.with_name(stem.name + "-mask.png"))
    backdrop.save(paths[0])
    mask.save(paths[1])
    return paths[0], paths[1], (left, top, wide, tall)


def fetched(spec, work):
    path = work / spec["file"]
    if not path.exists():
        urllib.request.urlretrieve(spec["url"], path)
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if digest != spec["sha256"]:
        raise SystemExit(f"{path}: sha256 is {digest}, film.json says {spec['sha256']}")
    return path


def font(work, weight, size):
    face = ImageFont.truetype(str(fetched(FILM["font"], work)), size)
    face.set_variation_by_axes([weight])
    return face


def flame(preset, size, path):
    if not path.exists():
        run([CLI, "render", "--preset", preset, "--size", f"{size[0]}x{size[1]}", "-o", path], capture_output=True)
    return Image.open(path).convert("RGB")


def shaded(picture, draw_on):
    shade = Image.new("RGBA", picture.size, (0, 0, 0, 0))
    draw_on(ImageDraw.Draw(shade), (0, 0, 0, 170))
    picture.alpha_composite(shade.filter(ImageFilter.GaussianBlur(picture.height / 90)))
    draw_on(ImageDraw.Draw(picture), "white")


# The icon and the name side by side, with their middle at a share of the picture's height.
def name_plate(picture, work, unit, middle):
    width, height = picture.size
    face = font(work, 400, round(140 * unit))
    side, gap = round(190 * unit), round(52 * unit)
    wide = ImageDraw.Draw(picture).textlength("Fractique", font=face)
    left = (width - side - gap - wide) / 2
    icon = Image.open(ROOT / "assets" / "icon" / "icon-1024.png").convert("RGBA").resize((side, side), Image.LANCZOS)
    picture.alpha_composite(icon, (round(left), round(height * middle - side / 2)))
    at = (left + side + gap, height * middle)
    shaded(picture, lambda draw, ink: draw.text(at, "Fractique", font=face, fill=ink, anchor="lm"))


def github_mark(work, side):
    mark = Image.open(fetched(FILM["github"], work)).convert("L").resize((side, side), Image.LANCZOS)
    white = Image.new("RGBA", mark.size, (255, 255, 255, 0))
    white.putalpha(mark.point(lambda value: 255 - value))
    return white


def card(spec, out, work):
    width, height = FILM["size"]
    unit = height / 1080
    picture = flame(spec["preset"], (width, height), work / f"{spec['preset']}-{width}x{height}.png")
    picture = picture.point(lambda value: value * 0.62).convert("RGBA")
    name_plate(picture, work, unit, 0.44 if spec.get("line") else 0.5)
    if spec.get("line"):
        face = font(work, 300, round(40 * unit))
        mark = github_mark(work, round(58 * unit))
        wide = ImageDraw.Draw(picture).textlength(spec["line"], font=face)
        gap, pad, tall = round(22 * unit), round(40 * unit), round(104 * unit)
        whole = mark.width + gap + wide + 2 * pad
        left, top = (width - whole) / 2, height * 0.64 - tall / 2
        pill = Image.new("RGBA", picture.size, (0, 0, 0, 0))
        ImageDraw.Draw(pill).rounded_rectangle(
            (left, top, left + whole, top + tall),
            tall / 2,
            fill=(255, 255, 255, 34),
            outline=(255, 255, 255, 90),
            width=max(round(2 * unit), 1),
        )
        picture.alpha_composite(pill)
        picture.alpha_composite(mark, (round(left + pad), round(top + (tall - mark.height) / 2)))
        ImageDraw.Draw(picture).text(
            (left + pad + mark.width + gap, top + tall / 2), spec["line"], font=face, fill="white", anchor="lm"
        )
    still = work / f"{spec['name']}.png"
    picture.convert("RGB").save(still)
    length = spec["seconds"]
    fades = f"fade=t=in:st=0:d={FADE * 2},fade=t=out:st={length - FADE * 2:.3f}:d={FADE * 2}"
    ffmpeg("-loop", "1", "-i", still, "-vf", f"fps={FPS},{fades},format=yuv420p", "-t", str(length), *ENCODE, out)
    return length


def stamp(seconds):
    whole = int(seconds * 1000)
    return f"{whole // 3600000:02}:{whole // 60000 % 60:02}:{whole // 1000 % 60:02},{whole % 1000:03}"


def subtitles(spans, out):
    out.mkdir(parents=True, exist_ok=True)
    for code in CAPTIONS["languages"]:
        blocks = []
        for key, start, end in spans:
            text = CAPTIONS["texts"][key][code]
            blocks.append(f"{len(blocks) + 1}\n{stamp(start + FADE)} --> {stamp(end - FADE)}\n{text}\n")
        (out / f"{code}.srt").write_text("\n".join(blocks), encoding="utf-8")


def music(args, work):
    if args.music:
        return Path(args.music)
    return fetched(FILM["music"], work)


# The passage of the track between `from` and `to`, repeated with a crossfade for as long as the video is, at the
# loudness of a background. Without `from` the track plays from its start.
def sound(length):
    track = FILM["music"]
    level = f"loudnorm=I={track['loudness']}:TP=-3:LRA=9,afade=t=in:d=1,afade=t=out:st={length - 4:.3f}:d=4[music]"
    if "from" not in track:
        return f"[1:a]{level}"
    cross = track["cross"]
    times = math.ceil((length + cross) / (track["to"] - track["from"] - cross))
    names = [f"[p{index}]" for index in range(times)]
    graph = f"[1:a]atrim={track['from']}:{track['to']},asetpts=PTS-STARTPTS,asplit={times}{''.join(names)};"
    last = names[0]
    for index in range(1, times):
        graph += f"{last}{names[index]}acrossfade=d={cross}:c1=tri:c2=tri[x{index}];"
        last = f"[x{index}]"
    return graph + f"{last}{level}"


# A subtitle as a picture, for a cut that carries its texts in the frame: where a video plays without sound and
# without subtitles of its own, as in a feed.
def caption_strip(text, work, size, name):
    width, height = size
    unit = height / 1080
    face = ImageFont.truetype(str(FONTS / "Inter-Medium.ttf"), round(40 * unit))
    strip = Image.new("RGBA", size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(strip)
    wide = draw.textlength(text, font=face)
    pad, tall = round(34 * unit), round(84 * unit)
    left, top = (width - wide) / 2 - pad, height - round(214 * unit)
    draw.rounded_rectangle((left, top, left + wide + 2 * pad, top + tall), tall / 2, fill=(8, 10, 16, 190))
    draw.text((width / 2, top + tall / 2), text, font=face, fill="white", anchor="mm")
    path = work / f"short-{name}.png"
    strip.save(path)
    return path


# The short cut: parts of the scenes already filmed, each with its subtitle in the frame, then the last card.
def short(args, work):
    spec = FILM["short"]
    size = spec["size"]
    pieces, at = [], 0.0
    for index, part in enumerate(spec["parts"]):
        scene = next(scene for scene in FILM["parts"] if scene["name"] == part["name"])
        length = part["to"] - part["from"]
        fades = f"fade=t=in:st=0:d={FADE},fade=t=out:st={length - FADE:.3f}:d={FADE}"
        piece = work / f"short-{index:02}.mp4"
        inputs = [*DECODE, "-ss", str(part["from"]), "-t", f"{length:.3f}", "-i", work / f"{part['name']}.mp4"]
        graph = f"[0:v]fps={FPS},scale={size[0]}:{size[1]}:flags=lanczos,setpts=PTS-STARTPTS"
        if scene.get("caption"):
            text = CAPTIONS["texts"][scene["caption"]][args.short]
            inputs += ["-loop", "1", "-i", caption_strip(text, work, size, f"{index:02}")]
            graph += f"[scene];[scene][1:v]overlay=0:0:shortest=1,{fades},format=yuv420p[v]"
        else:
            graph += f",{fades},format=yuv420p[v]"
        ffmpeg(*inputs, "-filter_complex", graph, "-map", "[v]", "-t", f"{length:.3f}", *ENCODE, piece)
        pieces.append(piece)
        at += length
    listing = work / "short.txt"
    listing.write_text("".join(f"file '{piece}'\n" for piece in pieces), encoding="utf-8")
    silent = work / "short-silent.mp4"
    ffmpeg("-f", "concat", "-safe", "0", "-i", listing, "-c", "copy", silent)
    ffmpeg(
        "-i", silent, "-stream_loop", "-1", "-vn", "-i", music(args, work),
        "-filter_complex", sound(at), "-map", "0:v", "-map", "[music]", "-c:v", "copy", "-c:a", "aac",
        "-b:a", "160k", "-t", f"{at:.3f}", "-movflags", "+faststart", args.out / "short.mp4",
    )  # fmt: skip
    # GitHub plays a video dragged into a README where the file is under 10 MB.
    small = spec["small"]
    rate = int(small["megabytes"] * 8000 / at)
    ffmpeg(
        "-i", silent, "-an", "-vf", f"fps={small['fps']},scale={small['size'][0]}:{small['size'][1]}:flags=lanczos",
        "-c:v", "libx264", "-preset", "slow", "-b:v", f"{rate}k", "-maxrate", f"{rate}k", "-bufsize", f"{2 * rate}k",
        "-pix_fmt", "yuv420p", "-movflags", "+faststart", args.out / "short-small.mp4",
    )  # fmt: skip
    print(f"{args.out / 'short.mp4'}: {at:.1f} s")


def preview(frames, out, work):
    width, height = 1280, 720
    spec = FILM["preview"]
    picture = flame(spec["preset"], (width, height), work / f"{spec['preset']}-{width}x{height}.png")
    picture = picture.filter(ImageFilter.GaussianBlur(6)).point(lambda value: value * 0.55).convert("RGBA")

    def rounded(image, radius):
        shape = Image.new("L", image.size, 0)
        ImageDraw.Draw(shape).rounded_rectangle((0, 0, *image.size), radius, fill=255)
        image = image.convert("RGBA")
        image.putalpha(shape)
        return image

    def place(image, at, radius):
        shadow = Image.new("RGBA", picture.size, (0, 0, 0, 0))
        ImageDraw.Draw(shadow).rounded_rectangle(
            (at[0] - 6, at[1] + 10, at[0] + image.width + 6, at[1] + image.height + 22), radius, fill=(0, 0, 0, 150)
        )
        picture.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(18)))
        picture.alpha_composite(rounded(image, radius), at)
        ImageDraw.Draw(picture).rounded_rectangle(
            (at[0] - 1, at[1] - 1, at[0] + image.width, at[1] + image.height), radius, outline=(120, 128, 140, 255), width=2
        )

    desk = Image.open(frames[spec["desk"]]).convert("RGB")
    desk = desk.resize((820, round(820 * desk.height / desk.width)), Image.LANCZOS)
    place(desk, (70, 200), 18)
    phone = Image.open(frames[spec["phone"]]).convert("RGB")
    phone = phone.resize((round(500 * phone.width / phone.height), 500), Image.LANCZOS)
    place(phone, (width - phone.width - 110, 150), 30)
    icon = Image.open(ROOT / "assets" / "icon" / "icon-1024.png").convert("RGBA").resize((112, 112), Image.LANCZOS)
    picture.alpha_composite(icon, (70, 44))
    ImageDraw.Draw(picture).text((210, 100), "Fractique", font=font(work, 400, 92), fill="white", anchor="lm")
    picture.convert("RGB").save(out / "thumbnail.png")
    play = picture.copy()
    middle = (width // 2, height // 2)
    draw = ImageDraw.Draw(play)
    draw.ellipse((middle[0] - 64, middle[1] - 64, middle[0] + 64, middle[1] + 64), fill=(0, 0, 0, 170), outline="white", width=4)
    draw.polygon([(middle[0] - 20, middle[1] - 30), (middle[0] - 20, middle[1] + 30), (middle[0] + 34, middle[1])], fill="white")
    play.convert("RGB").save(out / "preview.png")
    return play.convert("RGB")


# For each language the title and, in a block to copy whole, the description: the text, the chapters with the
# second each begins at, the credit of the music and the hashtags.
def notes(out, length, chapters):
    track = FILM["music"]
    lines = [f"# Texts for YouTube\n\nThe video is {length:.0f} s long.\n"]
    for code in CAPTIONS["languages"]:
        marks = [f"{int(at) // 60}:{int(at) % 60:02} {CAPTIONS['texts'][key][code]}" for key, at in chapters]
        body = [CAPTIONS["texts"]["description"][code], "\n".join(marks), "Music: " + track["credit"], FILM["hashtags"]]
        lines += [
            f"## {code} ({CAPTIONS['studio'][code]})\n",
            f"**Title:** {CAPTIONS['texts']['title'][code]}\n",
            "```text\n" + "\n\n".join(body) + "\n```\n",
        ]
    (out / "youtube.md").write_text("\n".join(lines), encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description="Films the app and writes the video, its subtitles and its preview.")
    parser.add_argument("--out", type=Path, default=ROOT / "dist" / "video")
    parser.add_argument("--scale", type=int, default=2, help="pixels a point: 2 films 3840x2160, 1 films 1920x1080")
    parser.add_argument("--only", help="film this scene alone, to look at it")
    parser.add_argument("--music", help="a file to use instead of the track of film.json")
    parser.add_argument("--no-music", action="store_true")
    parser.add_argument("--keep", action="store_true", help="film nothing: put the scenes already filmed together anew")
    parser.add_argument("--short", metavar="LANGUAGE", help="only the short cut of the scenes already filmed, with the subtitles of this language in the frame")
    parser.add_argument("--readme", action="store_true", help="also write docs/samples/video.webp, which the READMEs show")
    parser.add_argument("--texts", action="store_true", help="write the subtitles and youtube.md anew from the scenes already filmed")
    args = parser.parse_args()
    for program in (APP, CLI):
        if not program.exists():
            raise SystemExit(f"{program} is missing: cargo build --release")
    for tool in ("ffmpeg", "weston", "Xwayland"):
        if not shutil.which(tool):
            raise SystemExit(f"{tool} is not installed")
    FILM["size"] = [side * args.scale for side in FILM["size"]]
    work = args.out / "work"
    work.mkdir(parents=True, exist_ok=True)
    if args.short:
        short(args, work)
        return
    clips, spans, chapters, frames, at = [], [], [], {}, 0.0
    for part in FILM["parts"]:
        if args.only and part["name"] != args.only:
            continue
        clip = work / f"{part['name']}.mp4"
        if args.texts or args.keep:
            length = seconds(clip)
            frames[part["name"]] = work / f"{part['name']}-last.png"
        elif "steps" in part:
            raw = work / f"{part['name']}-raw.mkv"
            # A phone is filmed with as many pixels a point as its outline in the video is high.
            tall = math.ceil(FILM["size"][1] * PHONE / part["size"][1])
            length = record(part, raw, tall if part.get("phone") else args.scale)
            clip_of(part, raw, length, clip)
            frames[part["name"]] = work / f"{part['name']}-last.png"
            ffmpeg("-sseof", "-0.2", "-i", raw, "-frames:v", "1", frames[part["name"]])
        else:
            length = card(part, clip, work)
        print(f"{part['name']}: {length:.1f} s")
        clips.append(clip)
        if part.get("caption"):
            spans.append((part["caption"], at, at + length))
        if part.get("chapter"):
            chapters.append((part["chapter"], at))
        at += length
    if args.texts:
        subtitles(spans, args.out / "subtitles")
        notes(args.out, at, chapters)
        return
    listing = work / "film.txt"
    listing.write_text("".join(f"file '{clip}'\n" for clip in clips), encoding="utf-8")
    silent = work / "silent.mp4"
    ffmpeg("-f", "concat", "-safe", "0", "-i", listing, "-c", "copy", silent)
    film = args.out / ("fractique.mp4" if not args.only else f"{args.only}.mp4")
    if args.no_music or args.only:
        shutil.copyfile(silent, film)
    else:
        ffmpeg(
            "-i", silent, "-stream_loop", "-1", "-vn", "-i", music(args, work),
            "-filter_complex", sound(at), "-map", "0:v", "-map", "[music]", "-c:v", "copy", "-c:a", "aac",
            "-b:a", "192k", "-t", f"{at:.3f}", "-movflags", "+faststart", film,
        )  # fmt: skip
    if not args.only:
        subtitles(spans, args.out / "subtitles")
        shown = preview(frames, args.out, work)
        if args.readme:
            shown.save(ROOT / "docs" / "samples" / "video.webp", quality=88, method=6)
        notes(args.out, at, chapters)
    print(f"{film}: {at:.1f} s")


if __name__ == "__main__":
    main()
