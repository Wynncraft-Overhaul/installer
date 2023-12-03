mkdir macos_bundle/
mkdir macos_bundle/Installer.app/
mkdir macos_bundle/Installer.app/Contents/
mkdir macos_bundle/Installer.app/Contents/Resources
mkdir macos_bundle/Installer.app/Contents/MacOS/
cp icon_256.icns macos_bundle/Installer.app/Contents/Resources/icon_256.icns
cp target/debug/installer macos_bundle/Installer.app/Contents/MacOS/installer
cat > macos_bundle/Installer.app/Contents/Info.plist << EOF
{
   CFBundleName = installer;
   CFBundleDisplayName = Majestic Overhaul Installer;
   CFBundleIdentifier = "io.github.wynncraft-overhaul";
   CFBundleVersion = "1.0.0";
   CFBundleShortVersionString = "1.0.0";
   CFBundleInfoDictionaryVersion = "6.0";
   CFBundlePackageType = APPL;
   CFBundleSignature = inst;
   CFBundleExecutable = installer;
   CFBundleIconFile = "icon_256.icns";
}
EOF
