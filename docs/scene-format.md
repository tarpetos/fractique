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
| `layers` | list of layers | required | 1 to 16, blended in order: the first is at the bottom. |

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
