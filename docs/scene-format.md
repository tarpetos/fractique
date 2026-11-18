# Scene format

A scene is one JSON document. It holds everything a render depends on, so the same scene always renders to the
same image (see [determinism.md](determinism.md)). This file is the contract: it changes together with
`crates/fractique-core/src/scene.rs` and `variation.rs`.

Unknown fields are refused, so a typo in a field name is an error and not a silently ignored setting. Fields with
a default may be left out. Colors are sRGB with each channel from 0 to 1.

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
| `flame` | flame | required | What the layer shows. |

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
`shift` (radians, 0 where left out). A wave takes up to 4.

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

In the formulas `r` is the distance from the origin, `θ = atan2(x, y)` and `φ = atan2(y, x)`.

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

`fractique variations` prints this list from the build you run.

## Versions

A scene that loads in one version of Fractique loads in every later one. When a change to the renderer would alter
what existing scenes look like, `version` is how old scenes keep their look; see [determinism.md](determinism.md).
