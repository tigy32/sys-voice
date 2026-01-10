import 'dart:async';
import 'dart:math';

import 'package:flutter/material.dart';

// flutter_rust_bridge generated imports
import 'src/rust/api/voice_capture.dart';
import 'src/rust/frb_generated.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const VoiceCaptureApp());
}

class VoiceCaptureApp extends StatelessWidget {
  const VoiceCaptureApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Voice Capture',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.blue),
        useMaterial3: true,
      ),
      home: const VoiceCaptureScreen(),
    );
  }
}

class VoiceCaptureScreen extends StatefulWidget {
  const VoiceCaptureScreen({super.key});

  @override
  State<VoiceCaptureScreen> createState() => _VoiceCaptureScreenState();
}

class _VoiceCaptureScreenState extends State<VoiceCaptureScreen> {
  bool _isCapturing = false;
  bool _isRecording = false;
  bool _isPlaying = false;
  double _recordingDuration = 0.0;
  double _audioLevel = 0.0;
  int _sampleRate = 0;
  int _samplesReceived = 0;
  List<double> _recordedSamples = [];
  int _recordedSampleRate = 0;
  String? _error;
  VoiceCaptureHandle? _captureHandle;

  Timer? _recordingTimer;
  Timer? _pollTimer;

  static const double _maxRecordingDuration = 10.0;
  static const int _defaultSampleRate = 16000;

  @override
  void initState() {
    super.initState();
    _initCapture();
  }

  @override
  void dispose() {
    _stopCapture();
    super.dispose();
  }

  bool get _canPlayRecording => !_isPlaying && _recordedSamples.isNotEmpty;

  bool get _canPlayTestTone => _isCapturing && !_isPlaying;

  bool get _canGenerateTestRecording => !_isPlaying;

  void _initCapture() async {
    setState(() {
      _error = null;
      _audioLevel = 0.0;
    });

    try {
      final handle = await startCapture(sampleRate: _defaultSampleRate);
      setState(() {
        _captureHandle = handle;
        _isCapturing = true;
        _sampleRate = handle.sampleRate();
        _recordedSampleRate = _sampleRate;
      });

      _pollTimer = Timer.periodic(const Duration(milliseconds: 10), (_) {
        _pollAudio();
      });
    } catch (e) {
      setState(() {
        _error = 'Failed to start capture: $e';
      });
    }
  }

  void _startRecording() {
    setState(() {
      _recordedSamples = [];
      _recordingDuration = 0.0;
      _samplesReceived = 0;
      _isRecording = true;
    });

    _recordingTimer = Timer.periodic(const Duration(milliseconds: 100), (_) {
      setState(() {
        _recordingDuration += 0.1;
      });
      if (_recordingDuration >= _maxRecordingDuration) {
        _stopRecording();
      }
    });
  }

  void _stopRecording() {
    _recordingTimer?.cancel();
    _recordingTimer = null;

    setState(() {
      _isRecording = false;
    });
  }

  void _stopCapture() {
    _stopRecording();
    _pollTimer?.cancel();
    _pollTimer = null;

    _captureHandle = null;

    setState(() {
      _isCapturing = false;
    });
  }

  void _pollAudio() {
    final handle = _captureHandle;
    if (handle == null) return;

    try {
      final result = handle.pollAudio();
      if (result != null) {
        final samples = result.samples.map((e) => e.toDouble()).toList();

        double maxLevel = 0.0;
        for (final sample in samples) {
          final abs = sample.abs();
          if (abs > maxLevel) maxLevel = abs;
        }

        setState(() {
          _samplesReceived += samples.length;
          _audioLevel = maxLevel.clamp(0.0, 1.0);
          if (_isRecording) {
            _recordedSamples.addAll(samples);
          }
        });
      }
    } catch (e) {
      setState(() {
        _error = 'Poll error: $e';
      });
    }
  }

  void _playRecording() async {
    if (_recordedSamples.isEmpty || _recordedSampleRate <= 0) {
      setState(() {
        _error =
            'No recorded audio (count=${_recordedSamples.length}, rate=$_recordedSampleRate)';
      });
      return;
    }

    final handle = _captureHandle;
    if (handle == null) {
      setState(() {
        _error = 'Playback requires active capture session';
      });
      return;
    }

    setState(() {
      _isPlaying = true;
      _error = null;
    });

    try {
      double maxAmp = 0.0;
      for (final sample in _recordedSamples) {
        final abs = sample.abs();
        if (abs > maxAmp) maxAmp = abs;
      }

      List<double> playbackSamples;
      if (maxAmp > 0.0001) {
        final gain = min(0.8 / maxAmp, 100.0);
        playbackSamples = _recordedSamples.map((s) => s * gain).toList();
      } else {
        playbackSamples = _recordedSamples;
      }

      handle.playAudio(
        samples: playbackSamples,
        sampleRate: _recordedSampleRate,
      );

      final durationMs =
          (playbackSamples.length / _recordedSampleRate * 1000).toInt() + 100;
      await Future.delayed(Duration(milliseconds: durationMs));
    } catch (e) {
      setState(() {
        _error = 'Playback error: $e';
      });
    } finally {
      setState(() {
        _isPlaying = false;
      });
    }
  }

  void _playTestTone() async {
    final handle = _captureHandle;
    if (handle == null) {
      setState(() {
        _error = 'Test tone requires active capture session for AEC';
      });
      return;
    }

    setState(() {
      _isPlaying = true;
      _error = null;
    });

    try {
      final sampleRate = _sampleRate > 0 ? _sampleRate : _defaultSampleRate;
      const frequency = 440.0;
      const duration = 2.0;
      final sampleCount = (sampleRate * duration).toInt();

      final samples = List<double>.generate(sampleCount, (i) {
        final t = i / sampleRate;
        return sin(2.0 * pi * frequency * t) * 0.5;
      });

      handle.playAudio(samples: samples, sampleRate: sampleRate);

      await Future.delayed(Duration(milliseconds: (duration * 1000).toInt() + 100));
    } catch (e) {
      setState(() {
        _error = 'Test tone error: $e';
      });
    } finally {
      setState(() {
        _isPlaying = false;
      });
    }
  }

