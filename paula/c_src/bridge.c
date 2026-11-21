/* paula bridge: compiles the UNMODIFIED upstream pt2_audio.c into this translation unit with the
** SDL audio-device calls redirected to no-ops (macro trick, src/ is not edited), then supplies
** no-op stubs for the UI/visuals symbols the replayer/paula/audio files reference.
** SDL is only used for its headers (types); libSDL2 is never linked.
**
** The engine has global state: one engine per process. The Rust side serialises access. */
#include <SDL2/SDL.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>
#include <stdio.h>
#include <math.h>

/* ---- redirect SDL audio device so nothing real opens and no audio thread exists ---- */
static SDL_AudioDeviceID paula_open(const char *d, int c, const SDL_AudioSpec *w, SDL_AudioSpec *h, int a)
{ (void)d; (void)c; (void)a; *h = *w; return 2; }
static void paula_nop_dev(SDL_AudioDeviceID id) { (void)id; }
static void paula_pause(SDL_AudioDeviceID id, int p) { (void)id; (void)p; }
#define SDL_OpenAudioDevice(a,b,c,d,e) paula_open(a,b,c,d,e)
#define SDL_PauseAudioDevice(a,b) paula_pause(a,b)
#define SDL_LockAudioDevice(a) paula_nop_dev(a)
#define SDL_UnlockAudioDevice(a) paula_nop_dev(a)
#define SDL_CloseAudioDevice(a) paula_nop_dev(a)
#include "../../src/pt2_audio.c"   /* unmodified upstream file */
#undef SDL_OpenAudioDevice
#undef SDL_PauseAudioDevice
#undef SDL_LockAudioDevice
#undef SDL_UnlockAudioDevice
#undef SDL_CloseAudioDevice

#include "../../src/pt2_replayer.h"
#include "../../src/pt2_paula.h"
#include "../../src/pt2_config.h"
#include "../../src/pt2_hpc.h"

/* ---- globals normally owned by pt2_main.c / pt2_config.c / pt2_hpc.c / tables ---- */
module_t *song = NULL;
config_t config;
hpcFreq_t hpcFreq = { 1000000000ULL, 1.0, 1.0 };
uint16_t analyzerColors[36];
uint16_t vuMeterColors[48];

/* ---- no-op stubs for UI / visuals ---- */
void displayMsg(const char *m) { (void)m; }
void showErrorMsgBox(const char *fmt, ...) { va_list a; va_start(a, fmt); fprintf(stderr, "paula engine: "); vfprintf(stderr, fmt, a); fprintf(stderr, "\n"); va_end(a); }
void pointerSetMode(uint8_t m, bool c) { (void)m; (void)c; }
void pointerSetModeThreadSafe(uint8_t m, bool c) { (void)m; (void)c; }
void posEdClearNames(void) {}
void createSampleMarkTable(void) {}
void renderMuteButtons(void) {}
void updateCurrSample(void) {}
void updateCursorPos(void) {}
bool changePathToHome(void) { return true; }
void calcAudioLatencyVars(int32_t b, int32_t f) { (void)b; (void)f; }
void setSyncTickTimeLen(uint32_t i, uint64_t f) { (void)i; (void)f; }
void fillVisualsSyncBuffer(void) {}
void resetChSyncQueue(void) {}
/* ---- per-channel visuals state (spectrum analyser). The engine reports what Paula is doing
** through the setVisuals* hooks, like pt2's own scopes use them: pointer/length given while DMA is
** off or just before it starts are latched when DMACON turns the channel on; the pointer/length
** set after that are the loop that takes over when the first pass ends. The hooks run inside
** paula_render, on the caller's thread, under the Rust Mutex, so no extra locking is needed. ---- */
#define PAULA_CHANNELS 4
typedef struct
{
	const int8_t *cur, *pend; /* playing / next (loop) sample data */
	int32_t curLen, pendLen;  /* words */
	double pos;               /* bytes into cur */
	int32_t period, volume, active;
} paulaCh_t;
static paulaCh_t pch[PAULA_CHANNELS];

static void paula_clear_visuals(void) { memset(pch, 0, sizeof pch); }

