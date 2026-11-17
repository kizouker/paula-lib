---
name: push2
description: Drive an Ableton Push 2 directly from our Rust app, without Ableton Live (Phase A of the ableton-integration spec). Covers USB ports, User mode, pad/LED MIDI, scripted settings via sysex, and the 960x160 display protocol. Use when writing or reviewing code that talks to Push 2.
---

# Push 2 (direct, no Live)

Source of truth: Ableton's [Push 2 MIDI and Display Interface Manual](https://github.com/Ableton/push-interface/blob/main/doc/AbletonPush2MIDIDisplayInterface.asc) (rev. 1.1, firmware 1.0.60, Jan. 2017). The manual may not be reproduced, so this skill only summarises facts and links to it. Read the manual for any detail not listed here.

## Safety rules (read first)
- Use **only documented** sysex commands (allowlist below) and only the documented display messages (frame header + 512-byte pixel data). Ableton warns that reserved messages can corrupt calibration data or make the device unusable until reflashed.
- Wait for a sysex reply before sending the next command that expects one. Do not nest commands.
- Never write to pad calibration (`0x22`, `0x1D` reads only).

## 1. How Push 2 shows up
- One USB 2.0 composite device: vendor ID `0x2982`, product ID `0x1967`.
- **Two MIDI ports** (macOS names): `Ableton Push 2 Live Port` and `Ableton Push 2 User Port`. Only one app can open a port at a time.
- **One display interface**: USB bulk, interface 0, OUT endpoint `0x01`, opened with libusb (in Rust: `rusb`).

## 2. MIDI modes
- Power-on mode is **Live mode** (port 1 active). Our app uses **User mode** (port 2 active).
- Set mode: `F0 00 21 1D 01 01 0A <m> F7`, m = 0 Live, 1 User, 2 Dual. The reply goes to both ports.
- The User button always sends to both ports, so the app can toggle mode on that button.
- In User mode, MIDI clock/start/stop for LED animations must be sent on port 2.

## 3. Controls and LEDs (MIDI messages)
| Message | Meaning |
|---|---|
| Note on `0x90`, note n, velocity v | pad pressed (v = velocity); also sets an LED colour when sent to Push |
| Note off `0x80` | pad released |
| Control change `0xB0`, cc n, value v | button pressed/released, encoder turned; also sets button LEDs |
| Poly pressure `0xA0` / channel pressure `0xD0` | pad aftertouch |
| Pitch bend `0xE0` | touch strip |

- Pads: an 8×8 grid of notes. The manual's examples confirm note 36 (`0x24`) bottom-left and note 99 (`0x63`) top-right. **[verify the full grid and all button/encoder CC numbers against the MIDI Mapping figure in the manual before coding]**
- LED colour: velocity = palette index. MIDI channel 0 = static; other channels = blink/pulse/one-shot animations timed by MIDI clock.

## 4. Scripted settings (sysex)
General format: `F0 00 21 1D 01 01 <command id> <args...> F7`.

Allowlist of documented commands useful for us:
| ID | Command | Reply |
|---|---|---|
| `0x03` / `0x04` / `0x05` | set / get / reapply LED colour palette entry | get: yes |
| `0x06` / `0x07` | set / get global LED brightness | get: yes |
| `0x08` / `0x09` | set / get display backlight brightness | |
| `0x0A` | set MIDI mode | yes |
| `0x0B` | LED PWM frequency correction | |
| `0x17` / `0x18` / `0x19` | touch strip config / get / LEDs | get: yes |
| `0x1A` | request statistics (power supply, uptime, run ID) | yes |
| `0x1B` | set pad parameters | |
| `0x1E` / `0x1F` | set / get aftertouch mode | get: yes |
| `0x20` / `0x21` | set / get pad velocity curve entry | get: yes |
| `0x28` / `0x29` | select / get pad settings | get: yes |
| `0x30` / `0x31` / `0x32` | pedal configuration and curves | |

Argument layouts are in the manual's chapter for each command. **[verify whether settings survive a power cycle; assume they do not, and re-send them when the app connects]**

Suggested pattern: keep one settings file (for example `push2-settings.toml`: brightness, velocity curve, palette) and apply it on connect. That makes settings scriptable and repeatable.

## 5. Display protocol
- 960 × 160 pixels, 16 bit per pixel, little endian, bit layout `bbbbb gggggg rrrrr` (blue in the high bits, red in the low bits).
- Each frame: send the fixed **16-byte frame header** `FF CC AA 88` followed by 12 bytes `00`, then the pixel data.
- Pixel data: 160 lines, top line first. Each line = 1920 bytes of pixels (leftmost first) + 128 filler bytes = 2048 bytes. Total 327 680 bytes. Send in large buffers (for example 16 KB) for speed.
- **XOR** every line buffer with the 32-bit pattern `0xFFE7F3E7`, little endian: bytes are XORed with `E7 F3 E7 FF` repeating.
- The display runs at 60 fps and is double-buffered. A frame shows when complete. With no frame for 2 s the display goes black, so keep sending (at least every ~1 s even when nothing changes).

## 6. Rust sketch (Phase A)
- MIDI: `midir`, open ports whose name contains `Push 2 User Port`.
- Display: `rusb`, open `0x2982:0x1967`, claim interface 0, bulk-write header then pixels to endpoint `0x01`.
- Rendering: draw the tracker view (ABL-4) into a 960×160 RGB buffer, convert to the 16-bit layout, add filler, XOR, send.
- Put the device code in its own module (`push2::midi`, `push2::display`) that knows nothing about MOD data.

## 7. Tests
- Unit: pixel conversion (one known colour → expected two bytes), XOR of a line, line length 2048, frame length 327 680.
- Unit: sysex builder produces `F0 00 21 1D 01 01 0A 01 F7` for "User mode".
- Hardware (manual): pad press prints note 36–99; a test frame (colour bars) appears; unplugging does not crash the app.
