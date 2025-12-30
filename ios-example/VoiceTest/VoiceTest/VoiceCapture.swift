import Foundation
import Combine

class VoiceCapture: ObservableObject {
    @Published var isCapturing = false
    @Published var audioLevel: Float = 0
    @Published var sampleRate: UInt32 = 0
    @Published var samplesReceived: Int = 0
    @Published var error: String?
    
    private var handle: UnsafeMutableRawPointer?
    private var pollTimer: Timer?
    private var audioBuffer = [Float](repeating: 0, count: 4096)
    
    func start(sampleRate: UInt32 = 16000) {
        guard handle == nil else { return }
        
        handle = capture_start(sampleRate)
        if handle == nil {
            error = "Failed to start capture"
            return
        }
        
        self.sampleRate = capture_sample_rate(handle)
        isCapturing = true
        error = nil
        samplesReceived = 0
        
        pollTimer = Timer.scheduledTimer(withTimeInterval: 0.01, repeats: true) { [weak self] _ in
            self?.pollAudio()
        }
    }
    
    func stop() {
        pollTimer?.invalidate()
        pollTimer = nil
        
        if let handle = handle {
            capture_stop(handle)
            self.handle = nil
        }
        
        isCapturing = false
        audioLevel = 0
    }
    
    private func pollAudio() {
        guard let handle = handle else { return }
        
        let count = capture_recv(handle, &audioBuffer, audioBuffer.count)
        
        if count > 0 {
            let samples = Array(audioBuffer.prefix(Int(count)))
            samplesReceived += samples.count
            
            let rms = sqrt(samples.map { $0 * $0 }.reduce(0, +) / Float(samples.count))
            
            DispatchQueue.main.async {
                self.audioLevel = min(rms * 10, 1.0)
            }
        } else if count == -2 {
            DispatchQueue.main.async {
                self.error = "Capture error"
                self.stop()
            }
        }
    }
    
    deinit {
        stop()
    }
}
