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
void setVisualsDMACON(uint16_t b) { (void)b; }
void setVisualsVolume(int32_t c, uint16_t v) { (void)c; (void)v; }
void setVisualsPeriod(int32_t c, uint16_t p) { (void)c; (void)p; }
void setVisualsLength(int32_t c, uint16_t l) { (void)c; (void)l; }
void setVisualsDataPtr(int32_t c, const int8_t *s) { (void)c; (void)s; }

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
static void paula_reset_audio_state(void)
{
	const int32_t mixFreq = audio.oversamplingFlag ? audio.outputRate*2 : audio.outputRate;
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

void paula_stop(void) { modStop(); editor.currMode = MODE_IDLE; }
int  paula_is_playing(void) { return editor.songPlaying ? 1 : 0; }
int  paula_get_pos(void) { return song->currPos; }
int  paula_get_row(void) { return song->currRow; }

/* stereo interleaved int16; same loop the SDL callback runs */
void paula_render(int16_t *out, int frames)
{
	if (!editor.songPlaying) { memset(out, 0, (size_t)frames * 4); return; }
	audioCallback(NULL, (Uint8 *)out, frames * 4);
}
