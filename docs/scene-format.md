# Scene format

A scene is one JSON document. It holds everything a render depends on, so the same scene always renders to the
same image (see [determinism.md](determinism.md)). This file is the contract: it changes together with
`crates/fractique-core/src/scene.rs` and `variation.rs`.

Unknown fields are refused, so a typo in a field name is an error and not a silently ignored setting. Fields with
a default may be left out. Colors are sRGB with each channel from 0 to 1.

A scene is one or more [layers](#layer), each a [flame](#flame) or a [fractal](#fractal) of another kind, a
wallpaper ([sheets](#sheets)), or both. The app saves scenes in a packed file of its own, [`.frcq`](#the-frcq-file), which every command reads as well.

## Example

The `azure-rings` preset:

```json
{
  "version": 1,
  "name": "azure-rings",
  "seed": 41,
  "background": [
    0.012,
    0.014,
    0.045
  ],
  "layers": [
    {
      "name": "base",
      "blend": "add",
      "flame": {
        "camera": {
          "center": [
            0.001447,
            0.006064
          ],
          "zoom": 0.780277
        },
        "tone": {
          "brightness": 0.979397,
          "gamma": 2.584139,
          "vibrancy": 1,
          "highlight": 0.039007,
          "glow_radius": 0.006,
          "glow_strength": 0.5
        },
        "palette": {
          "stops": [
            {
              "pos": 0,
              "color": [
                0.105882,
                0.231373,
                0.435294
              ]
            },
            {
              "pos": 0.2,
              "color": [
                0.184314,
                0.498039,
                0.796078
              ]
            },
            {
              "pos": 0.4,
              "color": [
                0.498039,
                0.831373,
                1
              ]
            },
            {
              "pos": 0.6,
              "color": [
                0.909804,
                0.984314,
                1
              ]
            },
            {
              "pos": 0.8,
              "color": [
                0.603922,
                0.815686,
                0.960784
              ]
            },
            {
              "pos": 1,
              "color": [
                0.227451,
                0.384314,
                0.721569
              ]
            }
          ]
        },
        "symmetry": 4,
        "transforms": [
          {
            "weight": 0.706819,
            "color": 0,
            "color_speed": 0.516711,
            "affine": [
              0.237859,
              0.37139,
              0.352343,
              -0.262065,
              0.153908,
              -0.636054
            ],
            "variations": [
              {
                "type": "spherical",
                "weight": 0.918028
              },
              {
                "type": "swirl",
                "weight": 0.186141
              }
            ]
          },
          {
            "weight": 0.612897,
            "color": 1,
            "color_speed": 0.410144,
            "affine": [
              0.512268,
              -0.273912,
              -0.751519,
              0.279638,
              0.754045,
              0.486691
            ],
            "variations": [
              {
                "type": "linear",
                "weight": 0.62817
              },
              {
                "type": "spherical",
                "weight": 0.311493
              }
            ]
          }
        ]
      }
    }
  ]
}
```

## Scene

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `version` | integer | required | The format version. This build reads `1`. |
| `name` | string | empty | A label. Not used by the renderer. |
| `seed` | integer, 0 to 2⁶⁴−1 | required | Seeds every random choice of the render. A different seed gives a different grain of the same picture. Above 2⁵³ a JavaScript client loses digits: keep seeds below it when scenes pass through one. |
| `background` | color | `[0, 0, 0]` | The color under all layers. |
| `sheets` | sheets | none | A stack of flat shapes drawn over the background and under all layers: a wallpaper. See [Sheets](#sheets). |
| `layers` | list of layers | empty | Up to 16, blended in order: the first is at the bottom. A scene needs at least one layer, or `sheets`. |

## Layer

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `name` | string | empty | A label. |
| `blend` | `add`, `screen`, `over` | `add` | How the layer meets what is under it. `add` sums light (bright overlaps burn out), `screen` sums light without burning out, `over` covers what is under it where the layer is dense. |
| `opacity` | 0 to 1 | 1 | Strength of the layer. |
| `flame` | flame | one of the two | What the layer shows: a [flame](#flame). |
| `fractal` | fractal | one of the two | What the layer shows: a [fractal](#fractal) of another kind. A layer holds `flame` or `fractal`, never both and never neither. |

## Flame

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `camera` | camera | centered, zoom 1 | The part of the world the image shows. |
| `tone` | tone | see below | How density becomes light. |
| `palette` | palette | required | The colors the transforms pick from. |
| `symmetry` | integer, 1 to 64 or −1 to −64 | 1 | Rotational copies around the origin. `4` shows the flame four times, a quarter turn apart. A negative number also mirrors it. |
| `transforms` | list of transforms | required | 1 to 64. The flame is the shape these functions map onto itself. |
| `final_transform` | transform | none | Applied to every point before it is drawn, without feeding back into the flame. Its `weight` is not used. |

### Camera

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `center` | `[x, y]` | `[0, 0]` | The world point at the middle of the image. |
| `zoom` | above 0 | 1 | At 1 the shorter side of the image shows 4 world units, at any resolution. |
| `rotation` | degrees | 0 | Counter-clockwise. |
| `flip_y` | `true`, `false` | `false` | Draws the world's y axis downwards, as flam3 and Apophysis do. Set by the `.flame` import. |

### Tone

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `brightness` | 0 or more | 1 | Scales the light. |
| `gamma` | above 0 | 2.5 | Higher lifts the faint parts. |
| `vibrancy` | 0 to 1 | 1 | At 1 dim areas keep the palette's hue; at 0 gamma is applied to each channel, which washes colors out. |
| `highlight` | 0 or more | 0 | How fast the densest areas burn out to white. |
| `glow_radius` | 0 to 0.25 | 0 | Radius of the glow as a share of the image's shorter side. |
| `glow_strength` | 0 or more | 0 | How much of the blurred layer is added back. |
| `smoothing` | 0 to 4 | 1 | Density estimation. 0 leaves every hit in its own cell (grainy at low quality); 1 is the standard amount. |

### Palette

`stops` is a list of 1 to 256 `{ "pos": 0 to 1, "color": [r, g, b] }` entries in ascending `pos` order. Colors
between two stops are interpolated.

## Fractal

A layer with `fractal` in place of `flame` shows a fractal of another kind: the Mandelbrot set, a Julia set, a
Newton fractal, a strange attractor. It blends with the layers around it as a flame does.

```json
{
  "version": 1,
  "seed": 1,
  "layers": [
    {
      "fractal": {
        "kind": "julia",
        "camera": { "zoom": 1.5 },
        "palette": { "stops": [{ "pos": 0, "color": [0.25, 0.13, 0.54] }, { "pos": 1, "color": [1, 0.82, 0.5] }] },
        "params": { "real": -0.8, "imaginary": 0.156, "iterations": 400 }
      }
    }
  ]
}
```

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `kind` | a name | required | Which fractal: one of the kinds below. `fractique fractals` lists those of the build you run. |
| `camera` | [camera](#camera) | centered, zoom 1 | The part of the plane the image shows, as for a flame. |
| `tone` | [tone](#tone) | see below | How the layer is lit. A kind uses the fields its way of drawing has a use for. |
| `palette` | [palette](#palette) | required | The colors of the layer. |
| `params` | object of numbers | empty | The parameters of the kind, by name. One that is left out has its default; an unknown name or a value outside the parameter's range is refused. |

A kind is drawn in one of two ways:

| Way | What happens | Tone fields used |
| --- | --- | --- |
| Per pixel | Every pixel asks the kind for a place on the palette and a light from 0 to 1, or for nothing: such a point stays clear and shows the background and the layers under it. The place runs along the palette and back: 0 is the first stop, 1 the last, 2 the first again. The layer's color is the palette's color times the light, and where a layer is blended `over` it covers what is under it by its light. | `brightness` scales the layer, `gamma` raises the light to the power 2.5 / gamma (2.5 leaves it), `glow_radius` and `glow_strength` add the glow. `vibrancy`, `highlight` and `smoothing` are not used. |
| Density | The kind plots the points of orbits into the histogram a flame uses, and the layer is developed like a flame: density estimation, log tone mapping, glow. The scene's `seed` picks the starting points. | All of them, as for a flame. |

A per-pixel kind takes one sample at the middle of every pixel. Where a pixel differs from one of its eight
neighbours by 0.02 or more in a channel, it is drawn again from a grid of samples and averaged:

| Samples per pixel of the render | Grid, times `supersample`, at most 8 × 8 |
| --- | --- |
| under 16 | 1 × 1 |
| 16 to 63 | 2 × 2 |
| 64 to 599 | 3 × 3 |
| 600 and more | 4 × 4 |

The kinds, by family:

| Family | Drawn | Kinds |
| --- | --- | --- |
| Escape-time | per pixel | `burning_ship`, `julia`, `magnet`, `mandelbrot`, `multibrot`, `phoenix`, `tricorn` |
| Root-finding | per pixel | `newton`, `nova` |
| Orbit | density | `buddhabrot`, `clifford`, `de_jong`, `gumowski_mira`, `hopalong`, `lorenz` |
| Geometric | either | |
| Solid | per pixel | `mandelbox`, `mandelbulb`, `menger_sponge` |

### Escape-time kinds

A point is iterated until it leaves the circle |z| = 256 or the steps run out. The steps allowed are
`iterations`, times 1 + log₁₀(zoom) / 2 where the camera's zoom is above 1: a deeper view gets more. A point that
never leaves, or whose orbit comes back to a value it had, belongs to the set and stays clear.

A point that leaves after n steps at z has the smooth count ν = n + 1 − log₂ ln |z| and the distance to the set
d = |z| ln |z| / |z′|, measured in shorter sides of the picture (z′ is the derivative of z by the point for
`mandelbrot`, by the starting point for `julia`). Its place on the palette is
`color_shift` + `color_density` × ln(1 + ν). Its light is 1, darkened twice: by (d / r)^0.4 where d is under
r = 0.012 × `edge`², which draws the thin dark rim along the set, and by 1 / (1 + (d / R)^1.2) with
R = 0.02 / `fade`², which lets the colors fade to dark away from the set. `edge` 0 and `fade` 0 switch each off.

| Parameter | Range | Default | Meaning |
| --- | --- | --- | --- |
| `iterations` | 16 to 100 000, whole | 300 | The steps a point is given at zoom 1. |
| `color_density` | 0.01 to 100 | 1 | How fast the colors run along the palette. |
| `color_shift` | 0 to 2 | 0 | Where on the palette the colors start. |
| `edge` | 0 to 1 | 0.35 | The width of the dark rim along the set. |
| `fade` | 0 to 1 | 0.6 | How soon the colors fade to dark away from the set. |

| Kind | Iteration | Own parameters |
| --- | --- | --- |
| `mandelbrot` | z → z² + c from z = 0, with c the point | none |
| `julia` | z → z² + c from z the point, with c = `real` + i × `imaginary` | `real` (−2 to 2, default −0.8), `imaginary` (−2 to 2, default 0.156) |
| `burning_ship` | z → (\|Re z\| + i \|Im z\|)² + c from z = 0. The picture shows the imaginary axis pointing down, so that the ship stands mast up: the point x + i y is c = x − i y | none |
| `tricorn` | z → z̄² + c from z = 0, with z̄ the conjugate of z and c the point | none |
| `multibrot` | z → z^d + c from z = 0, with c the point and d = `power` | `power` (2 to 8, default 3) |
| `phoenix` | z → z² + c + p z₋₁, with z₋₁ the value one step before (0 at the start) and p = `feedback`. `form` 1 is the Julia form: z starts at the point and c = `real` + i × `imaginary`. `form` 0 is the map of the constants: c is the point and z starts at 0. The plane is turned a quarter, as the phoenix is always shown: the point x + i y stands for y − i x | `form` (0 or 1, default 1), `real` (−2 to 2, default 0.56667), `imaginary` (−2 to 2, default 0), `feedback` (−1.5 to 1.5, default −0.5) |
| `magnet` | `type` 1: z → ((z² + c − 1) / (2z + c − 2))². `type` 2: z → ((z³ + 3(c − 1)z + (c − 1)(c − 2)) / (3z² + 3(c − 2)z + (c − 1)(c − 2) + 1))². Both from z = 0, with c the point | `type` (1 or 2, default 1) |

The default tone of all of them is `brightness` 0.92, `glow_radius` 0.006 and `glow_strength` 0.12.

Where a kind differs from the two above:

- **`burning_ship`, `tricorn`:** the step is not an analytic function, so z has no derivative by the point. The
  distance is d = |z| ln |z| / |∇|z||, with ∇|z| the gradient of |z| by the two coordinates of the point; for an
  analytic step it is the same number as above.
- **`multibrot`:** the smooth count is ν = n + 1 − log_d ln |z|. A whole `power` is computed by multiplying. Any
  other is z^d = |z|^d (cos(d θ) + i sin(d θ)) with θ the angle of z from −π to π: the picture then has seams,
  thin straight cuts through the colors where an orbit crosses the negative real axis. They belong to such a power.
- **`phoenix`:** the derivative follows the step: z′ → 2 z z′ + p z′₋₁, plus 1 in `form` 0. With `feedback` 0 the
  two forms are the Mandelbrot set and the Julia sets, turned. With another `feedback` the map of `form` 0 is a
  guide to the constants, not the exact set of the connected Julia forms: the step then has no critical point.
- **`magnet`:** an orbit ends in one of two ways. It leaves the circle |z| = 256: far out the step is z² / 4
  (`type` 2: z² / 9), so the count and the distance take |z| / 4 (|z| / 9) in place of |z|. Or it comes within
  10⁻⁴ of 1, the point both maps hold still: next to 1 the step is 1 + 2(z − 1)² / c (`type` 2:
  1 + 2(z − 1)³ / c²), so with e = 2|z − 1| / |c| (`type` 2: √2 |z − 1| / |c|) the smooth count is
  ν = n + 1 − log₂(−ln e) (`type` 2: log₃) and the distance to the edge of that region is
  |z − 1| (−ln e) / |z′|. Both endings are colored, darkened at the rim and faded in the same way; a point whose
  orbit does neither belongs to the set and stays clear. Far from the set the orbits settle on 1.

`form`, `type` and the `polynomial` of `newton` are whole numbers that stand for a choice. The app shows the names
of the choices in place of a slider.

### Root-finding kinds

A point is moved by Newton's method, relaxed: z → z − a p(z) / p′(z), until it has arrived or the steps run out.
The steps allowed are `iterations`, times 1 + log₁₀(zoom) / 2 where the camera's zoom is above 1. A point that
never arrives stays clear, and so does one where p′ is 0.

The smooth count ν of a point that arrives at step n is n − 1 plus the part of the last step it needed: with d₋₁
the distance before the step, d the distance after it and t the distance that counts as arrived, the part is
ln(ln t / ln d₋₁) / ln(ln d / ln d₋₁) where the distance is squared at every step (a = 1), and
(ln t − ln d₋₁) / (ln d − ln d₋₁) where it shrinks by a factor (any other a). The light of a point is
1 / (1 + (`shade` × ν / h)²): the more steps a point took, the darker it is.

| Parameter | Range | Default | Meaning |
| --- | --- | --- | --- |
| `iterations` | 8 to 10 000, whole | 200 | The steps a point is given at zoom 1. |
| `color_density` | 0.01 to 100 | 1 | How far the count moves the color along the palette. |
| `color_shift` | 0 to 2 | 0 | Where on the palette the colors start. |
| `relaxation` | 0.1 to 1.9 | 1 | The real part of a: 1 is Newton's method itself. |
| `shade` | 0 to 1 | 0.5 | How much the count darkens a point; 0 leaves the colors flat. |

| Kind | What it draws | Own parameters |
| --- | --- | --- |
| `newton` | The roots of a polynomial and which of them every point of the plane is led to. a = `relaxation` + i × `twist`; the roots attract only while \|1 − a\| is under 1. A point has arrived within 10⁻³ of a root. Its place on the palette is the fractional part of `color_shift` + k / m, plus 0.05 × `color_density` × ln(1 + ν): k counts the m roots from 0 in the order of their angles, from the positive real axis round. h is 6, times −2 / ln \|1 − a\| where that is more than 1 | `polynomial` (0 to 5, default 0), `degree` (2 to 12, default 3), `twist` (−1 to 1, default 0) |
| `nova` | The step of `newton` for p = zⁿ − 1 with a constant added: z → z − a (zⁿ − 1) / (n zⁿ⁻¹) + c, with n = `degree` and a = `relaxation`. `form` 0 is the map of the constants: c is the point and z starts at (a (n − 1) / (n − a))^(1/n), where the derivative of the step is 0 (at 1 for a = 1). `form` 1 is the Julia form: z starts at the point and c = `real` + i × `imaginary`. A point has arrived when a step moves it less than 10⁻⁵; an orbit that ends in a cycle never does. The place on the palette is `color_shift` + `color_density` × ln(1 + ν), with ν = n plus the part of the last step, and h is 40 | `form` (0 or 1, default 0), `degree` (2 to 8, default 3), `real` (−2 to 2, default 0.3), `imaginary` (−2 to 2, default 0) |

The polynomials of `newton`:

| `polynomial` | p(z) | Roots |
| --- | --- | --- |
| 0 | zⁿ − 1, with n = `degree` | n, on the unit circle |
| 1 | z³ − 2z + 2 | 3; the points around 0 and 1 swap for ever and stay clear |
| 2 | z⁸ + 15z⁴ − 16 | 8 |
| 3 | z⁶ + z³ − 1 | 6 |
| 4 | z⁵ − 3i z³ − (5 + 2i) z² + 3z + 1 | 5, with no symmetry |
| 5 | z⁴ + 3z² − 4 | 4: ±1 and ±2i |

`degree` is used by polynomial 0 only. The default tone of both kinds is `glow_radius` 0.004 and `glow_strength`
0.06.

### Orbit kinds

Each of the 64 streams of a render follows orbits from its own random numbers and plots their points.

| Kind | Map | Parameters | Place on the palette |
| --- | --- | --- | --- |
| `clifford` | x′ = sin(a y) + c cos(a x), y′ = sin(b x) + d cos(b y); the start is within ±0.5 and the first 32 steps are not plotted | `a` (−1.4), `b` (1.6), `c` (1), `d` (0.7), each from −3 to 3 | (1 + sin(a y)) / 2 |
| `de_jong` | x′ = sin(a y) − cos(b x), y′ = sin(c x) − cos(d y); the start is within ±0.5 and the first 32 steps are not plotted | `a` (1.4), `b` (−2.3), `c` (2.4), `d` (−2.1), each from −4 to 4 | `color_by` |
| `gumowski_mira` | x′ = y + a (1 − b y²) y + f(x), y′ = f(x′) − x, with f(x) = μ x + 2 (1 − μ) x² / (1 + x²). An orbit starts within ±`scatter` in x and in y, its first 2000 steps are not plotted and the next 200 000 are; then another starts | `mu` (−1 to 1, −0.496), `a` (0 to 0.1, 0.008), `b` (0 to 1, 0.05), `scatter` (0.1 to 20, 4) | `color_by` |
| `hopalong` | x′ = y − sgn(x) √\|b x − c\|, y′ = a − x, with sgn(0) = 0. An orbit starts within ±0.1 in x and in y and is plotted for `orbit_length` steps; then another starts | `a` (−100 to 100, 0.4), `b` (0 to 10, 1), `c` (−100 to 100, 0), `orbit_length` (1000 to 10 000 000, whole, 300 000) | `color_by` |
| `lorenz` | dx/dt = σ (y − x), dy/dt = x (ρ − z) − y, dz/dt = x y − β z, by the fourth-order Runge-Kutta method in steps of 0.005. The orbit starts at (1, 1, ρ), its first 4000 steps are not plotted, and it is followed for `duration` units of time; then the same orbit is followed again, so every stream draws the same lines. A point is plotted at a random place between two steps | `sigma` (1 to 40, 10), `rho` (1 to 250, 28), `beta` (0.1 to 10, 8/3), `turn` (−180 to 180 degrees, 0), `tilt` (−90 to 90 degrees, 0), `duration` (5 to 100 000, 150) | `color_by` |

`lorenz` is seen from the side, turned about its upright axis by `turn` and tipped towards the viewer by `tilt`:
the picture's x is x cos(turn) − y sin(turn), and its y is (z − ρ + 1) cos(tilt) + (x sin(turn) + y cos(turn))
sin(tilt). `tilt` 0 is the view along the y axis, the butterfly; 90 is the view from above. A short `duration`
shows the orbit's separate lines, a long one the density of the attractor.

`color_by` (0, 1 or 2) chooses what the palette shows. It is a choice, as `anti` and `colored_by` of
`buddhabrot` are: the app shows the names of the values in place of a slider.

| `color_by` | The place on the palette |
| --- | --- |
| 0 | The speed: the distance from a point to the next (for `lorenz`, in its three dimensions) |
| 1 | The direction of the step in the picture: \|angle\| / π |
| 2 | The position: the distance from the middle of the picture; for `lorenz`, the height z |

A speed and a position are not used as they are, but by their rank: before the render a kind measures 4096 points
spread over the first steps of 8 orbits of its own (at most 500 000 steps in all), and a value's place is the
share of the measured values under it. So the palette is used evenly whatever the numbers are.

The points of `gumowski_mira`, `hopalong` and `lorenz` are moved and scaled before they are plotted, because
their own sizes run from 1 to several hundred: the middle between the 2nd and the 98th percentile of the measured
points, in x and in y, becomes the origin, and half of the larger of the two spans becomes 1. A camera at the
origin with a zoom near 1.5 therefore frames any of them, and the picture stays framed while a parameter changes.
`clifford` and `de_jong` are plotted in their own coordinates.

#### Buddhabrot

`buddhabrot` plots the orbits of z → z² + c from z = 0 that leave the circle |z| = 2, for starting points c spread
evenly over the disc |c| ≤ 2. The picture is turned a quarter, as it is usually shown: its x is Im z and its y is
−Re z, so the real axis points down and the figure sits upright.

| Parameter | Range | Default | Meaning |
| --- | --- | --- | --- |
| `iterations` | 20 to 100 000, whole | 1000 | The steps an orbit is given. One that has not left by then is not drawn. |
| `shortest` | 1 to 10 000, whole | 10 | Orbits that leave in fewer steps are not drawn; at most half of `iterations`. A low value keeps the haze of short orbits, a high one leaves the long orbits that hold the detail. |
| `anti` | 0 or 1 | 0 | 1 draws the orbits that do not leave within `iterations` instead: the Anti-Buddhabrot. `shortest` is not used. |
| `turn` | −180 to 180 degrees | 0 | Turns the picture's x from Im z towards Im c: x = Im z cos(turn) + Im c sin(turn). |
| `tilt` | −180 to 180 degrees | 0 | Turns the picture's y from Re z towards Re c: y = −(Re z cos(tilt) + Re c sin(tilt)). With both at 90 the picture is the set of starting points, shaped like the Mandelbrot set. |
| `colored_by` | 0, 1 or 2 | 0 | See below. |

| `colored_by` | The place on the palette |
| --- | --- |
| 0 | The length n of the orbit the point belongs to: ln(n / `shortest`) / ln(`iterations` / `shortest`). With `anti`, the step k the point is: ln k / ln `iterations` |
| 1 | Where the orbit started: (Re c + 1.8) / 2.25, from the tip of the antenna to the cusp of the cardioid |
| 2 | The Nebulabrot: three iteration limits in place of the palette |

The Nebulabrot is the picture made of three Buddhabrots with iteration limits L₁ < L₂ < L₃, one in each of blue,
green and red. Here L₃ is `iterations` and the two others divide the range from `shortest` to `iterations` in
equal ratios (`shortest` 5 and `iterations` 5000 give the usual 50, 500 and 5000). An orbit of length up to L₁
counts in all three channels, one up to L₂ in green and red, a longer one in red alone, and each channel is
weighted by the share of the points it holds, which is close to 1 : 2 : 3. So the three classes are plotted in
the colors (85, 128, 255), (170, 255, 0) and (255, 0, 0), and the layer's palette is not used. With `anti` the
class is that of the step k in place of the length.

An orbit is plotted from z₁ = c to the first point outside the circle, and each point together with its mirror
image in the real axis, which the set of orbits has. The first point outside is plotted so that no rim shows at
|z| = 2.

Most starting points give nothing to draw, so they are not chosen blindly. The upper half of the disc is divided
into square cells, from a side of 0.25 down to 1/128 where it matters, and a cell is left out only when no point
of it can start an orbit that is drawn:

- all four corners lie in the main cardioid, or all four in the disc of period 2: both are convex above the real
  axis, so the cell lies inside the set;
- all four corners lie outside |c| = 2;
- for the Buddhabrot, every corner leaves at least 8 steps sooner than `shortest` and is, by the distance
  estimate, at least 1.4 sides of the cell away from the set. The potential of the set is a positive harmonic
  function, so by Harnack's inequality it falls to no less than a third within half of that distance, which is
  log₂ 3 steps; the rest of the 8 allows for counting the steps at radius 2;
- for the Anti-Buddhabrot, every corner leaves at least 8 steps sooner than `iterations` and is as far from the
  set.

The cells that remain are visited in a fixed order that returns to each in proportion to its area (a stride of
the golden ratio through their areas, from a starting place of the stream's own), and the point inside the cell
is random. A point in the cardioid or the disc of period 2 is refused by their formulas, and one whose orbit
comes back to a value it had is given up. The sampling stays even over the starting points that count, so the
picture is the same Buddhabrot as from blind sampling. With `shortest` 10 the cells cover 39 % of the half disc,
with `shortest` 200 or more 7 %.

#### Tone and views

The default tone of `clifford`, `de_jong`, `gumowski_mira`, `hopalong` and `lorenz` is `brightness` 1.1,
`gamma` 2.8, `highlight` 0.2, `glow_radius` 0.006 and `glow_strength` 0.5; that of `buddhabrot` is `brightness`
2.2, `gamma` 1.7, `highlight` 0.2, `glow_radius` 0.004 and `glow_strength` 0.3.

An orbit kind has no deep views as a flame has: a view that holds little of the picture is drawn from the few
points that land in it. A view magnified z times gets 1 / z² of them, so at 100 samples per pixel an attractor
that is smooth whole is grainy from about 8 times and falls apart into single points past 20 times; a
`buddhabrot`, whose points are spread over the whole disc, is grainy from about 4 times and noise at 20. More
samples per pixel push that out by their square root; the coordinates themselves, 64-bit numbers, would last to
a zoom of about 10¹³. A long orbit of the Lorenz system is not the orbit a more exact method would give, since its errors
double about every 0.8 units of time; it stays on the attractor, which is what the picture shows.

### Geometric kinds

The family has no kind yet.

### Solid kinds

A solid kind is a shape in three dimensions, seen by an eye that looks at its middle: every sample of the picture
is a ray from the eye. The eye stands `distance` away from the middle, `turn` degrees around the solid's upright
axis and `tilt` degrees above its level, and the shorter side of the picture spans `field_of_view` degrees at
zoom 1. The layer's camera moves, turns and magnifies that picture as it does any other: its `zoom` narrows the
angle the picture spans, as a longer lens does, and does not move the eye. To go nearer, or inside, lower
`distance`.

A ray walks by the kind's distance estimate, a number that is not more than the distance from a point to the
solid (for `mandelbulb` and `mandelbox` an approximation of it, and the ray walks 0.9 of it). It walks between
the two points where it crosses a ball around the solid, for at most 160 steps, and meets the solid where the
estimate is less than the width of a pixel at that depth: a larger picture or a deeper zoom shows finer detail,
and a small draft costs little. A ray whose steps run out is taken to have met the solid where it stopped. Before
the samples, one wider ray for every 4 × 4 pixels finds how deep all the rays of its cell are clear of the solid,
and they start there: the picture depends on its size in pixels, as a flame's does, and on nothing else.

Where a ray meets the solid, with n the normal (from four estimates a pixel's width around the point), l the
direction of the light and h the direction halfway between the light and the eye:

| Part | Value |
| --- | --- |
| The light | From `light_turn` degrees to the right of the eye and `light_tilt` degrees above it, as seen from the solid's middle: 0 and 0 is a lamp at the eye |
| lit | 1 where the way to the light is clear. A walk toward the light of at most 20 steps: 0 where it meets the solid, otherwise the least of 6 × estimate / depth along it |
| direct | max(n · l, 0) × (1 − `shadow` × (1 − lit)) |
| open | 1 − 0.75 × the sum over i = 1 … 5 of 0.6^(i − 1) × max(0, 1 − estimate(point + i s n) / (i s)), not under 0, with s a hundredth of the picture's shorter side at that depth: how open the surface is |
| linear light | open × (0.16 + 0.2 × (1 + n.y) / 2 + 0.08 × (1 − n · l) / 2) + direct × (0.72 + 0.5 × max(n · h, 0)³²) |
| `light` of the sample | √(linear light × e^(−1.6 × `fog` × far)), with far the depth past the nearest point of the ball, in radii of the ball |
| Place on the palette | `color_shift` + `color_density` × trap, with trap a number from about 0 to 1 the kind takes from the orbit of the point |

A ray that passes the solid by gives its halo: with a the least of estimate / depth along the ray and w a
fiftieth of the angle of the picture's shorter side, the light is 0.45 × `glow` / (1 + (a / w)²)², fading to
nothing where the ray cuts less than a third of the ball's radius into it, in the color of the nearest point. Under
0.004 the point stays clear.

A pixel of a solid is drawn again from the grid of samples where it differs from a neighbour by 0.2 or more, not
0.02: a lit rough surface differs a little everywhere, and only its sharper steps show as stairs.

| Parameter | Range | Default | Meaning |
| --- | --- | --- | --- |
| `turn` | −180 to 180 | 30 | Where the eye stands around the solid, in degrees. |
| `tilt` | −90 to 90 | 20 | How high the eye stands, in degrees: 90 looks straight down. |
| `distance` | see the kind | see the kind | From the eye to the middle of the solid. |
| `field_of_view` | 5 to 120 | 40 | The angle the shorter side of the picture spans at zoom 1, in degrees. |
| `light_turn` | −180 to 180 | −40 | The light, in degrees to the right of the eye. |
| `light_tilt` | −90 to 90 | 40 | The light, in degrees above the eye. |
| `shadow` | 0 to 1 | 0.7 | How dark the cast shadows are; 0 skips them and is faster. |
| `fog` | 0 to 1 | 0.3 | How fast the far parts fade to dark. |
| `glow` | 0 to 1 | 0.25 | The halo around the solid. |
| `color_density` | 0.01 to 100 | 1 | How fast the colors run along the palette. |
| `color_shift` | 0 to 2 | 0 | Where on the palette the colors start. |

| Kind | Solid and estimate | Own parameters |
| --- | --- | --- |
| `mandelbulb` | The points c whose orbit v → vⁿ + c from v = c stays within 4 of the origin, where a point at the distance r, the angle θ from the upright axis and the angle φ around it has the power rⁿ (sin nθ cos nφ, sin nθ sin nφ, cos nθ). The estimate is ½ r ln r / r′ at the end of the orbit, with r′ → n rⁿ⁻¹ r′ + 1 from 1. A whole power is computed by products alone, another by the angles. Trap: the least r of the orbit | `power` (2 to 16, default 8), `iterations` (2 to 200, whole, default 10), `distance` (0.5 to 20, default 3.6) |
| `mandelbox` | The points c whose orbit v → `scale` × ball(box(v)) + c from v = c stays bounded: box replaces each coordinate x outside ±`fold` by ±2 × `fold` − x, and ball multiplies v by 1 / `inner_radius`² where r < `inner_radius` and by 1 / r² where r < 1. The estimate is r / r′ at the end, with r′ → `scale` × (the factor of ball) × r′ + 1 from 1; the orbit ends past r = 1000. A `scale` between −1.2 and 1.2 is taken for −1.2 or 1.2: nearer to zero the solid has no end. Trap: half the least r after box | `scale` (−4 to 4, default 2), `fold` (0.5 to 2, default 1), `inner_radius` (0.05 to 1, default 0.5), `iterations` (2 to 100, whole, default 14), `distance` (0.1 to 150, default 30) |
| `menger_sponge` | `iterations` times: the point is turned `twist` degrees about the z axis, its coordinates are made positive and sorted, x the largest, it is turned `bend` degrees about the x axis, then x → s x − (s − 1), y → s y − (s − 1), z → (s − 1) / 2 − \|s z − (s − 1) / 2\| with s the `scale`. The estimate is the distance from the result to the cube from −1 to 1, divided by s to the power `iterations`. With `scale` 3 and no `twist` or `bend` it is the Menger sponge of that level in the cube from −1 to 1, and the estimate is a true bound; other values fold it into other shapes. Trap: half the least distance of the point from the origin after a step | `iterations` (1 to 30, whole, default 5), `scale` (2 to 4, default 3), `twist` and `bend` (−45 to 45, default 0), `distance` (0.05 to 30, default 5) |

The default tone of the three is `glow_radius` 0.004 and `glow_strength` 0.1. The `mandelbox` of `scale` 2 is a
cube 12 wide and that of a negative `scale` one 4 wide, both times `fold`; the `mandelbulb` of `power` 8 is
about 2.2 across.

### How far a view goes

A per-pixel kind computes every point with 64-bit numbers. Two neighbouring pixels stay different points while
the zoom times the picture's shorter side in pixels is under about 10¹⁶ near the origin: a zoom of 10¹³ at 1080
pixels. Past that neighbouring pixels get the same point and the picture turns into blocks; nothing is refused.
The app stops where a fractal's own zoom times the view's reaches 10¹⁰. It is the same for every per-pixel kind:
the distances at which `magnet`, `newton` and `nova` take an orbit to have arrived are measured in the plane of
z and do not shrink with the view.

A solid kind holds to the same zoom of 10¹⁰: a `menger_sponge` of 30 iterations still shows square holes there.
What runs out first is the detail the iterations give. A level of the sponge is a third of the one before, so a
view magnified three times as much needs one more of `iterations`; a `mandelbox` of 100 iterations has detail to
a zoom of about 10⁸; the surface of a `mandelbulb` is smooth in places at any number of iterations.

## Sheets

A stack of flat sheets, each lying on the ones before it, lit from one side: every sheet has a bright rim along its
edge, a gradient across it, and casts a soft shadow onto what is under it. It is drawn exactly, not sampled, so it
does not depend on `seed`, on the samples per pixel or on the supersample.

Positions and sizes are in units where the shorter side of the image is 2 wide, the middle of the image is
`[0, 0]`, x grows to the right and y grows downwards. A 1000 × 2000 image shows x from −1 to 1 and y from −2 to 2.

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `light` | degrees | 135 | Where the light comes from, counter-clockwise from the right: 90 is from above, 135 from the upper left. |
| `rim` | 0 to 4 | 0.5 | Brightness of the line along every edge; strongest on the edges that face the light. |
| `rim_width` | above 0, at most 2 | 0.006 | How far the rim reaches into the sheet. |
| `bevel` | −1 to 1 | 0.12 | How much brighter a sheet is on its side toward the light than on the other. |
| `shadow` | 0 to 1 | 0.3 | Darkness of the shadow a sheet casts. |
| `shadow_offset` | 0 to 1 | 0.03 | How far the shadow lies from the sheet, away from the light. |
| `shadow_blur` | above 0, at most 2 | 0.12 | Softness of the shadow's edge. |
| `gradient` | −1 to 1 | 0 | A brightness slope across the whole image, toward the light. |
| `stack` | list of sheets | required | 1 to 48, the first at the bottom. |

### Sheet

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `color` | color | required | The sheet's color. |
| `color_to` | color | none | With it the sheet is a gradient: `color` on its side toward the light, `color_to` on the far side of the image. |
| `opacity` | 0 to 1 | 1 | Below 1 the sheets under it show through, and its shadow is as much lighter. |
| `shapes` | list of shapes | required | 1 to 32. The sheet is their union. |

### Shape

A shape has a `kind` and the fields of that kind. `rotation` is in degrees, clockwise, and 0 where it is left out.

| Kind | Fields | What it is |
| --- | --- | --- |
| `box` | `center` `[x, y]`, `half` `[x, y]` above 0, `corners` `[top left, top right, bottom right, bottom left]`, `rotation` | A rectangle, half its width and height given, turned around its middle. Each corner is rounded with its own radius (0 where `corners` is left out); two corners on one side must fit on that side together. A radius as large as the height turns a short side into a quarter circle; all four at half the side make a circle. |
| `petal` | `base` `[x, y]`, `length`, `width` above 0 and at most `length`, `rotation` | A leaf with two pointed tips, standing on `base` and pointing up at rotation 0. |
| `ring` | `center` `[x, y]`, `radius`, `thickness`, both above 0, `span` above 0 and at most 360, `rotation` | A circle line `thickness` wide. With `span` below 360 it is an arc of that many degrees with round ends, its middle at the top at rotation 0. |
| `wave` | `level`, `swells`, `thickness` 0 or more, `rotation` | A curve across the whole image: the line `y = level` moved up and down by its swells and turned around `[0, level]`. With `thickness` 0 the shape is everything under the curve; above 0 it is a ribbon that wide along the curve. Its edge distance is approximate where the curve is steep. |

A swell is one sine: `height` (how far it moves the curve), `length` above 0 (the distance from crest to crest) and
`shift` (radians, 0 where left out). `swells` is a list of up to 4; an empty list leaves the line straight.

### Generated wallpapers

`fractique wallpaper SEED` and `fractique render --wallpaper SEED` write such a scene for a seed
(`crates/fractique-core/src/wallpaper.rs`). The seed picks one of 14 layouts and 6 materials, the colors and the
light; a layout or a material can be given instead. The shapes are laid out for the proportion of the size given,
from 1:4 to 4:1, so the scene is for images of that proportion. The scene has `sheets`, a `background` and no
layers, and is named `wallpaper-SEED`.

| Layout | What it stacks |
| --- | --- |
| `bands`, `discs`, `tiles`, `bars`, `fan` | Rounded boxes: wide bands, circles, a grid of quarter circles, slanted bars, an opened fan |
| `burst` | Petals around a middle |
| `arcs` | Rings and arcs |
| `waves`, `ribbons` | Waves: filled under the curve, or as ribbons |
| `slats`, `steps`, `frames` | Boxes with square corners: slanted slats, steps, nested frames |
| `planes`, `lines` | Waves without swells, which are straight: large tilted planes, thin lines |

| Material | What it sets |
| --- | --- |
| `relief` | One color, bright rims |
| `paper` | A ramp of colors, deep shadows |
| `glass` | Translucent sheets (`opacity` below 1) that mix where they overlap |
| `glow` | Dim sheets with wide bright rims |
| `steel` | Graphite, navy, forest, oxblood or bronze, thin sharp rims |
| `carbon` | Near black or near white with one accent color |

Each has a light and a dark version of the same shapes.

## Transform

One step of the flame: an affine map, then a weighted sum of variations, then an optional second affine map.

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `weight` | 0 or more | 1 | How often this transform is chosen, relative to the others. |
| `color` | 0 to 1 | 0 | The position in the palette this transform pulls points toward. |
| `color_speed` | 0 to 1 | 0.5 | How hard it pulls. At 1 every point it produces has exactly its color. |
| `opacity` | 0 to 1 | 1 | The share of its points that are drawn. At 0 it shapes the flame without being seen. |
| `affine` | `[a, b, c, d, e, f]` | identity | `x' = a·x + b·y + c`, `y' = d·x + e·y + f`, applied before the variations. |
| `post` | `[a, b, c, d, e, f]` | none | The same form, applied after the variations. |
| `variations` | list of variations | required | At least one. Their results are multiplied by their weights and added. |

## Variation

`{ "type": name, "weight": number, "params": [numbers] }`. `weight` defaults to 1. `params` is positional, only
for the variations that have parameters, and missing ones take their defaults.

In the formulas `r` is the distance from the origin, `θ = atan2(x, y)` and `φ = atan2(y, x)`. `ψ`, `ψ₁`, `ψ₂` … are
random numbers from 0 to 1, new for every point. `a` to `f` are the numbers of the transform's `affine` (of the
`final_transform`'s own when the variation is in it), and `w` is the variation's own `weight`. The result is then
multiplied by the weight like any other, so `radial_blur`, `arch`, `rays`, `blade`, `secant2` and `twintrian`
change shape with their weight and not only size.

| Type | Result (x, y) | Parameters |
| --- | --- | --- |
| `linear` | x, y |  |
| `sinusoidal` | sin x, sin y |  |
| `spherical` | (x, y) / r² |  |
| `swirl` | x sin r² − y cos r², x cos r² + y sin r² |  |
| `horseshoe` | (x − y)(x + y) / r, 2xy / r |  |
| `polar` | θ / π, r − 1 |  |
| `handkerchief` | r sin(θ + r), r cos(θ − r) |  |
| `heart` | r sin(θ r), −r cos(θ r) |  |
| `disc` | (θ / π) sin(π r), (θ / π) cos(π r) |  |
| `spiral` | (cos θ + sin r) / r, (sin θ − cos r) / r |  |
| `hyperbolic` | sin θ / r, r cos θ |  |
| `diamond` | sin θ cos r, cos θ sin r |  |
| `ex` | r (p₀³ + p₁³), r (p₀³ − p₁³) with p₀ = sin(θ + r), p₁ = cos(θ − r) |  |
| `julia` | √r (cos(θ/2 + Ω), sin(θ/2 + Ω)), Ω is 0 or π at random |  |
| `bent` | x doubled when negative, y halved when negative |  |
| `fisheye` | 2 (y, x) / (r + 1) |  |
| `eyefish` | 2 (x, y) / (r + 1) |  |
| `exponential` | exp(x − 1) (cos π y, sin π y) |  |
| `power` | r^(sin θ) (cos θ, sin θ) |  |
| `cosine` | cos(π x) cosh y, −sin(π x) sinh y |  |
| `bubble` | 4 (x, y) / (r² + 4) |  |
| `cylinder` | sin x, y |  |
| `tangent` | sin x / cos y, tan y |  |
| `cross` | (x, y) / |x² − y²| |  |
| `blur` | a random point in the unit disc |  |
| `gaussian_blur` | a random point, denser near the origin |  |
| `julian` | r^(dist / power) (cos t, sin t), t = (φ + 2π k) / power, k random | `power` = 2, `dist` = 1 |
| `curl` | (x, y) divided by the complex number 1 + c1 z + c2 z² | `c1` = 0.5, `c2` = 0.2 |
| `waves` | x + b sin(y / c²), y + e sin(x / f²) |  |
| `popcorn` | x + c sin(tan 3y), y + f sin(tan 3x) |  |
| `rings` | k (cos θ, sin θ), k = ((r + c²) mod 2c²) − c² + r (1 − c²) |  |
| `fan` | r (cos(θ ∓ t/2), sin(θ ∓ t/2)) with t = π c²: minus where (θ + f) mod t is above t/2 |  |
| `blob` | r (low + (high − low) (sin(waves θ) + 1) / 2) (cos θ, sin θ) | `low` = 0.5, `high` = 1, `waves` = 5 |
| `pdj` | sin(a y) − cos(b x), sin(c x) − cos(d y), with the parameters and not the transform's coefficients | `a` = 1.5, `b` = 1.8, `c` = 1.2, `d` = 2 |
| `fan2` | r (sin(θ ∓ t/2), cos(θ ∓ t/2)) with t = π x²: minus where θ + y − t trunc(2 θ y / t) is above t/2 | `x` = 0.5, `y` = 0.3 |
| `rings2` | k (sin θ, cos θ), k = r − 2p trunc((r + p) / 2p) + r (1 − p), p = val² | `val` = 0.5 |
| `perspective` | dist (x, y cos α) / (dist − y sin α), α = angle · π/2 | `angle` = 0.5, `dist` = 2 |
| `noise` | ψ₁ (x cos 2πψ₂, y sin 2πψ₂) |  |
| `juliascope` | r^(dist / power) (cos t, sin t), t = (±φ + 2π k) / power, k random, minus where k is odd | `power` = 2, `dist` = 1 |
| `radial_blur` | (r cos(φ + g sin α) + (g cos α − 1) x, r sin(φ + g sin α) + (g cos α − 1) y) / w, α = angle · π/2, g = w (ψ₁ + ψ₂ + ψ₃ + ψ₄ − 2) | `angle` = 0.5 |
| `pie` | ψ₃ (cos t, sin t), t = rotation + 2π (k + ψ₂ thickness) / slices, k = trunc(ψ₁ slices + 0.5) | `slices` = 6, `rotation` = 0, `thickness` = 0.5 |
| `ngon` | (x, y) (corners (1 / cos t − 1) + circle) / r^power, t = φ folded into ±π / sides | `sides` = 5, `power` = 2, `circle` = 1, `corners` = 1 |
| `rectangles` | (2 floor(x / p) + 1) p − x with p = `x`, the same for y with `y`; a size of 0 leaves that coordinate as it is | `x` = 0.5, `y` = 0.5 |
| `arch` | sin(ψ π w), sin²(ψ π w) / cos(ψ π w) |  |
| `square` | ψ₁ − 0.5, ψ₂ − 0.5 |  |
| `rays` | w tan(ψ π w) (cos x, sin y) / r² |  |
| `blade` | x (cos(ψ r w) + sin(ψ r w)), x (cos(ψ r w) − sin(ψ r w)) |  |
| `secant2` | x, 1 / cos(w r) moved by 1 toward 0 |  |
| `twintrian` | x t, x (t − π sin(ψ r w)), t = log₁₀(sin²(ψ r w)) + cos(ψ r w) |  |

`julian`, `juliascope` (`power`), `pie` (`slices`) and `ngon` (`sides`) refuse a first parameter of 0.

The `.flame` import reads a variation by its name in this table and a parameter as `name_parameter`
(`blob_low`, `pdj_a`, `ngon_sides`, `perspective_angle`), as flam3 and Apophysis write them.

`fractique variations` prints this list from the build you run.

## The .frcq file

The app's own file is not text. It is the four bytes `FRCQ`, one byte with the file version (`1`), then a JSON
document squeezed with raw DEFLATE. `fractique_core::file` packs and unpacks it; a document may be up to 64 MB
unpacked. A `.frcq` file is about a seventh of the same scene as indented JSON. A file of another version, or one
that does not unpack, is refused.

The document is the app's session, so a file opens as it was saved:

| Field | Type | Meaning |
| --- | --- | --- |
| `fractique` | `1` | Marks the document as the app's. |
| `scene` | scene | The scene as it was chosen, before the sliders and the view. For a wallpaper this is the flame the app held before it. |
| `shown` | scene | The scene exactly as it was on the screen: the sliders and the view applied, or the wallpaper itself. `fractique render file.frcq` and the other commands read this one. |
| `title` | string | The name the app shows. |
| `random` | `true`, `false` | Whether the picture came from Randomize. |
| `seed` | integer | The number of the last random flame or wallpaper. |
| `generator` | `1`, `2` | The random generator that made the last random flame. A file without it was written before the second generator: `1`. |
| `fractals` | `true`, `false` | Whether the **Fractals** tab was chosen. A file without it opens in the tab of its scene's first layer. |
| `kind` | a name | The kind of fractal chosen in that tab. |
| `adjust` | `[brightness, gamma, glow, zoom]` | The tone and zoom sliders, as factors: 1 changes nothing. |
| `view` | `[right, down, zoom]` | Where the picture was moved to, in shorter sides of the picture from its middle, and the magnification. |
| `picture` | `[width, height]` or `null` | The picture's size; `null` is the window's own. |
| `layer` | integer | The chosen layer, from 0. |
| `inspector` | `true`, `false`, `null` | Whether the controls were open. |
| `optics` | `[refraction, depth, bezel]` | The fine tuning of the glass. |
| `wall` | object | The wallpaper choices: `on` (wallpapers or flames), `live` (a wallpaper is shown), `layout` and `material` (a name, or `null` for any), `held` (the layout and material kept by Mutate), `dark`, `hue` (degrees, or `null`). |

A plain JSON scene file has none of this: it is the scene alone. `fractique convert` turns a `.frcq` file into
JSON and back ([usage.md](usage.md#11-files-of-the-app)).

## Versions

A scene that loads in one version of Fractique loads in every later one. When a change to the renderer would alter
what existing scenes look like, `version` is how old scenes keep their look; see [determinism.md](determinism.md).

Fractal layers came without a new version: `fractal` is a field a layer may hold in place of `flame`, so every
scene written before reads and renders as it did. A build from before them refuses a scene that has one, as it
refuses any field it does not know.
