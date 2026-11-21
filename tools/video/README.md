# The video of the app

`make_video.py` films the app as it runs, with nobody at the mouse, and writes everything a YouTube upload needs:

| File in `dist/video/` | What |
| --- | --- |
| `fractique.mp4` | The video: 3840 × 2160 at 60 frames a second, with quiet music and no voice |
| `subtitles/<language>.srt` | The subtitles in the 15 languages of the app, with the same timing |
| `thumbnail.png` | The picture YouTube shows for the video, 1280 × 720 |
| `preview.png` | The same with a play button, for a README or a post that links to the video |
| `youtube.md` | For each of the 15 languages the title and the description to paste: the text, the chapters with their timecodes, the credit of the music and the hashtags |

Nothing is written on the video but the name: the texts are subtitles, so that every viewer reads them in their
own language.

## Making it

```sh
cargo build --release
python3 tools/video/make_video.py                 # the whole video, about four minutes
python3 tools/video/make_video.py --only zoom     # one scene alone, into dist/video/zoom.mp4
python3 tools/video/make_video.py --scale 1       # 1920 × 1080
python3 tools/video/make_video.py --music my.mp3  # another track; --no-music for none
python3 tools/video/make_video.py --keep          # films nothing: the scenes already filmed, put together anew
python3 tools/video/make_video.py --texts         # the subtitles and youtube.md anew, without filming
python3 tools/video/make_video.py --keep --readme # also docs/samples/video.webp, the picture the READMEs show
```

It needs `ffmpeg`, `weston`, `Xwayland` and Pillow (`sudo apt install ffmpeg weston xwayland python3-pil`), and a
graphics card with a Vulkan driver. Where `ffmpeg` has the encoder of an NVIDIA card (`h264_nvenc`), the camera and
the video use it: a processor does not keep up with 3840 × 2160 at 60 frames a second. `dist/video/work` keeps
the recordings of the scenes, several gigabytes; it can be deleted.

## How it films

The app is not filmed from the screen of the computer. For every scene the script starts a display of its own
that nobody sees (`weston` without a screen, with an X server in it), runs the app there on the graphics card and
records that display with `ffmpeg`. So the desktop is left alone, and the app runs at its own speed: a picture in
the video comes as fast as it does for a person, and the time the app shows under it is the real one.

The app is led by the steps of a test run (`FRACTIQUE_STEPS`, "The app" in `docs/architecture.md`). With
`FRACTIQUE_POINTER=arrow` or `finger` it draws the pointer those steps lead, since a display without a mouse has
none. The camera runs from the step `frames:N` for N frames of the app.

The phone scenes show the phone's layout (`FRACTIQUE_SHOW=phone`) drawn by the same code on the computer, inside
the outline of a phone. They are not filmed on a phone. `FRACTIQUE_WAY=4:2.5` makes the card answer late and
slower there, as a phone's chip does, so the pictures do not come faster than a phone draws them.

## The scenes: `film.json`

`parts` are shown in their order. A part with `steps` is a scene of the app; one with `seconds` is a still card
with the app's icon and name over a flame (`preset`), and `line` under them beside the mark of GitHub. The name is
set in Unbounded (SIL Open Font License), which the script downloads with the mark (`font`, `github`).

| Key of a scene | What |
| --- | --- |
| `size` | The window in points: `[1920, 1080]` for a computer, `[412, 892]` for a phone |
| `phone` | `true` puts the scene into the outline of a phone, over its own picture blurred |
| `pointer` | `arrow` (default) or `finger` |
| `open` | A scene file the app starts with, from the root of the repository: a layered picture of `docs/gallery` |
| `env` | Variables of the test run, such as `FRACTIQUE_SHOW` |
| `steps` | What happens. Steps before `frames:N` prepare the scene and wait for its picture; from `frames:N` on one step is taken a frame |
| `caption` | The key of the scene's subtitle in `captions.json` |
| `chapter` | The key of a chapter that begins with this part. YouTube shows chapters when the first begins at 0:00, there are three or more and each is 10 s or longer |

The steps a scene uses most:

| Step | What |
| --- | --- |
| `frames:300` | Films the next 300 frames of the app, about 7.5 s |
| `skip:75` | Waits 75 frames |
| `click:0.439:0.956` | Leads the pointer to that share of the window's width and height and presses; a third number is the frames the way takes (30) |
| `move:0.6:0.5` | Leads the pointer there without a press |
| `drag:0.6:0.5:0.45:0.62:60` | Drags from a point to a point over 60 frames |
| `wheel:1.01:100` | Magnifies the view by that much a frame for 100 frames |
| `preset:golden-bloom` | Shows a preset |

A button is pressed by where it stands, as a share of the window. After a change to the layout of the app, film
one scene with `--only`, look where the pointer lands and correct the numbers.

## The texts: `captions.json`

`studio` holds the name of each language as the owner's YouTube Studio lists it, in Ukrainian: `youtube.md` shows
it beside the language's code, so the right row is picked there. `texts` holds one text for each key in each of the 15 languages: the subtitles of the scenes, the names of the
chapters, and `title` and `description` for YouTube. The hashtags are `hashtags` in `film.json`, the same for every
language; YouTube ignores all of them when there are more than 15. A new scene needs its key here in every language.

## The music

"Long Note Two" by Kevin MacLeod (incompetech.com), licensed under Creative Commons Attribution 4.0. The script
downloads it once, checks its SHA-256 and sets it to the loudness of a background (`loudness` in `film.json`, in
LUFS). The track begins with half a minute that is far quieter than the rest, so the video plays it from where
it is heard (`from` and `to`, in seconds); where that passage is shorter than the video it begins again with a
crossfade (`cross`). The license asks for the credit: it is at the end of every description
in `youtube.md`, and it must stay in the description of the video.

## Uploading

1. Upload `fractique.mp4` in YouTube Studio with the English title and description of `youtube.md`, and
   `thumbnail.png` as the thumbnail.
2. **Subtitles**, **Add language**, then **Add** under **Subtitles**, **Upload file**, **With timing**: the `.srt`
   of that language. Repeat for the 15 languages.
3. In the same row, **Add** under **Title and description**: the texts of that language from `youtube.md`. A viewer
   then sees the title in their own language.

## Showing the video elsewhere

- **Reddit:** a link post with the address of the video shows the thumbnail and plays the video in place.
- **GitHub:** a README cannot hold a player of YouTube. `README.md` and its 14 translations show
  `docs/samples/video.webp`, the preview with the play button, as a picture that links to the video; `--readme`
  writes it anew. The address of the video is in the READMEs themselves.
