{ lib, stdenvNoCC, python3, runCommand, writeShellScript, writeText, makeWrapper }:
let
  source = ../apps/nexo-capture-extension;
  manifest = builtins.fromJSON (builtins.readFile ../apps/nexo-capture-extension/manifest.json);
  extensionId = "agmnljgahcijdoahbejifmdcdpamijin";
  extensionSubdir = "share/brave/extensions/nexo-capture";
  # Native bridge setup is distinct from this hermetic static extension build.
  # These are explicit trusted runtime parameters, never build-time secret inputs.
  mkNativeHost = { hostExecutable, pairingFile }:
    let
      launcher = writeShellScript "nexo-capture-native-host" ''
        exec ${lib.escapeShellArg hostExecutable} --pairing ${lib.escapeShellArg pairingFile} "$@"
      '';
      manifest = writeText "dev.money_tracker.nexo_capture.json" (builtins.toJSON {
        name = "dev.money_tracker.nexo_capture";
        description = "Money Tracker Nexo capture";
        path = toString launcher;
        type = "stdio";
        allowed_origins = [ "chrome-extension://${extensionId}/" ];
      });
    in runCommand "nexo-capture-native-host" { } ''
      mkdir -p "$out/etc/chromium/native-messaging-hosts"
      ln -s ${manifest} "$out/etc/chromium/native-messaging-hosts/dev.money_tracker.nexo_capture.json"
    '';
  # Wrap the App's real entry point (not just bin/brave): Dock, Finder, URLs and
  # CLI then share one startup path. makeWrapper execs it and forwards "$@".
  wrapBrave = brave: brave.overrideAttrs (old: {
    nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [ makeWrapper ];
    postInstall = (old.postInstall or "") + lib.optionalString stdenvNoCC.hostPlatform.isDarwin ''
      wrapProgram "$out/Applications/Brave Browser.app/Contents/MacOS/Brave Browser" \
        --add-flags ${lib.escapeShellArg "--load-extension=${package}/${extensionSubdir}"}
    '' + lib.optionalString stdenvNoCC.hostPlatform.isLinux ''
      wrapProgram "$out/bin/brave" \
        --add-flags ${lib.escapeShellArg "--load-extension=${package}/${extensionSubdir}"}
    '';
  });
  package = stdenvNoCC.mkDerivation {
  pname = "nexo-capture-extension";
  version = manifest.version;
  # Whitelist the public runtime inputs. No repo caches, credentials or key.
  src = lib.fileset.toSource {
    root = source;
    fileset = lib.fileset.unions [
      (source + "/manifest.json") (source + "/options.html") (source + "/options.css")
      (source + "/src/options.js") (source + "/src/background.js") (source + "/src/page.js")
    ];
  };
  nativeBuildInputs = [ python3 ];
  dontBuild = true;
  doCheck = true;
  checkPhase = ''
    runHook preCheck
    python3 ${./check-extension.py} . ${extensionId}
    runHook postCheck
  '';
  installPhase = ''
    runHook preInstall
    mkdir -p "$out/${extensionSubdir}" "$out/share/nexo-capture"
    cp manifest.json options.html options.css "$out/${extensionSubdir}/"
    cp -r src "$out/${extensionSubdir}/"
    runHook postInstall
  '';
  passthru = { inherit extensionId extensionSubdir mkNativeHost wrapBrave; };
  meta = {
    description = "Brave extension for local Nexo savings capture";
    platforms = lib.platforms.unix;
    license = lib.licenses.unfree;
  };
};
in package
