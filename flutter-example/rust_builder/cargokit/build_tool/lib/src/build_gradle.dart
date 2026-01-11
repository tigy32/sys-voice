/// This is copied from Cargokit (which is the official way to use it currently)
/// Details: https://fzyzcjy.github.io/flutter_rust_bridge/manual/integrate/builtin

import 'dart:io';

import 'package:logging/logging.dart';
import 'package:path/path.dart' as path;

import 'artifacts_provider.dart';
import 'builder.dart';
import 'environment.dart';
import 'options.dart';
import 'target.dart';

final log = Logger('build_gradle');

String _getHostArch() {
  if (Platform.isMacOS) {
    return 'darwin-x86_64';
  } else if (Platform.isLinux) {
    return 'linux-x86_64';
  } else if (Platform.isWindows) {
    return 'windows-x86_64';
  }
  throw Exception('Unsupported host platform');
}

String _getNdkLibraryTriple(Target target) {
  switch (target.rust) {
    case 'aarch64-linux-android':
      return 'aarch64-linux-android';
    case 'armv7-linux-androideabi':
      return 'arm-linux-androideabi';
    case 'i686-linux-android':
      return 'i686-linux-android';
    case 'x86_64-linux-android':
      return 'x86_64-linux-android';
    default:
      throw Exception('Unknown Android target: ${target.rust}');
  }
}

class BuildGradle {
  BuildGradle({required this.userOptions});

  final CargokitUserOptions userOptions;

  Future<void> build() async {
    final targets = Environment.targetPlatforms.map((arch) {
      final target = Target.forFlutterName(arch);
      if (target == null) {
        throw Exception(
            "Unknown darwin target or platform: $arch, ${Environment.darwinPlatformName}");
      }
      return target;
    }).toList();

    final environment = BuildEnvironment.fromEnvironment(isAndroid: true);
    final provider =
        ArtifactProvider(environment: environment, userOptions: userOptions);
    final artifacts = await provider.getArtifacts(targets);

    for (final target in targets) {
      final libs = artifacts[target]!;
      final outputDir = path.join(Environment.outputDir, target.android!);
      Directory(outputDir).createSync(recursive: true);

      for (final lib in libs) {
        if (lib.type == AritifactType.dylib) {
          File(lib.path).copySync(path.join(outputDir, lib.finalFileName));
        }
      }

      // Copy libc++_shared.so from NDK for C++ runtime support (required by oboe)
      final ndkPath = path.join(Environment.sdkPath, 'ndk', Environment.ndkVersion);
      final hostArch = _getHostArch();
      final ndkTriple = _getNdkLibraryTriple(target);
      final ndkToolchainPath = path.join(ndkPath, 'toolchains', 'llvm', 'prebuilt', hostArch);
      final libcppPath = path.join(ndkToolchainPath, 'sysroot', 'usr', 'lib', ndkTriple, 'libc++_shared.so');
      final libcppFile = File(libcppPath);
      if (libcppFile.existsSync()) {
        libcppFile.copySync(path.join(outputDir, 'libc++_shared.so'));
        log.info('Copied libc++_shared.so for ${target.rust}');
      } else {
        log.warning('libc++_shared.so not found at $libcppPath');
      }
    }
  }
}
