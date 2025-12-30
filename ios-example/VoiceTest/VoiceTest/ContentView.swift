import SwiftUI
import AVFoundation

struct ContentView: View {
    @StateObject private var voiceCapture = VoiceCapture()
    
    var body: some View {
        VStack(spacing: 30) {
            Text("Voice Capture Test")
                .font(.largeTitle)
            
            VStack {
                Text("Audio Level")
                    .font(.headline)
                GeometryReader { geometry in
                    ZStack(alignment: .leading) {
                        Rectangle()
                            .fill(Color.gray.opacity(0.3))
                        Rectangle()
                            .fill(voiceCapture.audioLevel > 0.5 ? Color.red : Color.green)
                            .frame(width: geometry.size.width * CGFloat(voiceCapture.audioLevel))
                    }
                }
                .frame(height: 30)
                .cornerRadius(5)
            }
            .padding(.horizontal)
            
            Text("Sample Rate: \(voiceCapture.sampleRate) Hz")
                .font(.subheadline)
            Text("Samples Received: \(voiceCapture.samplesReceived)")
                .font(.subheadline)
            
            Button(action: {
                if voiceCapture.isCapturing {
                    voiceCapture.stop()
                } else {
                    voiceCapture.start()
                }
            }) {
                Text(voiceCapture.isCapturing ? "Stop Capture" : "Start Capture")
                    .font(.title2)
                    .padding()
                    .frame(maxWidth: .infinity)
                    .background(voiceCapture.isCapturing ? Color.red : Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(10)
            }
            .padding(.horizontal)
            
            if let error = voiceCapture.error {
                Text("Error: \(error)")
                    .foregroundColor(.red)
                    .font(.caption)
            }
            
            Spacer()
        }
        .padding()
        .onAppear {
            requestMicrophonePermission()
        }
    }
    
    private func requestMicrophonePermission() {
        AVAudioSession.sharedInstance().requestRecordPermission { granted in
            if !granted {
                DispatchQueue.main.async {
                    voiceCapture.error = "Microphone permission denied"
                }
            }
        }
    }
}

#Preview {
    ContentView()
}