void setVisualsDMACON(uint16_t bits)
{
	for (int i = 0; i < PAULA_CHANNELS; i++)
	{
		if (!(bits & (1 << i))) continue;
		if (bits & 0x8000)
		{
			pch[i].cur = pch[i].pend;
			pch[i].curLen = pch[i].pendLen;
			pch[i].pos = 0.0;
			pch[i].active = pch[i].cur != NULL;
		}
		else
			pch[i].active = 0;
	}
}
void setVisualsVolume(int32_t c, uint16_t v) { if ((uint32_t)c < PAULA_CHANNELS) pch[c].volume = v > 64 ? 64 : v; }
void setVisualsPeriod(int32_t c, uint16_t p) { if ((uint32_t)c < PAULA_CHANNELS) pch[c].period = p; }
void setVisualsLength(int32_t c, uint16_t l) { if ((uint32_t)c < PAULA_CHANNELS) pch[c].pendLen = l; }
void setVisualsDataPtr(int32_t c, const int8_t *s) { if ((uint32_t)c < PAULA_CHANNELS) pch[c].pend = s; }

/* Paula reads one byte every `period` colour clocks (PAL 3546895 Hz): bytes per output frame */
static double paula_step(const paulaCh_t *c)
{
	return c->period < 1 ? 0.0 : 3546895.0 / (double)c->period / (double)audio.outputRate;
}

static void paula_advance_visuals(int frames)
{
	for (int i = 0; i < PAULA_CHANNELS; i++)
	{
		paulaCh_t *c = &pch[i];
		if (!c->active) continue;
		c->pos += paula_step(c) * frames;
		const double curBytes = c->curLen * 2.0;
		if (c->pos >= curBytes)
		{ /* first pass over: the loop data takes over */
			c->pos -= curBytes;
			c->cur = c->pend;
			c->curLen = c->pendLen;
			const double loopBytes = c->curLen * 2.0;
			if (c->cur == NULL || loopBytes < 2.0) { c->active = 0; continue; }
			c->pos = fmod(c->pos, loopBytes);
		}
	}
}

/* ---- the headless API (pull-style) ---- */
/* ---- the headless API (pull-style) ---- */
#define PAULA_MAX_SAMPLE_LEN 65534

int paula_init(int rate)
{
	if (song != NULL) return 0; /* already initialised: one engine per process */
	memset(&config, 0, sizeof config);
	config.maxSampleLength = PAULA_MAX_SAMPLE_LEN;
	config.soundFrequency = rate;
	config.soundBufferSize = 1024;
	config.mod2WavOutputFreq = rate;
	config.amigaModel = MODEL_A500;
	config.stereoSeparation = 20;
	editor.timingMode = TEMPO_MODE_CIA;
	editor.initialTempo = 125;
	editor.initialSpeed = 6;
	if (!setupAudio()) return 1;
	song = createEmptyMod();
	if (!song) return 2;
	editor.currPatternDisp = &song->header.patternTable[0];
	editor.currPosEdPattDisp = &song->header.patternTable[0];
	modSetTempo(125, false);
	modSetSpeed(6);
	return 0;
}

int paula_max_sample_len(void) { return PAULA_MAX_SAMPLE_LEN; }

/* clear filter, BLEP, downsampler, dither and tick-counter state, so that the same module and
** the same render calls give bit-identical output (Req 5.4) */
/* Paula's voices (static in pt2_paula.c) keep their held sample, DMA buffer and fetch stage
** after the replayer stops; startDMA() does not clear them, so a render after a mid-song stop
** started from different voice state (issue #35). We cannot edit src/, so we drive every voice
** through the same fixed sequence with the public register API: silent null sample, fixed period
** and volume, DMA on, mix a few samples (the voices fetch zeros), DMA off. Afterwards the voice
** state depends only on this sequence. paulaSetup() below then clears filters and BLEP. */
static void paula_reset_voices(void)
{
	static float scratchL[256], scratchR[256];
	const int8_t *nul = paulaGetNullSamplePtr();
	for (int32_t i = 0; i < PAULA_VOICES; i++)
	{
		const uint32_t a = 0xDFF0A0 + (i * 16);
		paulaWritePtr(a + 0, nul);      /* AUDxLC  */
		paulaWriteWord(a + 4, 1);       /* AUDxLEN, in words */
		paulaWriteWord(a + 6, 113);     /* AUDxPER */
		paulaWriteWord(a + 8, 64);      /* AUDxVOL */
	}
	paulaWriteWord(0xDFF096, 0x800F);   /* DMA on for all four voices */
	paulaGenerateSamples(scratchL, scratchR, 256);
	paulaWriteWord(0xDFF096, 0x000F);   /* DMA off */
	for (int32_t i = 0; i < PAULA_VOICES; i++)
		paulaWriteWord(0xDFF0A0 + (i * 16) + 8, 0);
}

