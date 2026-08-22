# Third-Party Notices

MediaForge source code is licensed under the MIT License in `LICENSE`. The Windows distribution also contains separately extracted third-party executables under their own terms.

## FFmpeg and FFprobe

- Component: FFmpeg and FFprobe 8.0.1 essentials build
- Distributor: [Gyan Doshi's Windows builds](https://www.gyan.dev/ffmpeg/builds/)
- Upstream source: [FFmpeg n8.0.1](https://github.com/FFmpeg/FFmpeg/tree/n8.0.1)
- License: GNU General Public License version 3 (GPLv3) for this build configuration
- License text: [FFmpeg `COPYING.GPLv3`](https://github.com/FFmpeg/FFmpeg/blob/n8.0.1/COPYING.GPLv3)

The bundled tools identify themselves with `--enable-gpl --enable-version3`. They are executed as separate programs after extraction; MediaForge does not modify them.

Bundled executable SHA-256 digests:

- `bin/ffmpeg.exe`: `5af82a0d4fe2b9eae211b967332ea97edfc51c6b328ca35b827e73eac560dc0d`
- `bin/ffprobe.exe`: `192a1d6899059765ac8c39764fc3148d4e6049955956dc2029f81f4bd6a8972d`

FFmpeg includes additional third-party libraries; its full configuration is available with `ffmpeg -version`. See the upstream FFmpeg license documentation and the Gyan build documentation for the applicable component details.
