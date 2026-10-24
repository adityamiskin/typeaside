#!/usr/bin/env python3
"""Decode hf-internal-testing/librispeech_asr_dummy's clean validation parquet.

Requires numpy, pyarrow and soundfile in a scratch virtualenv. Inputs are distinct
utterances, padded with silence instead of cutting the last word to hit a duration.
"""
import io
import argparse
import hashlib
import json
from pathlib import Path
import wave

import numpy as np
import pyarrow.parquet as pq
import soundfile as sf

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('parquet', type=Path)
parser.add_argument('destination', type=Path)
parser.add_argument('--corpus', action='store_true', help='Also export all utterances for release-cut risk checks')
parser.add_argument('--stress', action='store_true', help='Also repeat the existing English fixture six times')
args = parser.parse_args()
parquet, destination = args.parquet, args.destination
destination.mkdir(parents=True, exist_ok=True)
rows = pq.read_table(parquet).to_pylist()
manifest = []
for name, seconds, indices in [('5s', 5, [18]), ('10s', 10, [19, 20]),
                               ('30s', 30, [25, 30, 32, 36, 37]), ('word', .85, [37])]:
    parts = []
    for index in indices:
        audio, rate = sf.read(io.BytesIO(rows[index]['audio']['bytes']))
        assert rate == 16000 and audio.ndim == 1
        parts.extend([audio, np.zeros(800)])
    audio = np.concatenate(parts)
    if name == 'word':
        audio = audio[int(.45 * rate):int(1.3 * rate)]
    else:
        assert len(audio) <= seconds * rate
        audio = np.pad(audio, (0, seconds * rate - len(audio)))
    sf.write(destination / f'{name}.wav', audio, rate, subtype='PCM_16')
    manifest.append(dict(name=name, rows=indices, seconds=len(audio) / rate,
                         reference='TRUE' if name == 'word' else ' '.join(rows[i]['text'] for i in indices)))
(destination / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
word, rate = sf.read(destination / 'word.wav')
sf.write(destination / 'word-tight.wav', word[:int(.58 * rate)], rate, subtype='PCM_16')
(destination / 'source.json').write_text(json.dumps(dict(parquet_sha256=hashlib.sha256(parquet.read_bytes()).hexdigest()), indent=2) + '\n')
if args.stress:
    fixture = Path(__file__).resolve().parents[1] / 'tests/fixtures/local-models-en.wav'
    with wave.open(str(fixture)) as source:
        params, audio = source.getparams(), source.readframes(source.getnframes())
    with wave.open(str(destination / 'repeated.wav'), 'wb') as target:
        target.setparams(params)
        target.writeframes(audio * 6)
if args.corpus:
    utterances = destination / 'utterances'
    utterances.mkdir(exist_ok=True)
    clips = []
    for index, row in enumerate(rows):
        audio, rate = sf.read(io.BytesIO(row['audio']['bytes']))
        assert rate == 16000 and audio.ndim == 1
        name = f'{index:03d}'
        sf.write(utterances / f'{name}.wav', audio, rate, subtype='PCM_16')
        clips.append(dict(name=name, language='en', parts=[dict(wav=f'utterances/{name}.wav', text=row['text'])]))
    for clip in manifest:
        clips.append(dict(name=clip['name'], language='en', parts=[dict(wav=clip['name'] + '.wav', text=clip['reference'])]))
    (destination / 'release-risk.json').write_text(json.dumps(dict(models=['parakeet-v3'], iterations=1, clips=clips), indent=2) + '\n')
