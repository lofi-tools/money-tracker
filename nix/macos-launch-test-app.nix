# Manual acceptance fixture only. Production Brave keeps its original metadata.
{ runCommand, python3, stdenvNoCC, brave, profileDirectory }:
runCommand "nexo-brave-macos-launch-test" { nativeBuildInputs = [ python3 ]; } ''
  mkdir -p "$out/Applications"
  app="$out/Applications/Brave Nexo Launch Test.app"
  cp -R "${brave}/Applications/Brave Browser.app" "$app"
  chmod u+w "$app/Contents/MacOS" "$app/Contents/Info.plist" "$app/Contents/MacOS/Brave Browser"
  substituteInPlace "$app/Contents/MacOS/Brave Browser" \
    --replace-fail "${brave}/Applications/Brave Browser.app" "$app"
  python3 - "$app/Contents/Info.plist" <<'PY'
  import plistlib, sys
  path = sys.argv[1]
  with open(path, 'rb') as stream:
      info = plistlib.load(stream)
  info['CFBundleIdentifier'] = 'dev.money-tracker.BraveLaunchTest'
  info['CFBundleName'] = 'Brave Nexo Launch Test'
  info['CFBundleDisplayName'] = 'Brave Nexo Launch Test'
  with open(path, 'wb') as stream:
      plistlib.dump(info, stream)
  PY
  mv "$app/Contents/MacOS/Brave Browser" "$app/Contents/MacOS/nexo-test-entry"
  cat > "$app/Contents/MacOS/Brave Browser" <<EOF
  #!${stdenvNoCC.shell}
  exec "$app/Contents/MacOS/nexo-test-entry" \\
    '--user-data-dir=${profileDirectory}' --no-first-run --no-default-browser-check \\
    --use-mock-keychain --disable-sync "\$@"
  EOF
  chmod +x "$app/Contents/MacOS/Brave Browser"
''