static void paula_reset_audio_state(void)
{
	const int32_t mixFreq = audio.oversamplingFlag ? audio.outputRate*2 : audio.outputRate;
	paula_reset_voices();
	paulaSetup(mixFreq, audio.amigaModel);
	clearDownsample2xStates();
	resetAudioDither();
	audio.tickSampleCounter = 0;
	audio.tickSampleCounterFrac = 0;
}

/* clear the module to the Req 1.4 empty state and stop playback */
void paula_reset(void)
{
	modStop();
	paula_clear_visuals();
	for (int i = 0; i < MAX_PATTERNS; i++)
		memset(song->patterns[i], 0, MOD_ROWS * sizeof (note_t) * PAULA_VOICES);
	memset(song->sampleData, 0, (size_t)(MOD_SAMPLES + 2) * config.maxSampleLength);
	moduleSample_t *s = song->samples;
	for (int i = 0; i < MOD_SAMPLES; i++, s++)
	{
		memset(s->text, 0, sizeof s->text);
		s->length = 0; s->volume = 0; s->fineTune = 0; s->loopStart = 0; s->loopLength = 2;
	}
	memset(song->header.patternTable, 0, sizeof song->header.patternTable);
	memset(song->header.name, 0, sizeof song->header.name);
	song->header.songLength = 1;
	song->header.initialTempo = 125;
	modSetPos(0, 0);
	modSetSpeed(6);
	modSetTempo(125, false);
	paula_reset_audio_state();
}

/* idx0 = 0..30; len, loop_start, loop_len in bytes (even); returns 0 or 1 on bad args */
int paula_set_sample(int idx0, const int8_t *data, int len, int volume, int finetune, int loop_start, int loop_len)
{
	if (idx0 < 0 || idx0 >= MOD_SAMPLES || len < 0 || len > PAULA_MAX_SAMPLE_LEN) return 1;
	moduleSample_t *s = &song->samples[idx0];
	memset(&song->sampleData[s->offset], 0, config.maxSampleLength);
	if (len > 0) memcpy(&song->sampleData[s->offset], data, len);
	s->length = len;
	s->volume = (int8_t)(volume > 64 ? 64 : (volume < 0 ? 0 : volume));
	s->fineTune = (uint8_t)(finetune & 0xF);
	s->loopStart = loop_start;
	s->loopLength = loop_len < 2 ? 2 : loop_len;
	if (s->loopStart + s->loopLength > s->length) { s->loopStart = 0; s->loopLength = 2; }
	return 0;
}

void paula_set_cell(int pat, int row, int ch, int period, int sample, int cmd, int param)
{
	note_t *n = &song->patterns[pat][row * PAULA_VOICES + ch];
	n->period = period; n->sample = sample; n->command = cmd; n->param = param;
}

void paula_get_cell(int pat, int row, int ch, int *period, int *sample, int *cmd, int *param)
{
	note_t *n = &song->patterns[pat][row * PAULA_VOICES + ch];
	*period = n->period; *sample = n->sample; *cmd = n->command; *param = n->param;
}

int  paula_get_order(int pos) { return song->header.patternTable[pos & 127]; }
void paula_set_order(int pos, int pat) { song->header.patternTable[pos & 127] = (uint16_t)pat; }
void paula_set_song_length(int len) { song->header.songLength = (uint16_t)len; }
int  paula_get_song_length(void) { return song->header.songLength; }

void paula_set_bpm(int bpm) { modSetTempo(bpm, true); }
void paula_set_speed(int sp) { modSetSpeed(sp); }
int  paula_get_bpm(void) { return song->currBPM; }
int  paula_get_speed(void) { return song->speed; }

