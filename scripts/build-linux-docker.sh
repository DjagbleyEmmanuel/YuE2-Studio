#!/bin/bash
# The Linux x86-64 .deb and AppImage, built on this machine in an Ubuntu 22.04 container (glibc 2.35, so they run there and on anything newer; Docker under WSL on
# Windows) from the committed HEAD. Only the .dmg is built on GitHub (release-unix.yml): there is no Mac here.
#   scripts/build-linux-docker.sh <output folder> [cache folder]
# The cache keeps Rust, Node, the cargo target and the engine build between runs.
set -euo pipefail

if [ "${1:-}" = "--inside" ]; then
  export DEBIAN_FRONTEND=noninteractive APPIMAGE_EXTRACT_AND_RUN=1 RUSTUP_TOOLCHAIN=stable CARGO_TERM_COLOR=never
  export CARGO_HOME=/cache/cargo RUSTUP_HOME=/cache/rustup npm_config_cache=/cache/npm PATH=/cache/cargo/bin:/cache/node/bin:$PATH
  # this machine's DNS answers AAAA first and the IPv6 routes go nowhere: apt would wait on them forever
  printf 'Acquire::ForceIPv4 "true";\nAcquire::Retries "5";\n' > /etc/apt/apt.conf.d/99ipv4
  echo "[..] packages"
  apt-get update -q >/dev/null
  apt-get install -y -q --no-install-recommends -o Dir::Cache::Archives=/cache/apt build-essential pkg-config cmake ninja-build nasm autoconf automake libtool \
    curl ca-certificates git unzip xz-utils libssl-dev libasound2-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
    librsvg2-dev patchelf file libfuse2 gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-libav libvulkan-dev >/dev/null
  if [ ! -x /cache/node/bin/node ]; then
    echo "[..] node 22"
    v=$(curl -s https://nodejs.org/dist/latest-v22.x/ | grep -o 'node-v22[0-9.]*-linux-x64.tar.xz' | head -1)
    mkdir -p /cache/node && curl -sL "https://nodejs.org/dist/latest-v22.x/$v" | tar -xJ -C /cache/node --strip-components=1
  fi
  if [ ! -x /cache/cargo/bin/cargo ]; then
    echo "[..] rust stable"
    curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable >/dev/null
  fi
  # Ubuntu 22.04 has no glslc and older Vulkan headers than the engine's ggml wants: LunarG's SDK has both
  vulkan=1.4.363.0
  if [ ! -x /cache/vulkansdk-$vulkan/x86_64/bin/glslc ]; then
    echo "[..] Vulkan SDK $vulkan"
    mkdir -p /cache/vulkansdk-$vulkan
    curl -sSfL "https://sdk.lunarg.com/sdk/download/$vulkan/linux/vulkansdk-linux-x86_64-$vulkan.tar.xz" | tar -xJ -C /cache/vulkansdk-$vulkan --strip-components=1
  fi
  export VULKAN_SDK=/cache/vulkansdk-$vulkan/x86_64 PATH=/cache/vulkansdk-$vulkan/x86_64/bin:$PATH
  # the engine's CUDA backends: 12.9 serves Maxwell, Pascal, Volta and every card CUDA 13 does; CUDA 13 is added only
  # when asked (YUE_CUDA13=1), and this fork's packages carry the CUDA 12 build alone.
  cuda=13-3
  nvidia_repo() {
    curl -sSfL -o /tmp/cuda-keyring.deb https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.1-1_all.deb
    dpkg -i /tmp/cuda-keyring.deb >/dev/null
    apt-get update -q >/dev/null
  }
  mkdir -p /cache/apt
  if [ "${YUE_CUDA13:-0}" = "1" ]; then
    echo "[..] CUDA $cuda"
    nvidia_repo
    apt-get install -y -q --no-install-recommends -o Acquire::Retries=5 -o Dir::Cache::Archives=/cache/apt cuda-nvcc-$cuda cuda-cudart-dev-$cuda libcublas-dev-$cuda >/dev/null
    export PATH=/usr/local/cuda-13.3/bin:$PATH
  else
    echo "[..] CUDA 13 skipped (YUE_CUDA13=0): the package carries the CUDA 12 build"
  fi
  # the CUDA 12 backend's toolkit (the audio.cpp server's CUDA 12 build is 12.9 too). 12.9's nvcc is needed rather than
  # an older CUDA 12 one: the backend records device code for sm_120a, the Blackwell cards, which 12.8 added. NVIDIA's
  # repository throttles to tens of kB/s, so its 900 MB of cuBLAS development packages are skipped when the caller
  # mounts a CUDA 12 toolkit at /cuda12-host: its cuBLAS headers and libraries serve instead, since the CUDA 12 ABI is
  # stable across minor versions and only the .so.12 sonames are recorded in the backend.
  export CUDA12_ROOT=/usr/local/cuda-12.9
  if [ -d /cuda12-host/targets/x86_64-linux/lib ]; then
    echo "[..] CUDA 12.9 from the package cache, cuBLAS from the mounted toolkit"
    # the two cuda-toolkit-*-config-common packages the cache lacks only carry apt metadata: their absence is forced
    dpkg -i --force-depends /cache/apt/cuda-*-12-9_*.deb /cache/apt/cuda-toolkit-config-common_*.deb >/dev/null 2>&1
    cp -a /cuda12-host/targets/x86_64-linux/include/cublas*.h "$CUDA12_ROOT/targets/x86_64-linux/include/"
    cp -a /cuda12-host/targets/x86_64-linux/lib/libcublas*.so* "$CUDA12_ROOT/targets/x86_64-linux/lib/"
    # the cuda-toolkit-config-common packages that would register this folder come from the same slow repository
    echo "$CUDA12_ROOT/targets/x86_64-linux/lib" > /etc/ld.so.conf.d/cuda-12-9.conf
    ldconfig
    [ -x "$CUDA12_ROOT/bin/nvcc" ] || { echo "[ERROR] the CUDA 12 compiler did not unpack"; exit 1; }
    ldconfig -p | grep -q 'libcudart.so.12' || { echo "[ERROR] the CUDA 12 runtime is missing"; exit 1; }
    ldconfig -p | grep -q 'libcublas.so.12' || { echo "[ERROR] the cuBLAS libraries are missing"; exit 1; }
  else
    echo "[..] CUDA 12.9 from the NVIDIA repository"
    nvidia_repo
    apt-get install -y -q --no-install-recommends -o Acquire::Retries=5 -o Dir::Cache::Archives=/cache/apt cuda-nvcc-12-9 cuda-cudart-dev-12-9 libcublas-dev-12-9 >/dev/null
  fi
  echo "[..] source $(git -c safe.directory=/src -C /src rev-parse --short HEAD)"
  rm -rf /cache/src && mkdir -p /cache/src
  git -c safe.directory=/src -C /src archive HEAD | tar -x -C /cache/src
  cd /cache/src
  export CARGO_TARGET_DIR=/cache/target
  ln -sfn /cache/target desktop/src-tauri/target
  echo "[..] build (log: build.log)"
  bash scripts/build-release-linux.sh > /out/build.log 2>&1 || { echo "[ERROR] build failed"; tail -40 /out/build.log; exit 1; }
  engine=desktop/src-tauri/resources/yue2-cpp/yue-server
  if ldd "$engine" | grep -i 'not found'; then echo "[ERROR] the engine misses libraries"; exit 1; fi
  # libcuda.so.1 comes with the display driver, which a build container has not got
  if [ "${YUE_CUDA13:-0}" = "1" ]; then
    backend=desktop/src-tauri/resources/yue2-cpp/cuda13/libggml-cuda.so
    [ -f "$backend" ] || { echo "[ERROR] the engine was built without its CUDA 13 backend"; exit 1; }
    if ldd "$backend" | grep -i 'not found' | grep -v 'libcuda.so'; then echo "[ERROR] the CUDA 13 backend misses libraries"; exit 1; fi
  fi
  backend12=desktop/src-tauri/resources/yue2-cpp/cuda12/libggml-cuda.so
  [ -f "$backend12" ] || { echo "[ERROR] the engine was built without its CUDA 12 backend"; exit 1; }
  if ldd "$backend12" | grep -i 'not found' | grep -v 'libcuda.so'; then echo "[ERROR] the CUDA 12 backend misses libraries"; exit 1; fi
  # the engine answers --help with its usage and a non-zero code
  "$engine" --help > /tmp/engine-help.txt 2>&1 || true
  grep -q 'Usage' /tmp/engine-help.txt || { echo "[ERROR] the engine does not start"; head -5 /tmp/engine-help.txt; exit 1; }
  midi=desktop/src-tauri/resources/music-midi/music-midi
  if ldd "$midi" | grep -i 'not found'; then echo "[ERROR] the MIDI transcriber misses libraries"; exit 1; fi
  caption=desktop/src-tauri/resources/music-midi/music-caption
  if ldd "$caption" | grep -i 'not found'; then echo "[ERROR] the captioner misses libraries"; exit 1; fi
  version="$(node -p "require('./desktop/src-tauri/tauri.conf.json').version")"
  product="$(node -p "require('./desktop/src-tauri/tauri.conf.json').productName")"
  # the cache may be shared with the other studios and keeps earlier versions: take this program's and this version's by full name
  deb="/cache/target/release/bundle/deb/${product}_${version}_amd64.deb"
  appimage="/cache/target/release/bundle/appimage/${product}_${version}_amd64.AppImage"
  [ -f "$deb" ] && [ -f "$appimage" ] || { echo "[ERROR] no ${product} ${version} package in the bundle folder"; ls -la /cache/target/release/bundle/deb /cache/target/release/bundle/appimage; exit 1; }
  # tauri-bundler names the package by heck::AsKebabCase(productName) and the menu entry and resource folder by productName
  package="$(node -p "process.argv[1].replace(/([a-z0-9])([A-Z])/g,'\$1 \$2').replace(/([A-Z]+)([A-Z][a-z])/g,'\$1 \$2').split(/[^A-Za-z0-9]+/).filter(Boolean).join('-').toLowerCase()" "$product")"
  found="$(dpkg-deb -f "$deb" Package)"
  [ "$found" = "$package" ] || { echo "[ERROR] $deb is the package $found, not $package"; exit 1; }
  listing="$(dpkg-deb -c "$deb")"
  grep -qF " usr/share/applications/${product}.desktop" <<< "$listing" || { echo "[ERROR] $deb has no ${product}.desktop"; exit 1; }
  grep -qF " usr/lib/${product}/resources/yue2-cpp/yue-server" <<< "$listing" || { echo "[ERROR] $deb carries no YuE2 engine"; exit 1; }
  rm -rf /tmp/appimage-check && mkdir -p /tmp/appimage-check
  # the .desktop at the image's root is a link into usr/share/applications: take the file it points to
  (cd /tmp/appimage-check && env -u APPIMAGE_EXTRACT_AND_RUN "$appimage" --appimage-extract 'usr/share/applications/*.desktop' >/dev/null)
  grep -qx "Name=${product}" /tmp/appimage-check/squashfs-root/usr/share/applications/*.desktop || { echo "[ERROR] $appimage is not ${product}"; exit 1; }
  echo "[OK] packages are ${product}: deb ${found}, AppImage $(basename /tmp/appimage-check/squashfs-root/usr/share/applications/*.desktop)"
  cp "$deb" "/out/YuE2-Studio-${version}-linux-amd64.deb"
  cp "$appimage" "/out/YuE2-Studio-${version}-linux-x86_64.AppImage"
  ls -la /out/*.deb /out/*.AppImage
  echo "[OK] YuE2-Studio ${version} for Linux"
  exit 0
fi

out="${1:?output folder}"
cache="${2:-$HOME/yue2-linux2204-cache}"
repo="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$out" "$cache"
cp "$repo/scripts/build-linux-docker.sh" "$out/.build-linux-docker.sh"
# A complete CUDA 12 toolkit on this machine saves the container from NVIDIA's throttled repository: the build needs
# only its cuBLAS headers and libraries, and 12.9's nvcc comes from the package cache.
cuda12_host="${YUE_CUDA12_HOST:-}"
if [ -z "$cuda12_host" ]; then
  for candidate in /usr/local/cuda-12.9 /usr/local/cuda-12; do
    if [ -x "$candidate/bin/nvcc" ]; then cuda12_host="$candidate"; break; fi
  done
fi
mounts=()
if [ -n "$cuda12_host" ]; then
  echo "[..] cuBLAS from $cuda12_host"
  mounts=(-v "$cuda12_host:/cuda12-host:ro")
fi
docker run --rm -e YUE_CUDA13="${YUE_CUDA13:-0}" -v "$repo:/src:ro" -v "$(cd "$out" && pwd):/out" -v "$cache:/cache" ${mounts[@]+"${mounts[@]}"} ubuntu:22.04 bash /out/.build-linux-docker.sh --inside
