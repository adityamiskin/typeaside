package app.typeaside.keyboard

import android.Manifest
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import androidx.annotation.RequiresPermission
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.math.log10
import kotlin.math.sqrt

private const val SAMPLE_RATE = 16_000
private const val WAV_HEADER = 44
// The hosted API rejects audio over 20 MB, about ten minutes of 16 kHz mono PCM.
private const val MAX_PCM_BYTES = 20_000_000 - WAV_HEADER

/** One recording; [stop] may be called before [record] starts reading. */
class Recorder {
  @Volatile private var active = true

  fun stop() {
    active = false
  }

  @RequiresPermission(Manifest.permission.RECORD_AUDIO)
  suspend fun record(onProgress: suspend (seconds: Int, level: Float) -> Unit): ByteArray = withContext(Dispatchers.IO) {
    val frame = ShortArray(SAMPLE_RATE / 20)
    val minBuffer = AudioRecord.getMinBufferSize(SAMPLE_RATE, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT)
    val audio = AudioRecord(
      MediaRecorder.AudioSource.VOICE_RECOGNITION,
      SAMPLE_RATE,
      AudioFormat.CHANNEL_IN_MONO,
      AudioFormat.ENCODING_PCM_16BIT,
      maxOf(minBuffer, frame.size * 4),
    )
    val pcm = ByteArrayOutputStream()
    val bytes = ByteBuffer.allocate(frame.size * 2).order(ByteOrder.LITTLE_ENDIAN)
    try {
      check(audio.state == AudioRecord.STATE_INITIALIZED) { "Microphone unavailable" }
      audio.startRecording()
      while (active && isActive && pcm.size() < MAX_PCM_BYTES) {
        val read = audio.read(frame, 0, minOf(frame.size, (MAX_PCM_BYTES - pcm.size()) / 2))
        check(read >= 0) { "Microphone unavailable" }
        bytes.clear()
        var sum = 0.0
        for (i in 0 until read) {
          bytes.putShort(frame[i])
          sum += frame[i].toDouble() * frame[i]
        }
        pcm.write(bytes.array(), 0, read * 2)
        val decibels = 20 * log10(sqrt(sum / maxOf(read, 1)) / Short.MAX_VALUE + 1e-9)
        withContext(Dispatchers.Main.immediate) {
          onProgress(pcm.size() / 2 / SAMPLE_RATE, ((decibels + 50) / 50).toFloat().coerceIn(0f, 1f))
        }
      }
    } finally {
      try {
        if (audio.recordingState == AudioRecord.RECORDSTATE_RECORDING) audio.stop()
      } finally {
        audio.release()
      }
    }
    pcm.toByteArray()
  }
}

fun wav(pcm: ByteArray): ByteArray = ByteBuffer.allocate(WAV_HEADER + pcm.size).order(ByteOrder.LITTLE_ENDIAN).run {
  put("RIFF".toByteArray()).putInt(36 + pcm.size).put("WAVE".toByteArray())
  put("fmt ".toByteArray()).putInt(16).putShort(1).putShort(1).putInt(SAMPLE_RATE).putInt(SAMPLE_RATE * 2).putShort(2).putShort(16)
  put("data".toByteArray()).putInt(pcm.size).put(pcm)
  array()
}

/** At least a quarter second, so a stray tap does not spend a request. */
fun ByteArray.isLongEnough() = size >= SAMPLE_RATE / 2
