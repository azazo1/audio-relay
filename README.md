Windows static build:
vcpkg install opus:x64-windows-static
set VCPKG_DEFAULT_TRIPLET=x64-windows-static

If you set OPUS_LIB_DIR or OPUS_DIR manually, point it at a static Opus library directory.
Set OPUS_STATIC=0 to fall back to the previous dynamic-link preference.
