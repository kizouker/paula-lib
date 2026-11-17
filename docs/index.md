---
title: protracker2-agent-assist
---

<img class="hero" src="{{ '/assets/banner.svg' | relative_url }}" alt="protracker2-agent-assist: describe a beat, get editable tracker patterns">

An agent-assisted ProTracker 2-style tracker, built on an open-source ProTracker 2 replayer (see [Credits](#credits)). Describe a beat or a bassline in plain language and get editable tracker patterns back.

## Explore

<div class="explore">
  <a href="{{ '/sample-conversion/' | relative_url }}">
    <img src="{{ '/sample-conversion/figures/1_alias.png' | relative_url }}" alt="">
    <strong>8-bit sample conversion</strong>
    <span>From a modern recording to a ProTracker sample, with the math and figures.</span>
  </a>
  <a href="{{ '/sample-conversion/simulations.html' | relative_url }}">
    <img src="{{ '/sample-conversion/figures/2_quantize.png' | relative_url }}" alt="">
    <strong>Interactive simulations</strong>
    <span>Aliasing, dither and error moments, live in the browser.</span>
  </a>
  <a href="{{ '/sample-conversion/three-d/' | relative_url }}">
    <img src="{{ '/sample-conversion/figures/3_moments.png' | relative_url }}" alt="">
    <strong>Simulations in Rust</strong>
    <span>The same, compiled to WebAssembly with three-d.</span>
  </a>
</div>

## Specs

- [Vision](vision.html)
- [Non-functional requirements](non-functional-requirements.html)
- [UX requirements](ux-requirements.html)

Source: [github.com/kizouker/protracker2-agent-assist](https://github.com/kizouker/protracker2-agent-assist)

## Credits

The replayer is based on the ProTracker 2 clone by Olav "8bitbubsy" Sørensen ([pt2-clone](https://github.com/8bitbubsy/pt2-clone)), used under its BSD licence. See [LICENSE](https://github.com/kizouker/protracker2-agent-assist/blob/main/LICENSE). The starter samples are the author's own recordings.
