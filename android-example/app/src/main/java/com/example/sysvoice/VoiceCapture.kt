package com.example.sysvoice

import android.util.Log

object VoiceCapture {
    private const val TAG = "VoiceCapture"
    
    init {
        Log.d(TAG, "Loading native library...")
        System.loadLibrary("sys_voice_android_ffi")
        Log.d(TAG, "Native library loaded")
    }

    external fun nativeStart(sampleRate: Int): Int
    external fun nativeRecv(buffer: FloatArray): Int
    external fun nativeGetSampleRate(): Int
    external fun nativeStop()

    private var isCapturing = false
    private var listener: ((FloatArray) -> Unit)? = null

    fun start(sampleRate: Int = 16000, onSamples: (FloatArray) -> Unit): Boolean {
        Log.d(TAG, "start() called with sampleRate=$sampleRate")
        if (isCapturing) {
            Log.w(TAG, "Already capturing")
            return false
        }

        Log.d(TAG, "Calling nativeStart...")
        val result = nativeStart(sampleRate)
        Log.d(TAG, "nativeStart returned: $result")
        if (result != 0) {
            Log.e(TAG, "nativeStart failed with code: $result")
            return false
        }

        isCapturing = true
        listener = onSamples

        Thread {
            Log.d(TAG, "Polling thread started")
            val buffer = FloatArray(4096)
            var pollCount = 0
            while (isCapturing) {
                val count = nativeRecv(buffer)
                pollCount++
                if (pollCount <= 10 || pollCount % 100 == 0) {
                    Log.d(TAG, "nativeRecv[$pollCount] returned: $count")
                }
                if (count > 0) {
                    listener?.invoke(buffer.copyOf(count))
                } else if (count < 0) {
                    Log.e(TAG, "nativeRecv error: $count")
                    break
                }
                Thread.sleep(10)
            }
            Log.d(TAG, "Polling thread exiting after $pollCount polls")
        }.start()

        return true
    }

    fun getSampleRate(): Int = nativeGetSampleRate()

    fun stop() {
        isCapturing = false
        listener = null
        nativeStop()
    }
}
