// Bridge.h - C FFI declarations for native-voice-io

#ifndef Bridge_h
#define Bridge_h

#include <stdint.h>
#include <stddef.h>

// Start audio capture. Returns opaque handle or NULL on failure.
void* capture_start(uint32_t sample_rate);

// Receive audio samples. Returns number of samples written to buffer.
// Returns 0 if no data available, -1 on invalid handle, -2 on capture error.
int32_t capture_recv(void* handle, float* buffer, size_t buffer_len);

// Get the sample rate of the capture handle.
uint32_t capture_sample_rate(void* handle);

// Stop capture and release all resources.
void capture_stop(void* handle);

#endif /* Bridge_h */