void paula_play(void)
{
	editor.playMode = PLAY_MODE_NORMAL;
	paula_reset_audio_state();
	modPlay(-1, 0, 0);   /* song mode, position 0, row 0 */
	editor.currMode = MODE_PLAY;
}

void paula_stop(void) { modStop(); editor.currMode = MODE_IDLE; paula_clear_visuals(); }

/* Play the song from song position `pos` (0..songLength-1), row 0, like paula_play does for
** position 0. Used to hear a pattern that was just appended without waiting for the song. */
int paula_play_from(int pos)
{
	if (pos < 0 || pos >= song->header.songLength) return -1;
	editor.playMode = PLAY_MODE_NORMAL;
	paula_reset_audio_state();
	modPlay(-1, (int16_t)pos, 0);
	editor.currMode = MODE_PLAY;
	return 0;
}
int  paula_is_playing(void) { return editor.songPlaying ? 1 : 0; }

/* Channel mute, as the tracker's own mute keys do it: with editor.muted set, the replayer starts
** no new DMA and skips volume updates for that channel. Muting also stops the channel's DMA, so
** a note already sounding goes quiet at once; unmuting takes effect at the channel's next note. */
void paula_set_muted(int ch, int on)
{
	if (ch < 0 || ch >= PAULA_CHANNELS) return;
	editor.muted[ch] = on ? true : false;
	if (on) paulaWriteWord(0xDFF096, (uint16_t)(1 << ch)); /* DMACON: clear bit ch */
}
int paula_is_muted(int ch) { return (ch >= 0 && ch < PAULA_CHANNELS && editor.muted[ch]) ? 1 : 0; }
int  paula_get_pos(void) { return song->currPos; }
int  paula_get_row(void) { return song->currRow; }

/* stereo interleaved int16; same loop the SDL callback runs */
void paula_render(int16_t *out, int frames)
{
	if (!editor.songPlaying) { memset(out, 0, (size_t)frames * 4); return; }
	/* small slices, so the channel positions stay close to the audio (a retrigger inside a slice
	** is at most 32 frames off). Plain slicing: the audio is the same for any slice size. */
	while (frames > 0)
	{
		const int n = frames > 32 ? 32 : frames;
		audioCallback(NULL, (Uint8 *)out, n * 4);
		paula_advance_visuals(n);
		out += n * 2;
		frames -= n;
	}
}

/* State of channel ch (0..3): returns 1 when it is sounding. period 0 when idle, volume 0..64. */
int paula_channel_state(int ch, int *period, int *volume)
{
	if (ch < 0 || ch >= PAULA_CHANNELS || !editor.songPlaying || !pch[ch].active) { *period = 0; *volume = 0; return 0; }
	*period = pch[ch].period; *volume = pch[ch].volume;
	return 1;
}

/* Rebuild the next n output frames of channel ch (raw, before the Amiga filters and the stereo
** mix): sample data stepped at the channel period, linear interpolation, scaled by volume,
** -1..1. Returns 0 (and zeros) when the channel is silent. Does not move the channel. */
int paula_channel_signal(int ch, float *out, int n)
{
	memset(out, 0, (size_t)n * sizeof *out);
	if (ch < 0 || ch >= PAULA_CHANNELS || !editor.songPlaying || !pch[ch].active || pch[ch].volume == 0) return 0;
	const paulaCh_t *c = &pch[ch];
	const double step = paula_step(c);
	const float gain = (float)c->volume / 64.0f / 128.0f;
	const int8_t *data = c->cur;
	int32_t len = c->curLen * 2;
	const int8_t *loop = c->pend;
	const int32_t loopLen = c->pendLen * 2;
	double pos = c->pos;
	if (loopLen < 2 && loop != NULL) return 0;
	for (int i = 0; i < n; i++, pos += step)
	{
		if (data == NULL || len < 2) break;
		while (pos >= len)
		{
			pos -= len;
			data = loop; len = loopLen;
			if (data == NULL || len < 2) return 1;
		}
		const int32_t k = (int32_t)pos;
		const float f = (float)(pos - k);
		const float a = data[k];
		const float b = (k + 1 < len) ? data[k + 1] : (loop != NULL && loopLen >= 2 ? loop[0] : a);
		out[i] = (a + (b - a) * f) * gain;
	}
	return 1;
}