  void _generateTestRecording() {
    const sampleRate = 16000;
    const frequency1 = 440.0;
    const frequency2 = 660.0;
    const duration = 2.0;
    final sampleCount = (sampleRate * duration).toInt();

    final samples = List<double>.generate(sampleCount, (i) {
      final t = i / sampleRate;
      return sin(2.0 * pi * frequency1 * t) * 0.3 +
          sin(2.0 * pi * frequency2 * t) * 0.2;
    });

    setState(() {
      _recordedSamples = samples;
      _recordedSampleRate = sampleRate;
      _error = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Voice Capture Test'),
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const SizedBox(height: 16),
            const Text(
              'Audio Level',
              style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 8),
            _buildAudioLevelMeter(),
            const SizedBox(height: 24),
            Text(
              'Sample Rate: $_sampleRate Hz',
              style: const TextStyle(fontSize: 14),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 8),
            Text(
              'Samples Received: $_samplesReceived',
              style: const TextStyle(fontSize: 14),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 8),
            Text(
              _isCapturing ? 'Listening...' : 'Not listening',
              style: TextStyle(
                fontSize: 14,
                color: _isCapturing ? Colors.green : Colors.grey,
              ),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 8),
            if (_isRecording)
              Text(
                'Recording: ${_recordingDuration.toStringAsFixed(1)}s / ${_maxRecordingDuration.toStringAsFixed(0)}s',
                style: const TextStyle(fontSize: 14, color: Colors.red),
                textAlign: TextAlign.center,
              )
            else if (_recordedSamples.isNotEmpty)
              Text(
                'Recorded: ${(_recordedSamples.length / _recordedSampleRate).toStringAsFixed(1)}s',
                style: const TextStyle(fontSize: 14),
                textAlign: TextAlign.center,
              ),
            const SizedBox(height: 24),
            _buildRecordButton(),
            const SizedBox(height: 16),
            Row(
              children: [
                Expanded(child: _buildPlayRecordingButton()),
                const SizedBox(width: 16),
                Expanded(child: _buildTestToneButton()),
              ],
            ),
            const SizedBox(height: 16),
            _buildGenerateTestRecordingButton(),
            if (_error != null) ...[
              const SizedBox(height: 24),
              Text(
                'Error: $_error',
                style: const TextStyle(color: Colors.red, fontSize: 12),
                textAlign: TextAlign.center,
              ),
            ],
            const SizedBox(height: 24),
            Text(
              'Recorded: ${_recordedSamples.length} samples @ ${_recordedSampleRate}Hz',
              style: const TextStyle(color: Colors.grey, fontSize: 12),
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildAudioLevelMeter() {
    return Container(
      height: 30,
      decoration: BoxDecoration(
        color: Colors.grey.shade300,
        borderRadius: BorderRadius.circular(5),
      ),
      child: FractionallySizedBox(
        alignment: Alignment.centerLeft,
        widthFactor: _audioLevel,
        child: Container(
          decoration: BoxDecoration(
            color: _audioLevel >= 0.5 ? Colors.red : Colors.green,
            borderRadius: BorderRadius.circular(5),
          ),
        ),
      ),
    );
  }

  Widget _buildRecordButton() {
    final enabled = _isCapturing && !_isPlaying;
    return ElevatedButton(
      onPressed: enabled
          ? () {
              if (_isRecording) {
                _stopRecording();
              } else {
                _startRecording();
              }
            }
          : null,
      style: ElevatedButton.styleFrom(
        backgroundColor: _isRecording ? Colors.red : Colors.blue,
        foregroundColor: Colors.white,
        padding: const EdgeInsets.symmetric(vertical: 16),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
      ),
      child: Text(
        _isRecording ? 'Stop Recording' : 'Start Recording',
        style: const TextStyle(fontSize: 18),
      ),
    );
  }

  Widget _buildPlayRecordingButton() {
    return ElevatedButton(
      onPressed: _canPlayRecording ? _playRecording : null,
      style: ElevatedButton.styleFrom(
        backgroundColor: _canPlayRecording ? Colors.green : Colors.grey,
        foregroundColor: Colors.white,
        padding: const EdgeInsets.symmetric(vertical: 16),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
      ),
      child: const Text('Play Recording', style: TextStyle(fontSize: 16)),
    );
  }

  Widget _buildTestToneButton() {
    return ElevatedButton(
      onPressed: _canPlayTestTone ? _playTestTone : null,
      style: ElevatedButton.styleFrom(
        backgroundColor: _canPlayTestTone ? Colors.orange : Colors.grey,
        foregroundColor: Colors.white,
        padding: const EdgeInsets.symmetric(vertical: 16),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
      ),
      child: const Text('Test Tone', style: TextStyle(fontSize: 16)),
    );
  }

  Widget _buildGenerateTestRecordingButton() {
    return ElevatedButton(
      onPressed: _canGenerateTestRecording ? _generateTestRecording : null,
      style: ElevatedButton.styleFrom(
        backgroundColor: _canGenerateTestRecording ? Colors.purple : Colors.grey,
        foregroundColor: Colors.white,
        padding: const EdgeInsets.symmetric(vertical: 16),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
      ),
      child: const Text(
        'Generate Test Recording',
        style: TextStyle(fontSize: 16),
      ),
    );
  }
}
