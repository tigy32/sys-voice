package com.example.sysvoice

import android.Manifest
import android.content.pm.PackageManager
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.os.Bundle
import android.util.Log
import android.widget.Button
import android.widget.TextView
import android.widget.LinearLayout
import android.view.Gravity
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import kotlin.math.sqrt
import java.io.File
import java.io.FileOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder

class MainActivity : AppCompatActivity() {
    companion object {
        private const val TAG = "MainActivity"
    }
    
    private var isRecording = false
    private var isPlaying = false
    private lateinit var statusText: TextView
    private lateinit var levelText: TextView
    private lateinit var recordButton: Button
    private lateinit var playButton: Button
    
    private val recordedSamples = mutableListOf<Float>()
    private var sampleRate = 16000
    private var audioTrack: AudioTrack? = null
    private lateinit var testToneButton: Button

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val layout = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setPadding(32, 32, 32, 32)
        }

        statusText = TextView(this).apply {
            text = "Ready"
            textSize = 24f
            gravity = Gravity.CENTER
        }

        levelText = TextView(this).apply {
            text = "Level: --"
            textSize = 18f
            gravity = Gravity.CENTER
        }

        recordButton = Button(this).apply {
            text = "Record 10 Seconds"
            setOnClickListener { startRecording() }
        }
        
        playButton = Button(this).apply {
            text = "Play Recording"
            isEnabled = false
            setOnClickListener { playRecording() }
        }
        
        testToneButton = Button(this).apply {
            text = "Test Tone (440Hz)"
            setOnClickListener { playTestTone() }
        }

        layout.addView(statusText)
        layout.addView(levelText)
        layout.addView(recordButton)
        layout.addView(playButton)
        layout.addView(testToneButton)

        setContentView(layout)

        requestMicPermission()
    }

    private fun requestMicPermission() {
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO)
            != PackageManager.PERMISSION_GRANTED) {
            ActivityCompat.requestPermissions(this,
                arrayOf(Manifest.permission.RECORD_AUDIO), 1)
        }
    }

    private fun startRecording() {
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO)
            != PackageManager.PERMISSION_GRANTED) {
            statusText.text = "Microphone permission required"
            requestMicPermission()
            return
        }
        
        if (isRecording) return
        
        recordedSamples.clear()
        playButton.isEnabled = false
        
        val targetSampleRate = 16000
        val success = VoiceCapture.start(targetSampleRate) { samples ->
            synchronized(recordedSamples) {
                recordedSamples.addAll(samples.toList())
            }
            
            val rms = sqrt(samples.map { it * it }.average().toFloat())
            val actualRate = VoiceCapture.getSampleRate()
            val seconds = recordedSamples.size / actualRate.toFloat()
            
            runOnUiThread {
                levelText.text = "Level: %.4f (%.1fs)".format(rms, seconds)
                
                if (seconds >= 10f) {
                    stopRecording()
                }
            }
        }

        if (success) {
            isRecording = true
            sampleRate = VoiceCapture.getSampleRate()
            recordButton.isEnabled = false
            statusText.text = "Recording at ${sampleRate}Hz..."
        } else {
            statusText.text = "Failed to start"
        }
    }
    
    private fun stopRecording() {
        if (!isRecording) return
        
        VoiceCapture.stop()
        isRecording = false
        recordButton.isEnabled = true
        
        val seconds = recordedSamples.size / sampleRate.toFloat()
        statusText.text = "Recorded %.1f seconds".format(seconds)
        levelText.text = "Level: --"
        playButton.isEnabled = recordedSamples.isNotEmpty()
        
        // Save WAV file for debugging
        saveWavFile()
    }
    
    private fun saveWavFile() {
        if (recordedSamples.isEmpty()) return
        
        val floatSamples: FloatArray
        synchronized(recordedSamples) {
            floatSamples = recordedSamples.toFloatArray()
        }
        
        // Convert to 16-bit PCM
        val shortSamples = ShortArray(floatSamples.size) { i ->
            (floatSamples[i] * 32767f).toInt().coerceIn(-32768, 32767).toShort()
        }
        
        val wavFile = File(getExternalFilesDir(null), "recording.wav")
        try {
            FileOutputStream(wavFile).use { fos ->
                val dataSize = shortSamples.size * 2
                val fileSize = 36 + dataSize
                
                // WAV header
                val header = ByteBuffer.allocate(44).apply {
                    order(ByteOrder.LITTLE_ENDIAN)
                    put("RIFF".toByteArray())
                    putInt(fileSize)
                    put("WAVE".toByteArray())
                    put("fmt ".toByteArray())
                    putInt(16)  // subchunk1 size
                    putShort(1) // PCM format
                    putShort(1) // mono
                    putInt(sampleRate)
                    putInt(sampleRate * 2) // byte rate
                    putShort(2) // block align
                    putShort(16) // bits per sample
                    put("data".toByteArray())
                    putInt(dataSize)
                }
                fos.write(header.array())
                
                // Audio data
                val dataBuffer = ByteBuffer.allocate(dataSize).apply {
                    order(ByteOrder.LITTLE_ENDIAN)
                    shortSamples.forEach { putShort(it) }
                }
                fos.write(dataBuffer.array())
            }
            
            Log.d(TAG, "WAV saved: ${wavFile.absolutePath}")
            Log.d(TAG, "Pull with: adb pull ${wavFile.absolutePath}")
            runOnUiThread {
                statusText.text = "Saved: ${wavFile.name}"
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save WAV", e)
        }
    }
    
    private fun playRecording() {
        if (isPlaying || recordedSamples.isEmpty()) return
        
        val floatSamples: FloatArray
        synchronized(recordedSamples) {
            floatSamples = recordedSamples.toFloatArray()
        }
        
        // Check sample statistics to verify we have actual audio
        var minVal = Float.MAX_VALUE
        var maxVal = Float.MIN_VALUE
        var nonZeroCount = 0
        for (s in floatSamples) {
            if (s < minVal) minVal = s
            if (s > maxVal) maxVal = s
            if (s != 0f) nonZeroCount++
        }
        Log.d(TAG, "playRecording: ${floatSamples.size} samples at ${sampleRate}Hz")
        Log.d(TAG, "Sample stats: min=$minVal, max=$maxVal, nonZero=$nonZeroCount/${floatSamples.size}")
        
        // Convert float [-1.0, 1.0] to 16-bit PCM (universally supported)
        val shortSamples = ShortArray(floatSamples.size) { i ->
            (floatSamples[i] * 32767f).toInt().coerceIn(-32768, 32767).toShort()
        }
        
        isPlaying = true
        playButton.isEnabled = false
        recordButton.isEnabled = false
        statusText.text = "Playing..."
        
        Thread {
            try {
                val bufferSize = AudioTrack.getMinBufferSize(
                    sampleRate,
                    AudioFormat.CHANNEL_OUT_MONO,
                    AudioFormat.ENCODING_PCM_16BIT
                )
                Log.d(TAG, "AudioTrack minBufferSize: $bufferSize, samples: ${shortSamples.size}")
                
                if (bufferSize <= 0) {
                    Log.e(TAG, "Invalid buffer size: $bufferSize")
                    throw IllegalStateException("getMinBufferSize returned $bufferSize")
                }
                
                // Use MODE_STREAM for better emulator compatibility
                val track = AudioTrack.Builder()
                    .setAudioAttributes(
                        AudioAttributes.Builder()
                            .setUsage(AudioAttributes.USAGE_MEDIA)
                            .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
                            .build()
                    )
                    .setAudioFormat(
                        AudioFormat.Builder()
                            .setSampleRate(sampleRate)
                            .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                            .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                            .build()
                    )
                    .setBufferSizeInBytes(bufferSize * 4)
                    .setTransferMode(AudioTrack.MODE_STREAM)
                    .build()
                
                audioTrack = track
                Log.d(TAG, "AudioTrack created, state: ${track.state}")
                
                track.play()
                Log.d(TAG, "AudioTrack play() called, playState: ${track.playState}")
                
                // Write in chunks for MODE_STREAM
                val chunkSize = bufferSize / 2  // Write half buffer at a time
                var offset = 0
                var totalWritten = 0
                while (offset < shortSamples.size) {
                    val toWrite = minOf(chunkSize, shortSamples.size - offset)
                    val written = track.write(shortSamples, offset, toWrite, AudioTrack.WRITE_BLOCKING)
                    if (written < 0) {
                        Log.e(TAG, "Write error at offset $offset: $written")
                        break
                    }
                    totalWritten += written
                    offset += written
                    
                    // Update UI with playback progress
                    val progress = offset.toFloat() / shortSamples.size
                    runOnUiThread {
                        levelText.text = "Playing: %.0f%%".format(progress * 100)
                    }
                }
                Log.d(TAG, "AudioTrack total written: $totalWritten")
                
                // Wait for playback to finish (drain the buffer)
                Thread.sleep(500)
                
                track.stop()
                track.release()
                audioTrack = null
                Log.d(TAG, "AudioTrack released")
                
            } catch (e: Exception) {
                Log.e(TAG, "Playback error", e)
            } finally {
                runOnUiThread {
                    isPlaying = false
                    playButton.isEnabled = true
                    recordButton.isEnabled = true
                    statusText.text = "Playback complete"
                    levelText.text = "Level: --"
                }
            }
        }.start()
    }

    private fun playTestTone() {
        if (isPlaying) return
        
        isPlaying = true
        testToneButton.isEnabled = false
        recordButton.isEnabled = false
        playButton.isEnabled = false
        statusText.text = "Playing 440Hz test tone..."
        
        Thread {
            try {
                val testSampleRate = 44100
                val durationSec = 2
                val frequency = 440.0
                val numSamples = testSampleRate * durationSec
                
                // Generate sine wave
                val samples = ShortArray(numSamples) { i ->
                    val t = i.toDouble() / testSampleRate
                    val value = kotlin.math.sin(2.0 * Math.PI * frequency * t)
                    (value * 32767 * 0.5).toInt().toShort()  // 50% volume
                }
                
                Log.d(TAG, "Generated ${samples.size} samples of 440Hz tone")
                
                val bufferSize = AudioTrack.getMinBufferSize(
                    testSampleRate,
                    AudioFormat.CHANNEL_OUT_MONO,
                    AudioFormat.ENCODING_PCM_16BIT
                )
                
                val track = AudioTrack.Builder()
                    .setAudioAttributes(
                        AudioAttributes.Builder()
                            .setUsage(AudioAttributes.USAGE_MEDIA)
                            .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                            .build()
                    )
                    .setAudioFormat(
                        AudioFormat.Builder()
                            .setSampleRate(testSampleRate)
                            .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                            .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                            .build()
                    )
                    .setBufferSizeInBytes(bufferSize * 4)
                    .setTransferMode(AudioTrack.MODE_STREAM)
                    .build()
                
                Log.d(TAG, "Test tone AudioTrack state: ${track.state}")
                track.play()
                
                val written = track.write(samples, 0, samples.size, AudioTrack.WRITE_BLOCKING)
                Log.d(TAG, "Test tone written: $written samples")
                
                Thread.sleep(500)  // Let buffer drain
                track.stop()
                track.release()
                Log.d(TAG, "Test tone complete")
                
            } catch (e: Exception) {
                Log.e(TAG, "Test tone error", e)
            } finally {
                runOnUiThread {
                    isPlaying = false
                    testToneButton.isEnabled = true
                    recordButton.isEnabled = true
                    playButton.isEnabled = recordedSamples.isNotEmpty()
                    statusText.text = "Test tone finished"
                }
            }
        }.start()
    }
    
    override fun onDestroy() {
        super.onDestroy()
        if (isRecording) {
            VoiceCapture.stop()
        }
        audioTrack?.release()
    }
}
