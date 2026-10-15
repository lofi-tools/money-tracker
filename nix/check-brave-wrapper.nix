{ stdenvNoCC, runCommand, python3, extension }:
let
  fakeBrave = stdenvNoCC.mkDerivation {
    pname = "brave-argument-fixture";
    version = "1";
    dontUnpack = true;
    dontBuild = true;
    installPhase = ''
      runHook preInstall
      mkdir -p "$out/Applications/Brave Browser.app/Contents/MacOS" "$out/bin"
      cat > "$out/Applications/Brave Browser.app/Contents/MacOS/Brave Browser" <<'PY'
      #!${python3}/bin/python3
      import json, os, sys
      from pathlib import Path
      Path(os.environ['NEXO_ARG_CAPTURE']).write_text(json.dumps({
          'args': sys.argv[1:], 'cwd': os.getcwd(),
          'marker': os.environ['NEXO_ENV_MARKER'], 'pid': os.getpid(),
      }))
      sys.exit(37 if '--exit=37' in sys.argv else 0)
      PY
      chmod +x "$out/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"
      ln -s "$out/Applications/Brave Browser.app/Contents/MacOS/Brave Browser" "$out/bin/brave"
      runHook postInstall
    '';
  };
  wrapped = extension.wrapBrave fakeBrave;
  executable = if stdenvNoCC.hostPlatform.isDarwin then
    "${wrapped}/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"
    else "${wrapped}/bin/brave";
in runCommand "nexo-brave-argument-check" { nativeBuildInputs = [ python3 ]; } ''
  python3 ${../tests/nix/check-arguments.py} "${executable}" \
    "--load-extension=${extension}/${extension.extensionSubdir}"
  touch "$out"
''
